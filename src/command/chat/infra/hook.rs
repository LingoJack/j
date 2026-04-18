use super::super::permission::JcliConfig;
use super::super::storage::ChatMessage;
use crate::util::log::{write_error_log, write_info_log};
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::process::Command;
use std::sync::{Arc, Mutex};

// ========== 常量 ==========

/// Hook 链总超时（秒）：整条链执行超过此时间后，中止未执行的 hook
const MAX_CHAIN_DURATION_SECS: u64 = 30;

// ========== 数据结构 ==========

/// Hook 事件类型
///
/// 各事件的触发时机及可读/可写字段：
///
/// | 事件                  | 触发时机           | 可读字段                              | 可写字段（HookResult 中返回即生效）        |
/// |-----------------------|--------------------|-----------------------------------------|----------------------------------------------|
/// | `PreSendMessage`      | 用户消息入队前     | `user_input`, `messages`               | `user_input`（修改发送内容）, `abort`        |
/// | `PostSendMessage`     | 用户消息入队后     | `user_input`, `messages`               | 仅通知，返回值被忽略                         |
/// | `PreLlmRequest`       | LLM API 请求前     | `messages`, `system_prompt`, `model`   | `messages`, `system_prompt`, `inject_messages`, `abort` |
/// | `PostLlmResponse`     | LLM 回复完成后     | `assistant_output`, `messages`         | `assistant_output`（修改最终回复）           |
/// | `PreToolExecution`    | 工具执行前         | `tool_name`, `tool_arguments`          | `tool_arguments`（修改参数）, `abort`        |
/// | `PostToolExecution`   | 工具执行后         | `tool_name`, `tool_result`             | `tool_result`（修改结果）                    |
/// | `SessionStart`        | 会话启动时         | `messages`                             | 仅通知，返回值被忽略                         |
/// | `SessionEnd`          | 会话退出时         | `messages`                             | 仅通知，返回值被忽略                         |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookEvent {
    PreSendMessage,
    PostSendMessage,
    PreLlmRequest,
    PostLlmResponse,
    PreToolExecution,
    PostToolExecution,
    SessionStart,
    SessionEnd,
}

impl std::str::FromStr for HookEvent {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pre_send_message" => Ok(HookEvent::PreSendMessage),
            "post_send_message" => Ok(HookEvent::PostSendMessage),
            "pre_llm_request" => Ok(HookEvent::PreLlmRequest),
            "post_llm_response" => Ok(HookEvent::PostLlmResponse),
            "pre_tool_execution" => Ok(HookEvent::PreToolExecution),
            "post_tool_execution" => Ok(HookEvent::PostToolExecution),
            "session_start" => Ok(HookEvent::SessionStart),
            "session_end" => Ok(HookEvent::SessionEnd),
            _ => Err(()),
        }
    }
}

impl HookEvent {
    pub fn as_str(&self) -> &'static str {
        match self {
            HookEvent::PreSendMessage => "pre_send_message",
            HookEvent::PostSendMessage => "post_send_message",
            HookEvent::PreLlmRequest => "pre_llm_request",
            HookEvent::PostLlmResponse => "post_llm_response",
            HookEvent::PreToolExecution => "pre_tool_execution",
            HookEvent::PostToolExecution => "post_tool_execution",
            HookEvent::SessionStart => "session_start",
            HookEvent::SessionEnd => "session_end",
        }
    }

    pub fn all() -> &'static [HookEvent] {
        &[
            HookEvent::PreSendMessage,
            HookEvent::PostSendMessage,
            HookEvent::PreLlmRequest,
            HookEvent::PostLlmResponse,
            HookEvent::PreToolExecution,
            HookEvent::PostToolExecution,
            HookEvent::SessionStart,
            HookEvent::SessionEnd,
        ]
    }

    /// 从字符串解析，不匹配时返回 None
    pub fn parse(s: &str) -> Option<HookEvent> {
        s.parse().ok()
    }
}

/// Shell hook 失败时的处理策略
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OnError {
    /// 记录错误日志后继续执行后续 hook（默认）
    #[default]
    Skip,
    /// 立即中止整条 hook 链
    Abort,
}

/// Hook 条件过滤：仅当条件匹配时才执行该 hook
///
/// 所有字段为可选，未设置的字段不参与过滤（即视为匹配）。
/// 多个字段同时设置时取 AND 关系。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HookFilter {
    /// 工具名过滤（精确匹配，仅对 PreToolExecution / PostToolExecution 生效）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    /// 模型名前缀过滤（如 "gpt-4" 匹配 "gpt-4o"、"gpt-4-turbo"）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_prefix: Option<String>,
}

impl HookFilter {
    /// 是否为空过滤器（无任何条件，始终匹配）
    pub fn is_empty(&self) -> bool {
        self.tool_name.is_none() && self.model_prefix.is_none()
    }

    /// 根据 HookContext 判断是否匹配
    pub fn matches(&self, context: &HookContext) -> bool {
        if let Some(ref expected_tool) = self.tool_name {
            match &context.tool_name {
                Some(actual) if actual == expected_tool => {}
                Some(_) => return false,
                None => return false, // 事件中没有 tool_name，条件不满足
            }
        }
        if let Some(ref prefix) = self.model_prefix {
            match &context.model {
                Some(actual) if actual.starts_with(prefix.as_str()) => {}
                Some(_) => return false,
                None => return false,
            }
        }
        true
    }
}

/// Hook 定义（YAML 兼容）：一条 shell 命令 + 超时秒数 + 失败策略
/// 仅用于从 YAML 文件反序列化，内部使用 HookKind
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookDef {
    pub command: String,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    /// 脚本失败时的处理策略（默认 skip）
    #[serde(default)]
    pub on_error: OnError,
    /// 条件过滤：仅当条件匹配时执行（默认无过滤）
    #[serde(default, skip_serializing_if = "HookFilter::is_empty")]
    pub filter: HookFilter,
}

fn default_timeout() -> u64 {
    10
}

// ========== HookKind 枚举 ==========

/// Hook 种类：Shell 命令（子进程）或内置 Rust 闭包（进程内）
#[derive(Clone)]
pub enum HookKind {
    /// Shell 命令，通过 `sh -c` 子进程执行（现有行为）
    Shell(ShellHook),
    /// 内置 Rust 闭包，进程内零开销执行
    Builtin(BuiltinHook),
}

/// Shell hook：一条命令 + 超时 + 失败策略 + 条件过滤
#[derive(Debug, Clone)]
pub struct ShellHook {
    pub command: String,
    pub timeout: u64,
    pub on_error: OnError,
    pub filter: HookFilter,
}

impl From<HookDef> for ShellHook {
    fn from(def: HookDef) -> Self {
        ShellHook {
            command: def.command,
            timeout: def.timeout,
            on_error: def.on_error,
            filter: def.filter,
        }
    }
}

impl From<HookDef> for HookKind {
    fn from(def: HookDef) -> Self {
        HookKind::Shell(ShellHook::from(def))
    }
}

/// 内置 hook 的处理函数类型
pub type BuiltinHookFn = Arc<dyn Fn(&HookContext) -> Option<HookResult> + Send + Sync>;

/// 内置 hook：一个命名的 Rust 闭包
pub struct BuiltinHook {
    /// 唯一名称，用于列出/调试（如 "tasks_status"、"todo_nag"）
    pub name: String,
    /// 实际执行的 Rust 闭包
    pub handler: BuiltinHookFn,
}

impl Clone for BuiltinHook {
    fn clone(&self) -> Self {
        BuiltinHook {
            name: self.name.clone(),
            handler: Arc::clone(&self.handler),
        }
    }
}

impl std::fmt::Debug for HookKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HookKind::Shell(shell) => f
                .debug_struct("HookKind::Shell")
                .field("command", &shell.command)
                .field("timeout", &shell.timeout)
                .field("on_error", &shell.on_error)
                .finish(),
            HookKind::Builtin(builtin) => f
                .debug_struct("HookKind::Builtin")
                .field("name", &builtin.name)
                .finish(),
        }
    }
}

// ========== HookContext & HookResult ==========

/// Hook 执行上下文（通过 stdin JSON 传给脚本）
///
/// 各字段按事件类型有选择性地填充，未填充的字段序列化时会被跳过（`skip_serializing_if`）。
/// 脚本可通过 stdin 读取此 JSON 来获取当前事件的上下文信息。
#[derive(Debug, Serialize)]
pub struct HookContext {
    /// 当前触发的事件类型
    pub event: HookEvent,

    /// 当前对话的完整消息列表
    /// - 可读事件：PreSendMessage, PostSendMessage, PreLlmRequest, PostLlmResponse, SessionStart, SessionEnd
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages: Option<Vec<ChatMessage>>,

    /// 当前系统提示词
    /// - 可读事件：PreLlmRequest
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,

    /// 当前使用的模型名称
    /// - 可读事件：PreLlmRequest
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// 本轮用户输入的消息文本
    /// - 可读事件：PreSendMessage（发送前，可通过 HookResult 修改）、PostSendMessage（发送后，只读）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_input: Option<String>,

    /// 本轮 AI 回复的完整文本
    /// - 可读事件：PostLlmResponse（可通过 HookResult 修改最终展示内容）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assistant_output: Option<String>,

    /// 当前工具调用的工具名
    /// - 可读事件：PreToolExecution, PostToolExecution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,

    /// 当前工具调用的参数 JSON 字符串
    /// - 可读事件：PreToolExecution（可通过 HookResult 修改）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_arguments: Option<String>,

    /// 工具执行的结果内容
    /// - 可读事件：PostToolExecution（可通过 HookResult 修改）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_result: Option<String>,

    /// 当前会话 ID
    /// - 可读事件：所有事件
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,

    /// 当前工作目录
    pub cwd: String,
}

impl Default for HookContext {
    fn default() -> Self {
        Self {
            event: HookEvent::SessionStart,
            messages: None,
            system_prompt: None,
            model: None,
            user_input: None,
            assistant_output: None,
            tool_name: None,
            tool_arguments: None,
            tool_result: None,
            session_id: None,
            cwd: std::env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| ".".to_string()),
        }
    }
}

/// Hook 脚本返回结果（脚本通过 stdout 输出 JSON）
///
/// 脚本只需返回想要修改的字段，未返回的字段保持原值。
/// 空字符串或 `{}` 表示无修改。
///
/// 各字段的生效场景：
/// - `user_input`：仅 PreSendMessage 中生效，替换用户即将发送的消息
/// - `assistant_output`：仅 PostLlmResponse 中生效，替换 AI 最终展示的回复
/// - `messages`：仅 PreLlmRequest 中生效，替换发给 LLM 的消息列表
/// - `system_prompt`：仅 PreLlmRequest 中生效，替换系统提示词
/// - `tool_arguments`：仅 PreToolExecution 中生效，替换工具调用参数
/// - `tool_result`：仅 PostToolExecution 中生效，替换工具返回结果
/// - `inject_messages`：仅 PreLlmRequest 中生效，追加到消息列表末尾
/// - `abort`：Pre* 事件中生效，为 true 时中止当前操作
#[derive(Debug, Deserialize, Default)]
pub struct HookResult {
    /// 替换消息列表（PreLlmRequest）
    #[serde(default)]
    pub messages: Option<Vec<ChatMessage>>,
    /// 替换系统提示词（PreLlmRequest）
    #[serde(default)]
    pub system_prompt: Option<String>,
    /// 替换用户输入文本（PreSendMessage）
    #[serde(default)]
    pub user_input: Option<String>,
    /// 替换 AI 回复文本（PostLlmResponse）
    #[serde(default)]
    pub assistant_output: Option<String>,
    /// 替换工具调用参数（PreToolExecution）
    #[serde(default)]
    pub tool_arguments: Option<String>,
    /// 替换工具执行结果（PostToolExecution）
    #[serde(default)]
    pub tool_result: Option<String>,
    /// 追加消息到消息列表末尾（PreLlmRequest）
    #[serde(default)]
    pub inject_messages: Option<Vec<ChatMessage>>,
    /// 中止当前操作（Pre* 事件中有效）
    #[serde(default)]
    pub abort: bool,
}

// ========== HookManager ==========

/// 单个 hook 的执行统计
#[derive(Debug, Clone, Default)]
pub struct HookMetrics {
    /// 执行次数
    pub executions: u64,
    /// 成功次数
    pub successes: u64,
    /// 失败次数（含超时）
    pub failures: u64,
    /// 跳过次数（filter 不匹配）
    pub skipped: u64,
    /// 累计耗时（毫秒）
    pub total_duration_ms: u64,
}

/// Hook 管理器：管理四级 hook（内置、用户级、项目级、session 级）
///
/// 执行顺序：内置 → 用户级 → 项目级 → Session 级，链式执行。
/// 前者的输出会更新到 context 中，影响后者的输入。任何 `abort` 立即中止整条链。
#[derive(Debug, Default)]
pub struct HookManager {
    builtin_hooks: HashMap<HookEvent, Vec<HookKind>>,
    user_hooks: HashMap<HookEvent, Vec<HookKind>>,
    project_hooks: HashMap<HookEvent, Vec<HookKind>>,
    session_hooks: HashMap<HookEvent, Vec<HookKind>>,
    /// 按 hook label 记录的执行指标（内部可变，execute 不需要 &mut self）
    metrics: Mutex<HashMap<String, HookMetrics>>,
}

impl Clone for HookManager {
    fn clone(&self) -> Self {
        HookManager {
            builtin_hooks: self.builtin_hooks.clone(),
            user_hooks: self.user_hooks.clone(),
            project_hooks: self.project_hooks.clone(),
            session_hooks: self.session_hooks.clone(),
            metrics: Mutex::new(self.metrics.lock().map(|m| m.clone()).unwrap_or_default()),
        }
    }
}

/// 列出 hook 时的来源标记
const HOOK_SOURCE_BUILTIN: &str = "builtin";
const HOOK_SOURCE_USER: &str = "user";
const HOOK_SOURCE_PROJECT: &str = "project";
const HOOK_SOURCE_SESSION: &str = "session";

/// 列出 hook 时的摘要信息
pub struct HookEntry {
    pub event: HookEvent,
    pub source: &'static str,
    /// Shell hook 的命令，或 Builtin hook 的名称
    pub label: String,
    /// Shell hook 的超时秒数
    pub timeout: Option<u64>,
    /// Shell hook 的失败策略
    pub on_error: Option<OnError>,
    /// Session hook 在该 event 下的局部索引（仅 session 来源有值，用于 remove 操作）
    pub session_index: Option<usize>,
    /// 条件过滤（仅 Shell hook 有）
    pub filter: Option<HookFilter>,
    /// 执行指标
    pub metrics: Option<HookMetrics>,
}

impl HookManager {
    /// 加载用户级（`~/.jdata/agent/hooks.yaml`）+ 项目级（`.jcli/hooks.yaml`）hook
    pub fn load() -> Self {
        let mut manager = HookManager::default();

        // 加载用户级 hooks：~/.jdata/agent/hooks.yaml
        let user_hooks_path = super::super::storage::hooks_config_path();
        if user_hooks_path.is_file() {
            match std::fs::read_to_string(&user_hooks_path) {
                Ok(content) => {
                    match serde_yaml::from_str::<HashMap<String, Vec<HookDef>>>(&content) {
                        Ok(hooks_map) => {
                            for (event_name, defs) in hooks_map {
                                if let Some(event) = HookEvent::parse(&event_name) {
                                    manager
                                        .user_hooks
                                        .entry(event)
                                        .or_default()
                                        .extend(defs.into_iter().map(HookKind::from));
                                } else {
                                    write_error_log(
                                        "HookManager::load",
                                        &format!("未知 hook 事件: {}", event_name),
                                    );
                                }
                            }
                            write_info_log(
                                "HookManager::load",
                                &format!("已加载用户级 hooks: {}", user_hooks_path.display()),
                            );
                        }
                        Err(e) => {
                            write_error_log(
                                "HookManager::load",
                                &format!("解析用户级 hooks.yaml 失败: {}", e),
                            );
                        }
                    }
                }
                Err(e) => {
                    write_error_log("HookManager::load", &format!("读取 hooks.yaml 失败: {}", e));
                }
            }
        }

        // 加载项目级 hooks：从 .jcli/hooks.yaml
        if let Some(config_dir) = JcliConfig::find_config_dir() {
            let hooks_path = config_dir.join("hooks.yaml");
            if hooks_path.is_file() {
                match std::fs::read_to_string(&hooks_path) {
                    Ok(content) => {
                        match serde_yaml::from_str::<HashMap<String, Vec<HookDef>>>(&content) {
                            Ok(hooks_map) => {
                                for (event_name, defs) in hooks_map {
                                    if let Some(event) = HookEvent::parse(&event_name) {
                                        manager
                                            .project_hooks
                                            .entry(event)
                                            .or_default()
                                            .extend(defs.into_iter().map(HookKind::from));
                                    } else {
                                        write_error_log(
                                            "HookManager::load",
                                            &format!(
                                                "项目级 .jcli/hooks.yaml 中未知 hook 事件: {}",
                                                event_name
                                            ),
                                        );
                                    }
                                }
                                write_info_log(
                                    "HookManager::load",
                                    &format!("已加载项目级 hooks: {}", hooks_path.display()),
                                );
                            }
                            Err(e) => {
                                write_error_log(
                                    "HookManager::load",
                                    &format!("解析项目级 hooks.yaml 失败: {}", e),
                                );
                            }
                        }
                    }
                    Err(e) => {
                        write_error_log(
                            "HookManager::load",
                            &format!("读取项目级 hooks.yaml 失败: {}", e),
                        );
                    }
                }
            }
        }

        manager
    }

    /// 注册内置 hook（程序初始化时调用）
    ///
    /// 内置 hook 以 Rust 闭包形式在进程内执行，零开销。
    /// 执行优先级最高（在内置级中按注册顺序执行，先于用户/项目/session 级）。
    pub fn register_builtin(
        &mut self,
        event: HookEvent,
        name: impl Into<String>,
        handler: impl Fn(&HookContext) -> Option<HookResult> + Send + Sync + 'static,
    ) {
        self.builtin_hooks
            .entry(event)
            .or_default()
            .push(HookKind::Builtin(BuiltinHook {
                name: name.into(),
                handler: Arc::new(handler),
            }));
    }

    /// 注册 session 级 hook（由 register_hook 工具调用）
    pub fn register_session_hook(&mut self, event: HookEvent, def: HookDef) {
        self.session_hooks
            .entry(event)
            .or_default()
            .push(HookKind::Shell(ShellHook::from(def)));
    }

    /// 移除 session 级 hook（按事件和索引）
    pub fn remove_session_hook(&mut self, event: HookEvent, index: usize) -> bool {
        if let Some(hooks) = self.session_hooks.get_mut(&event)
            && index < hooks.len()
        {
            hooks.remove(index);
            return true;
        }
        false
    }

    /// 列出所有 hook（含来源标记和摘要信息）
    pub fn list_hooks(&self) -> Vec<HookEntry> {
        let mut result = Vec::new();
        let metrics_map = self.metrics.lock().ok();
        let empty_metrics = HashMap::new();
        let metrics_ref = metrics_map.as_deref().unwrap_or(&empty_metrics);
        let make_entry = |event: HookEvent,
                          source: &'static str,
                          hook: &HookKind,
                          session_index: Option<usize>,
                          metrics: &HashMap<String, HookMetrics>| {
            let label = hook_label(hook);
            HookEntry {
                event,
                source,
                timeout: hook_timeout(hook),
                on_error: hook_on_error(hook),
                filter: hook_filter(hook).cloned(),
                metrics: metrics.get(&label).cloned(),
                session_index,
                label,
            }
        };
        for event in HookEvent::all() {
            if let Some(hooks) = self.builtin_hooks.get(event) {
                for hook in hooks {
                    result.push(make_entry(
                        *event,
                        HOOK_SOURCE_BUILTIN,
                        hook,
                        None,
                        metrics_ref,
                    ));
                }
            }
            if let Some(hooks) = self.user_hooks.get(event) {
                for hook in hooks {
                    result.push(make_entry(
                        *event,
                        HOOK_SOURCE_USER,
                        hook,
                        None,
                        metrics_ref,
                    ));
                }
            }
            if let Some(hooks) = self.project_hooks.get(event) {
                for hook in hooks {
                    result.push(make_entry(
                        *event,
                        HOOK_SOURCE_PROJECT,
                        hook,
                        None,
                        metrics_ref,
                    ));
                }
            }
            if let Some(hooks) = self.session_hooks.get(event) {
                for (idx, hook) in hooks.iter().enumerate() {
                    result.push(make_entry(
                        *event,
                        HOOK_SOURCE_SESSION,
                        hook,
                        Some(idx),
                        metrics_ref,
                    ));
                }
            }
        }
        result
    }

    /// 热重载用户级和项目级 hook 配置
    ///
    /// 重新读取 `~/.jdata/agent/hooks.yaml` 和 `.jcli/hooks.yaml`，
    /// 替换当前的 user_hooks 和 project_hooks（builtin 和 session 级不受影响）。
    /// 指标数据保留不清零。
    #[allow(dead_code)]
    pub fn reload(&mut self) {
        let fresh = HookManager::load();
        self.user_hooks = fresh.user_hooks;
        self.project_hooks = fresh.project_hooks;
        write_info_log("HookManager::reload", "已重新加载用户级和项目级 hooks");
    }

    /// 获取所有 hook 的执行指标快照（按 label）
    #[allow(dead_code)]
    pub fn get_metrics(&self) -> HashMap<String, HookMetrics> {
        self.metrics.lock().map(|m| m.clone()).unwrap_or_default()
    }

    /// 检查某个事件是否有任何 hook 注册（内置/用户级/项目级/session 级）
    /// 用于调用方在构建 HookContext 之前短路，避免不必要的 clone 和内存分配
    pub fn has_hooks_for(&self, event: HookEvent) -> bool {
        self.builtin_hooks
            .get(&event)
            .is_some_and(|h| !h.is_empty())
            || self.user_hooks.get(&event).is_some_and(|h| !h.is_empty())
            || self
                .project_hooks
                .get(&event)
                .is_some_and(|h| !h.is_empty())
            || self
                .session_hooks
                .get(&event)
                .is_some_and(|h| !h.is_empty())
    }

    /// Fire-and-forget 执行：在后台线程中执行 hook，不阻塞调用方
    /// 适用于 PostSendMessage、SessionEnd 等不需要返回值的 hook
    pub fn execute_fire_and_forget(
        manager: Arc<Mutex<HookManager>>,
        event: HookEvent,
        context: HookContext,
    ) {
        std::thread::spawn(move || {
            if let Ok(m) = manager.lock() {
                let _ = m.execute(event, context);
            }
        });
    }

    /// 链式执行所有 hook（内置→用户→项目→session）
    ///
    /// 返回 `Some(HookResult)` 如果有任何修改或 abort，否则 `None`。
    /// 链式执行中，前一个 hook 的输出会更新到 context 中，成为下一个 hook 的输入。
    ///
    /// **注意**：调用方应先用 `has_hooks_for()` 检查，再构建 HookContext 并调用此方法，
    /// 避免在没有 hook 注册时进行不必要的内存分配。
    pub fn execute(&self, event: HookEvent, mut context: HookContext) -> Option<HookResult> {
        let mut all_hooks: Vec<&HookKind> = Vec::new();

        // 执行顺序：内置 → 用户 → 项目 → session
        if let Some(hooks) = self.builtin_hooks.get(&event) {
            all_hooks.extend(hooks.iter());
        }
        if let Some(hooks) = self.user_hooks.get(&event) {
            all_hooks.extend(hooks.iter());
        }
        if let Some(hooks) = self.project_hooks.get(&event) {
            all_hooks.extend(hooks.iter());
        }
        if let Some(hooks) = self.session_hooks.get(&event) {
            all_hooks.extend(hooks.iter());
        }

        if all_hooks.is_empty() {
            return None;
        }

        write_info_log(
            "HookManager::execute",
            &format!(
                "执行 {} 个 hook (事件: {})",
                all_hooks.len(),
                event.as_str()
            ),
        );

        let mut had_modification = false;
        let mut final_result = HookResult::default();
        let chain_start = std::time::Instant::now();
        let chain_timeout = std::time::Duration::from_secs(MAX_CHAIN_DURATION_SECS);

        for hook in all_hooks {
            // 链总超时检查
            if chain_start.elapsed() > chain_timeout {
                write_error_log(
                    "HookManager::execute",
                    &format!(
                        "Hook 链总超时 ({}s)，中止剩余 hook (事件: {})",
                        MAX_CHAIN_DURATION_SECS,
                        event.as_str()
                    ),
                );
                break;
            }

            let label = hook_label(hook);

            // 条件过滤检查
            if !hook_should_execute(hook, &context) {
                if let Ok(mut metrics) = self.metrics.lock() {
                    let m = metrics.entry(label).or_default();
                    m.skipped += 1;
                }
                continue;
            }

            let hook_start = std::time::Instant::now();
            match execute_hook(hook, &context) {
                Ok(result) => {
                    let elapsed_ms = hook_start.elapsed().as_millis() as u64;
                    if let Ok(mut metrics) = self.metrics.lock() {
                        let m = metrics.entry(label.clone()).or_default();
                        m.executions += 1;
                        m.successes += 1;
                        m.total_duration_ms += elapsed_ms;
                    }

                    if result.abort {
                        write_info_log("HookManager::execute", &format!("Hook abort ({})", label));
                        return Some(HookResult {
                            abort: true,
                            ..Default::default()
                        });
                    }

                    // 合并结果到 context（链式传递）
                    if let Some(ref msgs) = result.messages {
                        context.messages = Some(msgs.clone());
                        final_result.messages = Some(msgs.clone());
                        had_modification = true;
                    }
                    if let Some(ref sp) = result.system_prompt {
                        context.system_prompt = Some(sp.clone());
                        final_result.system_prompt = Some(sp.clone());
                        had_modification = true;
                    }
                    if let Some(ref ui) = result.user_input {
                        context.user_input = Some(ui.clone());
                        final_result.user_input = Some(ui.clone());
                        had_modification = true;
                    }
                    if let Some(ref ao) = result.assistant_output {
                        context.assistant_output = Some(ao.clone());
                        final_result.assistant_output = Some(ao.clone());
                        had_modification = true;
                    }
                    if let Some(ref ta) = result.tool_arguments {
                        context.tool_arguments = Some(ta.clone());
                        final_result.tool_arguments = Some(ta.clone());
                        had_modification = true;
                    }
                    if let Some(ref tr) = result.tool_result {
                        context.tool_result = Some(tr.clone());
                        final_result.tool_result = Some(tr.clone());
                        had_modification = true;
                    }
                    if let Some(ref inject) = result.inject_messages {
                        let existing = final_result.inject_messages.get_or_insert_with(Vec::new);
                        existing.extend(inject.clone());
                        had_modification = true;
                    }
                }
                Err(e) => {
                    let elapsed_ms = hook_start.elapsed().as_millis() as u64;
                    if let Ok(mut metrics) = self.metrics.lock() {
                        let m = metrics.entry(label.clone()).or_default();
                        m.executions += 1;
                        m.failures += 1;
                        m.total_duration_ms += elapsed_ms;
                    }

                    // Shell hook 非零退出 / 超时 → 按 on_error 策略处理
                    // Builtin hook 失败一律 abort（内置 hook 失败是真正的错误）
                    write_error_log(
                        "HookManager::execute",
                        &format!("Hook 执行失败 ({}): {}", label, e),
                    );
                    match hook_on_error_strategy(hook) {
                        OnError::Abort => {
                            return Some(HookResult {
                                abort: true,
                                ..Default::default()
                            });
                        }
                        OnError::Skip => {
                            // 记录日志后继续执行后续 hook
                            continue;
                        }
                    }
                }
            }
        }

        if had_modification {
            Some(final_result)
        } else {
            None
        }
    }
}

// ========== Hook 执行分派 ==========

/// 执行单个 hook（分派到 Shell 或 Builtin）
fn execute_hook(kind: &HookKind, context: &HookContext) -> Result<HookResult, String> {
    match kind {
        HookKind::Shell(shell) => execute_shell_hook(shell, context),
        HookKind::Builtin(builtin) => match (builtin.handler)(context) {
            Some(result) => Ok(result),
            None => Ok(HookResult::default()),
        },
    }
}

/// 执行 Shell hook 脚本
///
/// 协议：
/// - 执行方式: `sh -c "<command>"`
/// - 工作目录: 用户当前目录 (`std::env::current_dir()`)
/// - 环境变量: `JCLI_HOOK_EVENT`（事件名）、`JCLI_CWD`（当前目录）
/// - stdin: HookContext JSON
/// - stdout: HookResult JSON（可为空字符串/空 JSON `{}`，表示无修改）
/// - exit 0: 成功
/// - exit ≠0: 视为失败（调用方按 on_error 策略处理）
/// - 超时: kill 子进程，返回 Err
fn execute_shell_hook(hook: &ShellHook, context: &HookContext) -> Result<HookResult, String> {
    let context_json =
        serde_json::to_string(context).map_err(|e| format!("序列化 context 失败: {}", e))?;

    let cwd = std::env::current_dir().map_err(|e| format!("获取 cwd 失败: {}", e))?;

    let mut child = Command::new("sh")
        .arg("-c")
        .arg(&hook.command)
        .current_dir(&cwd)
        .env("JCLI_HOOK_EVENT", context.event.as_str())
        .env("JCLI_CWD", cwd.display().to_string())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("启动 hook 进程失败: {}", e))?;

    // 保存 PID 用于超时 kill
    let pid = child.id();

    // 写入 stdin 后关闭（drop stdin handle）
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(context_json.as_bytes());
    }

    // 子线程中 wait_with_output（阻塞等待进程退出 + 一次性读取 stdout/stderr）
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(child.wait_with_output());
    });

    let timeout = std::time::Duration::from_secs(hook.timeout);
    match rx.recv_timeout(timeout) {
        Ok(Ok(output)) => {
            // 捕获 stderr 并记录日志
            let stderr_str = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if !stderr_str.is_empty() {
                write_info_log(
                    "execute_shell_hook",
                    &format!("Hook stderr ({}): {}", hook.command, stderr_str),
                );
            }

            if !output.status.success() {
                let mut err = format!("Hook 退出码: {:?}", output.status.code());
                if !stderr_str.is_empty() {
                    err.push_str(&format!(", stderr: {}", stderr_str));
                }
                return Err(err);
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            let stdout = stdout.trim();

            if stdout.is_empty() || stdout == "{}" {
                return Ok(HookResult::default());
            }

            let result: HookResult = serde_json::from_str(stdout)
                .map_err(|e| format!("解析 hook 输出 JSON 失败: {} (输出: {})", e, stdout))?;

            write_info_log(
                "execute_shell_hook",
                &format!("Hook 完成 (cmd: {}), abort={}", hook.command, result.abort),
            );

            Ok(result)
        }
        Ok(Err(e)) => Err(format!("等待 hook 进程失败: {}", e)),
        Err(_) => {
            // 超时：通过 PID 发送 SIGKILL 终止进程
            let _ = signal::kill(Pid::from_raw(pid as i32), Signal::SIGKILL);
            Err(format!("Hook 超时 ({}s): {}", hook.timeout, hook.command))
        }
    }
}

// ========== 辅助函数 ==========

/// 获取 hook 的显示标签（Shell 用命令，Builtin 用名称）
fn hook_label(kind: &HookKind) -> String {
    match kind {
        HookKind::Shell(shell) => shell.command.clone(),
        HookKind::Builtin(builtin) => format!("[builtin: {}]", builtin.name),
    }
}

/// 获取 hook 的超时秒数（仅 Shell hook 有）
fn hook_timeout(kind: &HookKind) -> Option<u64> {
    match kind {
        HookKind::Shell(shell) => Some(shell.timeout),
        HookKind::Builtin(_) => None,
    }
}

/// 获取 hook 的失败策略（用于 list 展示，Shell 返回配置值，Builtin 为 None）
fn hook_on_error(kind: &HookKind) -> Option<OnError> {
    match kind {
        HookKind::Shell(shell) => Some(shell.on_error),
        HookKind::Builtin(_) => None,
    }
}

/// 获取 hook 执行失败时的实际策略（Shell 按配置，Builtin 一律 Abort）
fn hook_on_error_strategy(kind: &HookKind) -> OnError {
    match kind {
        HookKind::Shell(shell) => shell.on_error,
        HookKind::Builtin(_) => OnError::Abort,
    }
}

/// 获取 hook 的条件过滤器（仅 Shell hook 有）
fn hook_filter(kind: &HookKind) -> Option<&HookFilter> {
    match kind {
        HookKind::Shell(shell) if !shell.filter.is_empty() => Some(&shell.filter),
        _ => None,
    }
}

/// 检查 hook 是否应在当前 context 下执行（无 filter 或 filter 匹配时返回 true）
fn hook_should_execute(kind: &HookKind, context: &HookContext) -> bool {
    match kind {
        HookKind::Shell(shell) => shell.filter.matches(context),
        HookKind::Builtin(_) => true,
    }
}

// ========== 单元测试 ==========

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_event_roundtrip() {
        for event in HookEvent::all() {
            let s = event.as_str();
            let parsed = HookEvent::parse(s).unwrap();
            assert_eq!(*event, parsed);
        }
    }

    #[test]
    fn test_hook_event_from_str_invalid() {
        assert!(HookEvent::parse("unknown_event").is_none());
    }

    #[test]
    fn test_hook_def_default_timeout() {
        let yaml = r#"command: "echo hello""#;
        let def: HookDef = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(def.timeout, 10);
    }

    #[test]
    fn test_hook_def_to_hook_kind() {
        let def = HookDef {
            command: "echo test".to_string(),
            timeout: 5,
            on_error: OnError::Skip,
            filter: HookFilter::default(),
        };
        let kind = HookKind::from(def);
        match kind {
            HookKind::Shell(shell) => {
                assert_eq!(shell.command, "echo test");
                assert_eq!(shell.timeout, 5);
            }
            HookKind::Builtin(_) => panic!("应该转换为 Shell 变体"),
        }
    }

    #[test]
    fn test_hook_result_empty_json() {
        let result: HookResult = serde_json::from_str("{}").unwrap();
        assert!(!result.abort);
        assert!(result.messages.is_none());
        assert!(result.user_input.is_none());
    }

    #[test]
    fn test_hook_result_with_abort() {
        let json = r#"{"abort": true}"#;
        let result: HookResult = serde_json::from_str(json).unwrap();
        assert!(result.abort);
    }

    #[test]
    fn test_hook_result_with_user_input() {
        let json = r#"{"user_input": "[modified] hello"}"#;
        let result: HookResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.user_input.as_deref(), Some("[modified] hello"));
    }

    #[test]
    fn test_hook_context_serialization() {
        let ctx = HookContext {
            event: HookEvent::PreSendMessage,
            user_input: Some("hello".to_string()),
            ..Default::default()
        };
        let json = serde_json::to_string(&ctx).unwrap();
        assert!(json.contains("pre_send_message"));
        assert!(json.contains("hello"));
        assert!(json.contains("user_input"));
        // skip_serializing_if 应跳过 None 字段
        assert!(!json.contains("messages"));
        assert!(!json.contains("tool_name"));
    }

    #[test]
    fn test_execute_shell_hook_echo() {
        let hook = ShellHook {
            command: r#"echo '{"user_input": "hooked"}'"#.to_string(),
            timeout: 5,
            on_error: OnError::Skip,
            filter: HookFilter::default(),
        };
        let ctx = HookContext {
            event: HookEvent::PreSendMessage,
            user_input: Some("original".to_string()),
            ..Default::default()
        };
        let result = execute_shell_hook(&hook, &ctx).unwrap();
        assert_eq!(result.user_input.as_deref(), Some("hooked"));
        assert!(!result.abort);
    }

    #[test]
    fn test_execute_shell_hook_empty_output() {
        let hook = ShellHook {
            command: "echo ''".to_string(),
            timeout: 5,
            on_error: OnError::Skip,
            filter: HookFilter::default(),
        };
        let ctx = HookContext::default();
        let result = execute_shell_hook(&hook, &ctx).unwrap();
        assert!(!result.abort);
        assert!(result.user_input.is_none());
    }

    #[test]
    fn test_execute_shell_hook_nonzero_exit() {
        let hook = ShellHook {
            command: "exit 1".to_string(),
            timeout: 5,
            on_error: OnError::Skip,
            filter: HookFilter::default(),
        };
        let ctx = HookContext::default();
        let result = execute_shell_hook(&hook, &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_shell_hook_reads_stdin() {
        let hook = ShellHook {
            command: r#"input=$(cat); event=$(echo "$input" | python3 -c "import sys,json; print(json.load(sys.stdin).get('event',''))" 2>/dev/null || echo ""); echo '{"user_input": "got_input"}'"#.to_string(),
            timeout: 5,
            on_error: OnError::Skip,
        filter: HookFilter::default(),
        };
        let ctx = HookContext {
            event: HookEvent::PreSendMessage,
            user_input: Some("test".to_string()),
            ..Default::default()
        };
        let result = execute_shell_hook(&hook, &ctx).unwrap();
        assert_eq!(result.user_input.as_deref(), Some("got_input"));
    }

    #[test]
    fn test_execute_builtin_hook() {
        let builtin = BuiltinHook {
            name: "test_hook".to_string(),
            handler: Arc::new(|ctx| {
                if let Some(ref input) = ctx.user_input {
                    Some(HookResult {
                        user_input: Some(format!("[hooked] {}", input)),
                        ..Default::default()
                    })
                } else {
                    None
                }
            }),
        };
        let kind = HookKind::Builtin(builtin);
        let ctx = HookContext {
            event: HookEvent::PreSendMessage,
            user_input: Some("original".to_string()),
            ..Default::default()
        };
        let result = execute_hook(&kind, &ctx).unwrap();
        assert_eq!(result.user_input.as_deref(), Some("[hooked] original"));
    }

    #[test]
    fn test_execute_builtin_hook_returns_none() {
        let builtin = BuiltinHook {
            name: "no_op".to_string(),
            handler: Arc::new(|_| None),
        };
        let kind = HookKind::Builtin(builtin);
        let ctx = HookContext::default();
        let result = execute_hook(&kind, &ctx).unwrap();
        assert!(!result.abort);
        assert!(result.user_input.is_none());
    }

    #[test]
    fn test_hook_manager_empty() {
        let manager = HookManager::default();
        assert!(manager.list_hooks().is_empty());
        let result = manager.execute(HookEvent::PreSendMessage, HookContext::default());
        assert!(result.is_none());
    }

    #[test]
    fn test_hook_manager_session_hooks() {
        let mut manager = HookManager::default();
        manager.register_session_hook(
            HookEvent::PreSendMessage,
            HookDef {
                command: r#"echo '{"user_input": "session_hooked"}'"#.to_string(),
                timeout: 5,
                on_error: OnError::Skip,
                filter: HookFilter::default(),
            },
        );

        let hooks = manager.list_hooks();
        assert_eq!(hooks.len(), 1);
        assert_eq!(hooks[0].source, "session");

        let result = manager
            .execute(
                HookEvent::PreSendMessage,
                HookContext {
                    event: HookEvent::PreSendMessage,
                    user_input: Some("original".to_string()),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(result.user_input.as_deref(), Some("session_hooked"));
    }

    #[test]
    fn test_hook_manager_builtin_hooks() {
        let mut manager = HookManager::default();
        manager.register_builtin(HookEvent::PreSendMessage, "test_builtin", |ctx| {
            if let Some(ref input) = ctx.user_input {
                Some(HookResult {
                    user_input: Some(format!("[builtin] {}", input)),
                    ..Default::default()
                })
            } else {
                None
            }
        });

        let hooks = manager.list_hooks();
        assert_eq!(hooks.len(), 1);
        assert_eq!(hooks[0].source, "builtin");
        assert!(hooks[0].label.contains("test_builtin"));

        let result = manager
            .execute(
                HookEvent::PreSendMessage,
                HookContext {
                    event: HookEvent::PreSendMessage,
                    user_input: Some("hello".to_string()),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(result.user_input.as_deref(), Some("[builtin] hello"));
    }

    #[test]
    fn test_hook_manager_builtin_before_session() {
        // 内置 hook 应在 session hook 之前执行，session hook 应能覆盖内置 hook 的结果
        let mut manager = HookManager::default();
        manager.register_builtin(HookEvent::PreSendMessage, "prefix", |ctx| {
            if let Some(ref input) = ctx.user_input {
                Some(HookResult {
                    user_input: Some(format!("[builtin] {}", input)),
                    ..Default::default()
                })
            } else {
                None
            }
        });
        manager.register_session_hook(
            HookEvent::PreSendMessage,
            HookDef {
                command: r#"echo '{"user_input": "session_overridden"}'"#.to_string(),
                timeout: 5,
                on_error: OnError::Skip,
                filter: HookFilter::default(),
            },
        );

        let result = manager
            .execute(
                HookEvent::PreSendMessage,
                HookContext {
                    event: HookEvent::PreSendMessage,
                    user_input: Some("original".to_string()),
                    ..Default::default()
                },
            )
            .unwrap();
        // session hook 在 builtin 之后执行，覆盖了 builtin 的结果
        assert_eq!(result.user_input.as_deref(), Some("session_overridden"));
    }

    #[test]
    fn test_hook_manager_remove_session_hook() {
        let mut manager = HookManager::default();
        manager.register_session_hook(
            HookEvent::PreSendMessage,
            HookDef {
                command: "echo test".to_string(),
                timeout: 5,
                on_error: OnError::Skip,
                filter: HookFilter::default(),
            },
        );
        assert_eq!(manager.list_hooks().len(), 1);

        assert!(manager.remove_session_hook(HookEvent::PreSendMessage, 0));
        assert!(manager.list_hooks().is_empty());

        // 移除不存在的索引
        assert!(!manager.remove_session_hook(HookEvent::PreSendMessage, 0));
    }

    #[test]
    fn test_hook_chain_execution() {
        let mut manager = HookManager::default();

        manager.register_session_hook(
            HookEvent::PreSendMessage,
            HookDef {
                command: r#"echo '{"user_input": "first"}'"#.to_string(),
                timeout: 5,
                on_error: OnError::Skip,
                filter: HookFilter::default(),
            },
        );
        manager.register_session_hook(
            HookEvent::PreSendMessage,
            HookDef {
                command: r#"echo '{"user_input": "second"}'"#.to_string(),
                timeout: 5,
                on_error: OnError::Skip,
                filter: HookFilter::default(),
            },
        );

        let result = manager
            .execute(
                HookEvent::PreSendMessage,
                HookContext {
                    event: HookEvent::PreSendMessage,
                    user_input: Some("original".to_string()),
                    ..Default::default()
                },
            )
            .unwrap();

        // 最后一个 hook 的输出应该覆盖之前的
        assert_eq!(result.user_input.as_deref(), Some("second"));
    }

    #[test]
    fn test_hook_abort_stops_chain() {
        let mut manager = HookManager::default();

        manager.register_session_hook(
            HookEvent::PreSendMessage,
            HookDef {
                command: "exit 1".to_string(), // 非零退出 + on_error=abort → 中止链
                timeout: 5,
                on_error: OnError::Abort,
                filter: HookFilter::default(),
            },
        );
        manager.register_session_hook(
            HookEvent::PreSendMessage,
            HookDef {
                command: r#"echo '{"user_input": "should_not_reach"}'"#.to_string(),
                timeout: 5,
                on_error: OnError::Skip,
                filter: HookFilter::default(),
            },
        );

        let result = manager
            .execute(
                HookEvent::PreSendMessage,
                HookContext {
                    event: HookEvent::PreSendMessage,
                    ..Default::default()
                },
            )
            .unwrap();

        assert!(result.abort);
        assert!(result.user_input.is_none());
    }

    #[test]
    fn test_builtin_hook_clone() {
        let mut manager = HookManager::default();
        manager.register_builtin(HookEvent::PreLlmRequest, "test_clone", |_| {
            Some(HookResult::default())
        });
        // HookManager 的 Clone 依赖 BuiltinHook 的 Clone（通过 Arc）
        let cloned = manager.clone();
        assert_eq!(cloned.list_hooks().len(), 1);
    }

    #[test]
    fn test_on_error_skip_continues_chain() {
        // on_error=skip 时，第一个 hook 失败不影响后续 hook 执行
        let mut manager = HookManager::default();

        manager.register_session_hook(
            HookEvent::PreSendMessage,
            HookDef {
                command: "exit 1".to_string(), // 失败
                timeout: 5,
                on_error: OnError::Skip, // 但 skip → 继续
                filter: HookFilter::default(),
            },
        );
        manager.register_session_hook(
            HookEvent::PreSendMessage,
            HookDef {
                command: r#"echo '{"user_input": "survived"}'"#.to_string(),
                timeout: 5,
                on_error: OnError::Skip,
                filter: HookFilter::default(),
            },
        );

        let result = manager
            .execute(
                HookEvent::PreSendMessage,
                HookContext {
                    event: HookEvent::PreSendMessage,
                    user_input: Some("original".to_string()),
                    ..Default::default()
                },
            )
            .unwrap();

        // 第二个 hook 应正常执行
        assert!(!result.abort);
        assert_eq!(result.user_input.as_deref(), Some("survived"));
    }

    #[test]
    fn test_on_error_abort_stops_chain() {
        // on_error=abort 时，失败的 hook 中止整条链
        let mut manager = HookManager::default();

        manager.register_session_hook(
            HookEvent::PreSendMessage,
            HookDef {
                command: "exit 1".to_string(),
                timeout: 5,
                on_error: OnError::Abort,
                filter: HookFilter::default(),
            },
        );
        manager.register_session_hook(
            HookEvent::PreSendMessage,
            HookDef {
                command: r#"echo '{"user_input": "should_not_reach"}'"#.to_string(),
                timeout: 5,
                on_error: OnError::Skip,
                filter: HookFilter::default(),
            },
        );

        let result = manager
            .execute(
                HookEvent::PreSendMessage,
                HookContext {
                    event: HookEvent::PreSendMessage,
                    ..Default::default()
                },
            )
            .unwrap();

        assert!(result.abort);
        assert!(result.user_input.is_none());
    }

    #[test]
    fn test_on_error_default_is_skip() {
        // HookDef 不设 on_error 时，YAML 反序列化应默认为 skip
        let yaml = r#"command: "exit 1"
timeout: 5"#;
        let def: HookDef = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(def.on_error, OnError::Skip);
    }

    #[test]
    fn test_on_error_yaml_parsing() {
        // 验证 on_error 字段能正确从 YAML 反序列化
        let yaml_skip = r#"command: "echo test"
on_error: skip"#;
        let def: HookDef = serde_yaml::from_str(yaml_skip).unwrap();
        assert_eq!(def.on_error, OnError::Skip);

        let yaml_abort = r#"command: "echo test"
on_error: abort"#;
        let def: HookDef = serde_yaml::from_str(yaml_abort).unwrap();
        assert_eq!(def.on_error, OnError::Abort);
    }

    #[test]
    fn test_shell_hook_stderr_captured() {
        // 验证 stderr 输出不会导致死锁，且进程正常完成
        let hook = ShellHook {
            command: r#"echo '{"user_input": "ok"}'; echo "debug info" >&2"#.to_string(),
            timeout: 5,
            on_error: OnError::Skip,
            filter: HookFilter::default(),
        };
        let ctx = HookContext {
            event: HookEvent::PreSendMessage,
            user_input: Some("test".to_string()),
            ..Default::default()
        };
        let result = execute_shell_hook(&hook, &ctx).unwrap();
        assert_eq!(result.user_input.as_deref(), Some("ok"));
    }

    #[test]
    fn test_shell_hook_stderr_in_error() {
        // 验证失败时 stderr 内容包含在错误信息中
        let hook = ShellHook {
            command: r#"echo "something went wrong" >&2; exit 1"#.to_string(),
            timeout: 5,
            on_error: OnError::Skip,
            filter: HookFilter::default(),
        };
        let ctx = HookContext::default();
        let result = execute_shell_hook(&hook, &ctx);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("stderr:"), "错误信息应包含 stderr: {}", err);
        assert!(
            err.contains("something went wrong"),
            "错误信息应包含 stderr 内容: {}",
            err
        );
    }

    #[test]
    fn test_hook_entry_session_index() {
        // 验证 list_hooks 为 session hook 返回正确的局部索引
        let mut manager = HookManager::default();

        // 注册 builtin hook（不应有 session_index）
        manager.register_builtin(HookEvent::PreSendMessage, "test", |_| None);

        // 注册两个 session hook
        manager.register_session_hook(
            HookEvent::PreSendMessage,
            HookDef {
                command: "echo first".to_string(),
                timeout: 5,
                on_error: OnError::Skip,
                filter: HookFilter::default(),
            },
        );
        manager.register_session_hook(
            HookEvent::PreSendMessage,
            HookDef {
                command: "echo second".to_string(),
                timeout: 5,
                on_error: OnError::Abort,
                filter: HookFilter::default(),
            },
        );

        let hooks = manager.list_hooks();
        assert_eq!(hooks.len(), 3);

        // builtin hook 无 session_index
        assert_eq!(hooks[0].source, "builtin");
        assert!(hooks[0].session_index.is_none());
        assert!(hooks[0].on_error.is_none());

        // 第一个 session hook
        assert_eq!(hooks[1].source, "session");
        assert_eq!(hooks[1].session_index, Some(0));
        assert_eq!(hooks[1].on_error, Some(OnError::Skip));

        // 第二个 session hook
        assert_eq!(hooks[2].source, "session");
        assert_eq!(hooks[2].session_index, Some(1));
        assert_eq!(hooks[2].on_error, Some(OnError::Abort));
    }

    #[test]
    fn test_switch_model_field_removed() {
        // 验证旧脚本返回 _switch_model 字段时不会报错（serde 静默忽略未知字段）
        let json = r#"{"user_input": "test", "_switch_model": "gpt-4"}"#;
        let result: HookResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.user_input.as_deref(), Some("test"));
    }
}
