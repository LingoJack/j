use super::super::permission::JcliConfig;
use super::super::storage::{ChatMessage, ModelProvider};
use crate::command::chat::constants::{
    HOOK_DEFAULT_LLM_TIMEOUT_SECS, HOOK_DEFAULT_TIMEOUT_SECS, HOOK_LLM_MAX_TOKENS,
};
use crate::config::YamlConfig;
use crate::util::log::{write_error_log, write_info_log};
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
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
/// stop / skip 语义（统一规则）：
/// - `stop`：中止当前步骤及其所属子管线（不发送/不请求/不结束/不保存/中止 compact）
/// - `skip`：跳过当前步骤，同级步骤继续（仅 PreToolExecution：跳过该工具，其他工具继续）
///
/// | 事件                          | 触发时机           | 可读字段                              | 可写字段（HookResult 中返回即生效）        |
/// |-------------------------------|--------------------|-----------------------------------------|----------------------------------------------|
/// | `PreSendMessage`              | 用户消息入队前     | `user_input`, `messages`               | `user_input`（修改发送内容）, `action=stop`, `retry_feedback` |
/// | `PostSendMessage`             | 用户消息入队后     | `user_input`, `messages`               | 仅通知，返回值被忽略                         |
/// | `PreLlmRequest`               | LLM API 请求前     | `messages`, `system_prompt`, `model`   | `messages`, `system_prompt`, `inject_messages`, `additional_context`, `action=stop`, `retry_feedback` |
/// | `PostLlmResponse`             | LLM 回复完成后     | `assistant_output`, `messages`, `model` | `assistant_output`（修改最终回复）, `action=stop`, `retry_feedback`, `system_message` |
/// | `PreToolExecution`            | 工具执行前         | `tool_name`, `tool_arguments`          | `tool_arguments`（修改参数）, `action=skip`  |
/// | `PostToolExecution`           | 工具执行后         | `tool_name`, `tool_result`             | `tool_result`（修改结果）                    |
/// | `PostToolExecutionFailure`    | 工具执行失败后     | `tool_name`, `tool_error`              | `tool_error`（修改错误信息）, `additional_context` |
/// | `Stop`                        | LLM 即将结束回复   | `user_input`（回复文本）, `messages`, `system_prompt`, `model` | `retry_feedback`（带反馈重试）, `additional_context`, `action=stop` |
/// | `PreMicroCompact`             | micro_compact 前   | `messages`, `model`                   | `action=stop`                               |
/// | `PostMicroCompact`            | micro_compact 后   | `messages`                             | `messages`（修改压缩结果）                    |
/// | `PreAutoCompact`              | auto_compact 前    | `messages`, `system_prompt`, `model`   | `additional_context`, `action=stop`         |
/// | `PostAutoCompact`             | auto_compact 后    | `messages`                             | `messages`（修改压缩结果）                    |
/// | `SessionStart`                | 会话启动时         | `messages`                             | 仅通知，返回值被忽略                         |
/// | `SessionEnd`                  | 会话退出时         | `messages`                             | 仅通知，返回值被忽略                         |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookEvent {
    PreSendMessage,
    PostSendMessage,
    PreLlmRequest,
    PostLlmResponse,
    PreToolExecution,
    PostToolExecution,
    PostToolExecutionFailure,
    Stop,
    PreMicroCompact,
    PostMicroCompact,
    PreAutoCompact,
    PostAutoCompact,
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
            "post_tool_execution_failure" => Ok(HookEvent::PostToolExecutionFailure),
            "stop" => Ok(HookEvent::Stop),
            "pre_micro_compact" => Ok(HookEvent::PreMicroCompact),
            "post_micro_compact" => Ok(HookEvent::PostMicroCompact),
            "pre_auto_compact" => Ok(HookEvent::PreAutoCompact),
            "post_auto_compact" => Ok(HookEvent::PostAutoCompact),
            "session_start" => Ok(HookEvent::SessionStart),
            "session_end" => Ok(HookEvent::SessionEnd),
            _ => Err(()),
        }
    }
}

impl HookEvent {
    /// 返回 Hook 事件的字符串标识（如 "pre_send_message"）
    pub fn as_str(&self) -> &'static str {
        match self {
            HookEvent::PreSendMessage => "pre_send_message",
            HookEvent::PostSendMessage => "post_send_message",
            HookEvent::PreLlmRequest => "pre_llm_request",
            HookEvent::PostLlmResponse => "post_llm_response",
            HookEvent::PreToolExecution => "pre_tool_execution",
            HookEvent::PostToolExecution => "post_tool_execution",
            HookEvent::PostToolExecutionFailure => "post_tool_execution_failure",
            HookEvent::Stop => "stop",
            HookEvent::PreMicroCompact => "pre_micro_compact",
            HookEvent::PostMicroCompact => "post_micro_compact",
            HookEvent::PreAutoCompact => "pre_auto_compact",
            HookEvent::PostAutoCompact => "post_auto_compact",
            HookEvent::SessionStart => "session_start",
            HookEvent::SessionEnd => "session_end",
        }
    }

    /// 返回所有 HookEvent 枚举值的静态切片，用于遍历/校验
    pub fn all() -> &'static [HookEvent] {
        &[
            HookEvent::PreSendMessage,
            HookEvent::PostSendMessage,
            HookEvent::PreLlmRequest,
            HookEvent::PostLlmResponse,
            HookEvent::PreToolExecution,
            HookEvent::PostToolExecution,
            HookEvent::PostToolExecutionFailure,
            HookEvent::Stop,
            HookEvent::PreMicroCompact,
            HookEvent::PostMicroCompact,
            HookEvent::PreAutoCompact,
            HookEvent::PostAutoCompact,
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
    /// 中止整条 hook 链
    Stop,
}

/// Hook 条件过滤：仅当条件匹配时才执行该 hook
///
/// 所有字段为可选，未设置的字段不参与过滤（即视为匹配）。
/// 多个字段同时设置时取 AND 关系。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HookFilter {
    /// 工具名过滤（精确匹配，仅对工具相关事件生效）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    /// 工具名模式匹配（管道分隔，如 "Write|Edit|Bash"，仅对工具相关事件生效）
    /// 优先级低于 tool_name：当 tool_name 设置时忽略此字段
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_matcher: Option<String>,
    /// 模型名前缀过滤（如 "gpt-4" 匹配 "gpt-4o"、"gpt-4-turbo"）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_prefix: Option<String>,
}

impl HookFilter {
    /// 是否为空过滤器（无任何条件，始终匹配）
    pub fn is_empty(&self) -> bool {
        self.tool_name.is_none() && self.tool_matcher.is_none() && self.model_prefix.is_none()
    }

    /// 根据 HookContext 判断是否匹配
    pub fn matches(&self, context: &HookContext) -> bool {
        // 精确匹配 tool_name（优先级最高）
        if let Some(ref expected_tool) = self.tool_name {
            match &context.tool_name {
                Some(actual) if actual == expected_tool => {}
                Some(_) => return false,
                None => return false,
            }
        } else if let Some(ref pattern) = self.tool_matcher {
            // 管道分隔模式匹配（如 "Write|Edit|Bash"）
            let actual = match &context.tool_name {
                Some(a) => a,
                None => return false,
            };
            let matched = pattern.split('|').any(|p| p.trim() == actual);
            if !matched {
                return false;
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

/// Hook 类型（YAML `type` 字段）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum HookType {
    /// Shell 命令 hook（默认，通过 `sh -c` 子进程执行）
    #[default]
    Bash,
    /// LLM hook（通过 prompt 模板调用 LLM，返回 HookResult JSON）
    Llm,
}

impl std::fmt::Display for HookType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HookType::Bash => write!(f, "bash"),
            HookType::Llm => write!(f, "llm"),
        }
    }
}

/// Hook 定义（YAML 兼容）：支持 bash 和 llm 两种类型
///
/// YAML 示例（bash）：
/// ```yaml
/// - command: "echo '{\"user_input\": \"hooked\"}'"
///   timeout: 10
///   on_error: skip
/// ```
///
/// YAML 示例（llm）：
/// ```yaml
/// - type: llm
///   prompt: |
///     检查以下用户输入是否包含敏感信息：
///     {{user_input}}
///     如果包含，返回 action=stop + retry_feedback。
///   timeout: 30
///   retry: 1
///   on_error: skip
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookDef {
    /// Hook 类型：bash（默认）或 llm
    #[serde(default)]
    pub r#type: HookType,
    /// Shell 命令（type=bash 时必填，通过 `sh -c` 执行）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// LLM prompt 模板（type=llm 时必填，支持 {{variable}} 模板变量）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// LLM 模型名覆盖（type=llm 时可选，空则使用当前活跃 provider 的模型）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// 超时秒数（bash 默认 10，llm 默认 30）
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    /// 重试次数（仅 Err 路径生效，默认 0 即不重试）
    #[serde(default)]
    pub retry: u32,
    /// 脚本/LLM 失败时的处理策略（默认 skip）
    #[serde(default)]
    pub on_error: OnError,
    /// 条件过滤：仅当条件匹配时执行（默认无过滤）
    #[serde(default, skip_serializing_if = "HookFilter::is_empty")]
    pub filter: HookFilter,
}

fn default_timeout() -> u64 {
    HOOK_DEFAULT_TIMEOUT_SECS
}

fn default_llm_timeout() -> u64 {
    HOOK_DEFAULT_LLM_TIMEOUT_SECS
}

// ========== HookKind 枚举 ==========

/// Hook 种类：Shell 命令（子进程）、LLM（prompt 模板调 LLM）、内置 Rust 闭包（进程内）
#[derive(Clone)]
pub enum HookKind {
    /// Shell 命令，通过 `sh -c` 子进程执行（现有行为）
    Shell(ShellHook),
    /// LLM hook，通过 prompt 模板调用 LLM API，返回 HookResult JSON
    Llm(LlmHook),
    /// 内置 Rust 闭包，进程内零开销执行
    Builtin(BuiltinHook),
}

/// Shell hook：一条命令 + 超时 + 失败策略 + 条件过滤
#[derive(Debug, Clone)]
pub struct ShellHook {
    /// Hook 目录名（目录布局下有值，session hook 为 None）
    pub name: Option<String>,
    pub command: String,
    pub timeout: u64,
    pub retry: u32,
    pub on_error: OnError,
    pub filter: HookFilter,
    /// Hook 目录路径（目录布局下有值，session hook 为 None）
    pub dir_path: Option<PathBuf>,
}

/// LLM hook：prompt 模板 + 模型覆盖 + 超时 + 重试 + 失败策略 + 条件过滤
#[derive(Debug, Clone)]
pub struct LlmHook {
    /// Hook 目录名（目录布局下有值，session hook 为 None）
    pub name: Option<String>,
    /// Prompt 模板，支持 {{variable}} 模板变量
    pub prompt: String,
    /// 模型名覆盖（空则使用当前活跃 provider 的模型）
    pub model: Option<String>,
    /// 超时秒数
    pub timeout: u64,
    /// 重试次数（仅 Err 路径生效）
    pub retry: u32,
    /// 失败策略
    pub on_error: OnError,
    /// 条件过滤
    pub filter: HookFilter,
    /// Hook 目录路径（目录布局下有值，session hook 为 None）
    #[allow(dead_code)]
    pub dir_path: Option<PathBuf>,
}

impl HookDef {
    /// 转换为 HookKind（根据 type 字段分派）
    pub fn into_hook_kind(self) -> Result<HookKind, String> {
        match self.r#type {
            HookType::Bash => {
                let command = self.command.unwrap_or_default();
                if command.is_empty() {
                    return Err("bash hook 缺少 command 字段".to_string());
                }
                Ok(HookKind::Shell(ShellHook {
                    name: None,
                    command,
                    timeout: self.timeout,
                    retry: self.retry,
                    on_error: self.on_error,
                    filter: self.filter,
                    dir_path: None,
                }))
            }
            HookType::Llm => {
                let prompt = self.prompt.unwrap_or_default();
                if prompt.is_empty() {
                    return Err("llm hook 缺少 prompt 字段".to_string());
                }
                Ok(HookKind::Llm(LlmHook {
                    name: None,
                    prompt,
                    model: self.model,
                    timeout: if self.timeout == default_timeout() {
                        default_llm_timeout()
                    } else {
                        self.timeout
                    },
                    retry: if self.retry == 0 { 1 } else { self.retry },
                    on_error: self.on_error,
                    filter: self.filter,
                    dir_path: None,
                }))
            }
        }
    }
}

impl From<HookDef> for HookKind {
    fn from(def: HookDef) -> Self {
        def.into_hook_kind().unwrap_or_else(|e| {
            write_error_log("HookDef::into_hook_kind", &e);
            // 回退到空 Shell hook（不会执行有效操作，但不会 panic）
            HookKind::Shell(ShellHook {
                name: None,
                command: String::new(),
                timeout: 0,
                retry: 0,
                on_error: OnError::Skip,
                filter: HookFilter::default(),
                dir_path: None,
            })
        })
    }
}

// ========== HookDirDef（目录布局下的 HOOK.yaml 格式）==========

/// HOOK.yaml 定义（目录布局下的格式）
///
/// 与 `HookDef` 的区别：`events` 为列表（一个 hook 可绑定多个事件），无 `command`/`prompt` 以外的不必要字段。
/// 目录布局下 `command` 中的相对路径以 hook 目录为 cwd 解析。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookDirDef {
    /// 绑定的事件列表
    pub events: Vec<HookEvent>,
    /// Hook 类型
    #[serde(default)]
    pub r#type: HookType,
    /// Shell 命令（type=bash 时必填，通过 `sh -c` 执行，cwd 为 hook 目录）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// LLM prompt 模板（type=llm 时必填，支持 {{variable}} 模板变量）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// LLM 模型名覆盖
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// 超时秒数（bash 默认 10，llm 默认 30）
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    /// 重试次数（仅 Err 路径生效）
    #[serde(default)]
    pub retry: u32,
    /// 失败策略
    #[serde(default)]
    pub on_error: OnError,
    /// 条件过滤
    #[serde(default, skip_serializing_if = "HookFilter::is_empty")]
    pub filter: HookFilter,
}

impl HookDirDef {
    /// 转换为 `Vec<(HookEvent, HookKind)>`（每个 event 一个条目）
    pub fn into_hook_kinds(
        self,
        name: &str,
        dir_path: &Path,
    ) -> Result<Vec<(HookEvent, HookKind)>, String> {
        if self.events.is_empty() {
            return Err(format!("hook '{}' 的 events 为空", name));
        }
        let kind = match self.r#type {
            HookType::Bash => {
                let command = self.command.unwrap_or_default();
                if command.is_empty() {
                    return Err(format!("bash hook '{}' 缺少 command 字段", name));
                }
                HookKind::Shell(ShellHook {
                    name: Some(name.to_string()),
                    command,
                    timeout: self.timeout,
                    retry: self.retry,
                    on_error: self.on_error,
                    filter: self.filter,
                    dir_path: Some(dir_path.to_path_buf()),
                })
            }
            HookType::Llm => {
                let prompt = self.prompt.unwrap_or_default();
                if prompt.is_empty() {
                    return Err(format!("llm hook '{}' 缺少 prompt 字段", name));
                }
                HookKind::Llm(LlmHook {
                    name: Some(name.to_string()),
                    prompt,
                    model: self.model,
                    timeout: if self.timeout == default_timeout() {
                        default_llm_timeout()
                    } else {
                        self.timeout
                    },
                    retry: if self.retry == 0 { 1 } else { self.retry },
                    on_error: self.on_error,
                    filter: self.filter,
                    dir_path: Some(dir_path.to_path_buf()),
                })
            }
        };
        Ok(self.events.into_iter().map(|e| (e, kind.clone())).collect())
    }
}

// ========== 目录加载函数 ==========

/// 返回用户级 hooks 目录: ~/.jdata/agent/hooks/
pub fn hooks_dir() -> PathBuf {
    let dir = YamlConfig::data_dir().join("agent").join("hooks");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// 返回项目级 hooks 目录: .jcli/hooks/（如果存在）
pub fn project_hooks_dir() -> Option<PathBuf> {
    let config_dir = JcliConfig::find_config_dir()?;
    let dir = config_dir.join("hooks");
    if dir.is_dir() { Some(dir) } else { None }
}

/// 从指定目录加载 hooks（遍历子目录，解析 HOOK.yaml）
fn load_hooks_from_dir(dir: &Path, source_name: &str) -> Vec<(String, HookDirDef, PathBuf)> {
    let mut hooks = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return hooks,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let hook_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // 跳过 example 目录（模板示例，不是实际可执行的 hook）
        if hook_name == "example" {
            continue;
        }

        let hook_yaml = path.join("HOOK.yaml");
        if !hook_yaml.exists() {
            continue;
        }
        match std::fs::read_to_string(&hook_yaml) {
            Ok(content) => match serde_yaml::from_str::<HookDirDef>(&content) {
                Ok(def) => {
                    if def.events.is_empty() {
                        write_error_log(
                            "load_hooks_from_dir",
                            &format!("hook '{}' 的 events 为空，跳过", hook_name),
                        );
                        continue;
                    }
                    hooks.push((hook_name, def, path));
                }
                Err(e) => write_error_log(
                    "load_hooks_from_dir",
                    &format!("解析 {}/HOOK.yaml 失败: {}", hook_name, e),
                ),
            },
            Err(e) => write_error_log(
                "load_hooks_from_dir",
                &format!("读取 {}/HOOK.yaml 失败: {}", hook_name, e),
            ),
        }
    }
    write_info_log(
        "load_hooks_from_dir",
        &format!("从 {} 加载了 {} 个 hook", source_name, hooks.len()),
    );
    hooks
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
                .field("name", &shell.name)
                .field("command", &shell.command)
                .field("timeout", &shell.timeout)
                .field("on_error", &shell.on_error)
                .finish(),
            HookKind::Llm(llm) => f
                .debug_struct("HookKind::Llm")
                .field("name", &llm.name)
                .field("prompt", &llm.prompt.len())
                .field("model", &llm.model)
                .field("timeout", &llm.timeout)
                .field("retry", &llm.retry)
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

    /// 工具执行失败原因
    /// - 可读事件：PostToolExecutionFailure（可通过 HookResult 修改）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_error: Option<String>,

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
            tool_error: None,
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
/// - `messages`：仅 PreLlmRequest / PostMicroCompact / PostAutoCompact 中生效，替换消息列表
/// - `system_prompt`：仅 PreLlmRequest 中生效，替换系统提示词
/// - `tool_arguments`：仅 PreToolExecution 中生效，替换工具调用参数
/// - `tool_result`：仅 PostToolExecution 中生效，替换工具返回结果
/// - `tool_error`：仅 PostToolExecutionFailure 中生效，替换工具错误信息
/// - `inject_messages`：仅 PreLlmRequest 中生效，追加到消息列表末尾
/// - `retry_feedback`：Pre*/Stop/PostLlmResponse 中生效，中止并带反馈重试（注入为 user message 重新请求 LLM）
/// - `additional_context`：PreLlmRequest / Stop / PreAutoCompact 中生效，追加文本到 system_prompt 末尾
/// - `system_message`：所有事件中生效，展示给用户的提示消息
/// - `action`：`"stop"` 中止当前步骤及其所属子管线；`"skip"` 跳过当前步骤（同级继续）。旧字段 `abort=true` 等价于 `action="stop"`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookAction {
    /// 中止当前步骤及其所属子管线
    Stop,
    /// 跳过当前步骤，同级步骤继续
    Skip,
}

/// Hook 执行结果：允许替换消息列表、系统提示词、用户输入、工具参数等
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
    /// 替换工具执行失败原因（PostToolExecutionFailure）
    #[serde(default)]
    pub tool_error: Option<String>,
    /// 追加消息到消息列表末尾（PreLlmRequest）
    #[serde(default)]
    pub inject_messages: Option<Vec<ChatMessage>>,
    /// 审查反馈（Pre*/Stop/PostLlmResponse）：中止时附带反馈文本，触发 LLM 带反馈重试
    #[serde(default)]
    pub retry_feedback: Option<String>,
    /// 注入到模型上下文的额外信息（PreLlmRequest/Stop/PreAutoCompact）：纯文本追加到 system_prompt 末尾
    #[serde(default)]
    pub additional_context: Option<String>,
    /// 展示给用户的系统消息（所有事件：UI 上以 toast/提示形式显示）
    #[serde(default)]
    pub system_message: Option<String>,
    /// 控制流动作：`stop` = 中止当前步骤及其所属子管线，`skip` = 跳过当前步骤（同级继续）
    #[serde(default)]
    pub action: Option<HookAction>,
}

impl HookResult {
    /// 是否请求 stop（中止当前步骤及其所属子管线）
    pub fn is_stop(&self) -> bool {
        self.action == Some(HookAction::Stop)
    }

    /// 是否请求 skip（跳过当前步骤，同级继续）
    pub fn is_skip(&self) -> bool {
        self.action == Some(HookAction::Skip)
    }

    /// 是否请求 stop 或 skip（任何控制流中断）
    pub fn is_halt(&self) -> bool {
        self.is_stop() || self.is_skip()
    }
}

// ========== HookOutcome（三态结果）==========

/// Hook 执行的三态结果
///
/// - `Success`：执行成功，可能包含修改
/// - `Retry`：执行失败但还有重试机会
/// - `Err`：执行失败（重试耗尽或不可重试）
#[derive(Debug)]
#[allow(dead_code, clippy::large_enum_variant)]
enum HookOutcome {
    Success(HookResult),
    Retry {
        error: String,
        #[allow(dead_code)]
        attempts_left: u32,
    },
    Err(String),
}

// ========== HookManager ==========

/// 单个 hook 的执行统计
#[derive(Debug, Clone, Default, PartialEq)]
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
/// 前者的输出会更新到 context 中，影响后者的输入。任何 `stop` 或 `skip` 立即中止整条链。
#[derive(Debug, Default)]
pub struct HookManager {
    builtin_hooks: HashMap<HookEvent, Vec<HookKind>>,
    user_hooks: HashMap<HookEvent, Vec<HookKind>>,
    project_hooks: HashMap<HookEvent, Vec<HookKind>>,
    session_hooks: HashMap<HookEvent, Vec<HookKind>>,
    /// 按 hook label 记录的执行指标（内部可变，execute 不需要 &mut self）
    metrics: Mutex<HashMap<String, HookMetrics>>,
    /// 当前活跃的 LLM provider（LLM hook 执行时使用）
    provider: Option<Arc<Mutex<ModelProvider>>>,
}

impl Clone for HookManager {
    fn clone(&self) -> Self {
        HookManager {
            builtin_hooks: self.builtin_hooks.clone(),
            user_hooks: self.user_hooks.clone(),
            project_hooks: self.project_hooks.clone(),
            session_hooks: self.session_hooks.clone(),
            metrics: Mutex::new(self.metrics.lock().map(|m| m.clone()).unwrap_or_default()),
            provider: self.provider.clone(),
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
    /// Hook 目录名（目录布局下有值）
    pub name: Option<String>,
    pub event: HookEvent,
    pub source: &'static str,
    /// Hook 类型标签（bash / llm / builtin）
    pub hook_type: &'static str,
    /// Shell hook 的命令，LLM hook 的 prompt 摘要，或 Builtin hook 的名称
    pub label: String,
    /// Hook 的超时秒数
    pub timeout: Option<u64>,
    /// Hook 的失败策略
    pub on_error: Option<OnError>,
    /// Session hook 在该 event 下的局部索引（仅 session 来源有值，用于 remove 操作）
    pub session_index: Option<usize>,
    /// 条件过滤
    pub filter: Option<HookFilter>,
    /// 执行指标
    pub metrics: Option<HookMetrics>,
    /// Hook 唯一标识，格式：`builtin:<name>` / `user:<dir_name>` / `project:<dir_name>` / `session:<event_idx>`
    pub unique_id: String,
}

/// 生成 hook 唯一标识，格式：`source:unique_key`
pub fn hook_unique_id(source: &str, kind: &HookKind, session_index: Option<usize>) -> String {
    let key = match kind {
        HookKind::Builtin(b) => b.name.clone(),
        HookKind::Shell(s) => s
            .name
            .clone()
            .unwrap_or_else(|| s.command.chars().take(40).collect()),
        HookKind::Llm(l) => l
            .name
            .clone()
            .unwrap_or_else(|| l.prompt.chars().take(40).collect()),
    };
    match session_index {
        Some(idx) => format!("{}:{}", source, idx),
        None => format!("{}:{}", source, key),
    }
}

impl HookManager {
    /// 加载用户级（`~/.jdata/agent/hooks/`）+ 项目级（`.jcli/hooks/`）hook
    pub fn load() -> Self {
        let mut manager = HookManager::default();

        // 加载用户级 hooks: ~/.jdata/agent/hooks/
        let user_dir = hooks_dir();
        if user_dir.is_dir() {
            for (name, dir_def, dir_path) in load_hooks_from_dir(&user_dir, "用户级") {
                match dir_def.into_hook_kinds(&name, &dir_path) {
                    Ok(pairs) => {
                        for (event, kind) in pairs {
                            manager.user_hooks.entry(event).or_default().push(kind);
                        }
                    }
                    Err(e) => write_error_log("HookManager::load", &e),
                }
            }
            write_info_log(
                "HookManager::load",
                &format!("已加载用户级 hooks: {}", user_dir.display()),
            );
        }

        // 加载项目级 hooks: .jcli/hooks/
        if let Some(proj_dir) = project_hooks_dir() {
            for (name, dir_def, dir_path) in load_hooks_from_dir(&proj_dir, "项目级") {
                match dir_def.into_hook_kinds(&name, &dir_path) {
                    Ok(pairs) => {
                        for (event, kind) in pairs {
                            manager.project_hooks.entry(event).or_default().push(kind);
                        }
                    }
                    Err(e) => write_error_log("HookManager::load", &e),
                }
            }
            write_info_log(
                "HookManager::load",
                &format!("已加载项目级 hooks: {}", proj_dir.display()),
            );
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
        match def.into_hook_kind() {
            Ok(kind) => {
                self.session_hooks.entry(event).or_default().push(kind);
            }
            Err(e) => {
                write_error_log("HookManager::register_session_hook", &e);
            }
        }
    }

    /// 获取所有 session 级 hook 的可序列化快照（用于 session 持久化）
    /// 只保存 Shell 和 Llm 类型（Builtin 不可序列化）
    pub fn session_hooks_snapshot(&self) -> Vec<super::super::storage::SessionHookPersist> {
        let mut result = Vec::new();
        for (event, hooks) in &self.session_hooks {
            for kind in hooks {
                match kind {
                    HookKind::Shell(sh) => {
                        result.push(super::super::storage::SessionHookPersist {
                            event: *event,
                            definition: HookDef {
                                r#type: HookType::Bash,
                                command: Some(sh.command.clone()),
                                prompt: None,
                                model: None,
                                timeout: sh.timeout,
                                retry: sh.retry,
                                on_error: sh.on_error,
                                filter: sh.filter.clone(),
                            },
                        });
                    }
                    HookKind::Llm(lh) => {
                        result.push(super::super::storage::SessionHookPersist {
                            event: *event,
                            definition: HookDef {
                                r#type: HookType::Llm,
                                command: None,
                                prompt: Some(lh.prompt.clone()),
                                model: lh.model.clone(),
                                timeout: lh.timeout,
                                retry: lh.retry,
                                on_error: lh.on_error,
                                filter: lh.filter.clone(),
                            },
                        });
                    }
                    HookKind::Builtin(_) => {
                        // 内置 hook 不可序列化，跳过
                    }
                }
            }
        }
        result
    }

    /// 清除所有 session 级 hook（session 切换时使用）
    pub fn clear_session_hooks(&mut self) {
        self.session_hooks.clear();
    }

    /// 从持久化快照恢复 session 级 hook
    pub fn restore_session_hooks(&mut self, hooks: &[super::super::storage::SessionHookPersist]) {
        self.session_hooks.clear();
        for hook in hooks {
            self.register_session_hook(hook.event, hook.definition.clone());
        }
    }

    /// 注册 session 级 hook（直接传入 HookKind）
    #[allow(dead_code)]
    pub fn register_session_hook_kind(&mut self, event: HookEvent, kind: HookKind) {
        self.session_hooks.entry(event).or_default().push(kind);
    }

    /// 注入 LLM provider（用于 LLM hook 执行）
    pub fn set_provider(&mut self, provider: Arc<Mutex<ModelProvider>>) {
        self.provider = Some(provider);
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
            let uid = hook_unique_id(source, hook, session_index);
            HookEntry {
                name: hook_name(hook).map(|s| s.to_string()),
                event,
                source,
                hook_type: hook_type_str(hook),
                timeout: hook_timeout(hook),
                on_error: hook_on_error(hook),
                filter: hook_filter(hook).cloned(),
                metrics: metrics.get(&label).cloned(),
                session_index,
                label,
                unique_id: uid,
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
    /// 重新读取 `~/.jdata/agent/hooks/` 和 `.jcli/hooks/` 目录，
    /// 替换当前的 user_hooks 和 project_hooks（builtin 和 session 级不受影响）。
    /// 指标数据和 provider 保留不清零。
    #[allow(dead_code)]
    pub fn reload(&mut self) {
        let fresh = HookManager::load();
        self.user_hooks = fresh.user_hooks;
        self.project_hooks = fresh.project_hooks;
        // provider 和 metrics 保留
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
        disabled_hooks: Vec<String>,
    ) {
        std::thread::spawn(move || {
            if let Ok(m) = manager.lock() {
                let _ = m.execute(event, context, &disabled_hooks);
            }
        });
    }

    /// 链式执行所有 hook（内置→用户→项目→session）
    ///
    /// 返回 `Some(HookResult)` 如果有任何修改或 stop/skip，否则 `None`。
    /// 链式执行中，前一个 hook 的输出会更新到 context 中，成为下一个 hook 的输入。
    /// `disabled_hooks` 为被禁用的 hook 标识列表（来自 AgentConfig.disabled_hooks）。
    ///
    /// **注意**：调用方应先用 `has_hooks_for()` 检查，再构建 HookContext 并调用此方法，
    /// 避免在没有 hook 注册时进行不必要的内存分配。
    pub fn execute(
        &self,
        event: HookEvent,
        mut context: HookContext,
        disabled_hooks: &[String],
    ) -> Option<HookResult> {
        // 收集所有 hook 及其 source 标识
        struct HookRef<'a> {
            kind: &'a HookKind,
            source: &'static str,
            session_index: Option<usize>,
        }

        let mut all_hooks: Vec<HookRef<'_>> = Vec::new();

        // 执行顺序：内置 → 用户 → 项目 → session
        if let Some(hooks) = self.builtin_hooks.get(&event) {
            for h in hooks.iter() {
                all_hooks.push(HookRef {
                    kind: h,
                    source: HOOK_SOURCE_BUILTIN,
                    session_index: None,
                });
            }
        }
        if let Some(hooks) = self.user_hooks.get(&event) {
            for h in hooks.iter() {
                all_hooks.push(HookRef {
                    kind: h,
                    source: HOOK_SOURCE_USER,
                    session_index: None,
                });
            }
        }
        if let Some(hooks) = self.project_hooks.get(&event) {
            for h in hooks.iter() {
                all_hooks.push(HookRef {
                    kind: h,
                    source: HOOK_SOURCE_PROJECT,
                    session_index: None,
                });
            }
        }
        if let Some(hooks) = self.session_hooks.get(&event) {
            for (idx, h) in hooks.iter().enumerate() {
                all_hooks.push(HookRef {
                    kind: h,
                    source: HOOK_SOURCE_SESSION,
                    session_index: Some(idx),
                });
            }
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

        for hook_ref in &all_hooks {
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

            let label = hook_label(hook_ref.kind);

            // 禁用检查
            let uid = hook_unique_id(hook_ref.source, hook_ref.kind, hook_ref.session_index);
            if disabled_hooks.contains(&uid) {
                if let Ok(mut metrics) = self.metrics.lock() {
                    let m = metrics.entry(label).or_default();
                    m.skipped += 1;
                }
                continue;
            }

            // 条件过滤检查
            if !hook_should_execute(hook_ref.kind, &context) {
                if let Ok(mut metrics) = self.metrics.lock() {
                    let m = metrics.entry(label).or_default();
                    m.skipped += 1;
                }
                continue;
            }

            let max_attempts = 1 + hook_retry_count(hook_ref.kind); // 1 + retry
            let mut last_outcome = None;

            for attempt in 0..max_attempts {
                // 链总超时检查（每次重试前也检查）
                if chain_start.elapsed() > chain_timeout {
                    write_error_log(
                        "HookManager::execute",
                        &format!(
                            "Hook 链总超时，中止 {} 的重试 (事件: {})",
                            label,
                            event.as_str()
                        ),
                    );
                    last_outcome = Some(HookOutcome::Err(format!(
                        "链总超时，第 {} 次尝试中止",
                        attempt + 1
                    )));
                    break;
                }

                let hook_start = std::time::Instant::now();
                let result = execute_hook_with_provider(hook_ref.kind, &context, &self.provider);

                let elapsed_ms = hook_start.elapsed().as_millis() as u64;

                match result {
                    Ok(hook_result) => {
                        if let Ok(mut metrics) = self.metrics.lock() {
                            let m = metrics.entry(label.clone()).or_default();
                            m.executions += 1;
                            m.successes += 1;
                            m.total_duration_ms += elapsed_ms;
                        }

                        if hook_result.is_halt() {
                            let action_str = if hook_result.is_stop() {
                                "stop"
                            } else {
                                "skip"
                            };
                            write_info_log(
                                "HookManager::execute",
                                &format!("Hook {} ({})", action_str, label),
                            );
                            return Some(HookResult {
                                action: Some(if hook_result.is_stop() {
                                    HookAction::Stop
                                } else {
                                    HookAction::Skip
                                }),
                                retry_feedback: hook_result.retry_feedback.clone(),
                                system_message: hook_result.system_message.clone(),
                                ..Default::default()
                            });
                        }

                        // 合并结果到 context（链式传递）
                        if let Some(ref msgs) = hook_result.messages {
                            context.messages = Some(msgs.clone());
                            final_result.messages = context.messages.clone();
                            had_modification = true;
                        }
                        if let Some(ref sp) = hook_result.system_prompt {
                            context.system_prompt = Some(sp.clone());
                            final_result.system_prompt = context.system_prompt.clone();
                            had_modification = true;
                        }
                        if let Some(ref ui) = hook_result.user_input {
                            context.user_input = Some(ui.clone());
                            final_result.user_input = context.user_input.clone();
                            had_modification = true;
                        }
                        if let Some(ref ao) = hook_result.assistant_output {
                            context.assistant_output = Some(ao.clone());
                            final_result.assistant_output = context.assistant_output.clone();
                            had_modification = true;
                        }
                        if let Some(ref ta) = hook_result.tool_arguments {
                            context.tool_arguments = Some(ta.clone());
                            final_result.tool_arguments = context.tool_arguments.clone();
                            had_modification = true;
                        }
                        if let Some(ref tr) = hook_result.tool_result {
                            context.tool_result = Some(tr.clone());
                            final_result.tool_result = context.tool_result.clone();
                            had_modification = true;
                        }
                        if let Some(ref inject) = hook_result.inject_messages {
                            let existing =
                                final_result.inject_messages.get_or_insert_with(Vec::new);
                            existing.extend(inject.clone());
                            had_modification = true;
                        }
                        if let Some(ref rf) = hook_result.retry_feedback {
                            final_result.retry_feedback = Some(rf.clone());
                            had_modification = true;
                        }
                        if let Some(ref ac) = hook_result.additional_context {
                            final_result.additional_context = Some(ac.clone());
                            had_modification = true;
                        }
                        if let Some(ref sm) = hook_result.system_message {
                            final_result.system_message = Some(sm.clone());
                            had_modification = true;
                        }
                        if let Some(ref te) = hook_result.tool_error {
                            final_result.tool_error = Some(te.clone());
                            had_modification = true;
                        }

                        last_outcome = Some(HookOutcome::Success(hook_result));
                        break; // 成功，跳出重试循环
                    }
                    Err(e) => {
                        if let Ok(mut metrics) = self.metrics.lock() {
                            let m = metrics.entry(label.clone()).or_default();
                            m.executions += 1;
                            m.failures += 1;
                            m.total_duration_ms += elapsed_ms;
                        }

                        let attempts_left = max_attempts - attempt - 1;
                        if attempts_left > 0 {
                            write_info_log(
                                "HookManager::execute",
                                &format!(
                                    "Hook 执行失败 ({}), 第 {}/{} 次尝试, 剩余重试 {}: {}",
                                    label,
                                    attempt + 1,
                                    max_attempts,
                                    attempts_left,
                                    e
                                ),
                            );
                            last_outcome = Some(HookOutcome::Retry {
                                error: e,
                                attempts_left,
                            });
                            // 继续下一次重试
                        } else {
                            write_error_log(
                                "HookManager::execute",
                                &format!("Hook 执行失败 ({}), 重试耗尽: {}", label, e),
                            );
                            last_outcome = Some(HookOutcome::Err(e));
                            break; // 重试耗尽，跳出
                        }
                    }
                }
            }

            // 处理最终 outcome
            match last_outcome {
                Some(HookOutcome::Success(_)) => {
                    // 已在上面处理过，继续下一个 hook
                }
                Some(HookOutcome::Retry { error, .. }) => {
                    // 理论上不应该到这里（重试循环应该已经处理），但防御性处理
                    write_error_log(
                        "HookManager::execute",
                        &format!("Hook 重试未完成 ({}): {}", label, error),
                    );
                    // 按 on_error 策略处理
                    match hook_on_error_strategy(hook_ref.kind) {
                        OnError::Stop => {
                            return Some(HookResult {
                                action: Some(HookAction::Stop),
                                ..Default::default()
                            });
                        }
                        OnError::Skip => {
                            continue;
                        }
                    }
                }
                Some(HookOutcome::Err(e)) => {
                    // 重试耗尽后的失败，按 on_error 策略处理
                    write_error_log(
                        "HookManager::execute",
                        &format!("Hook 最终失败 ({}): {}", label, e),
                    );
                    match hook_on_error_strategy(hook_ref.kind) {
                        OnError::Stop => {
                            return Some(HookResult {
                                action: Some(HookAction::Stop),
                                ..Default::default()
                            });
                        }
                        OnError::Skip => {
                            continue;
                        }
                    }
                }
                None => {
                    // 不应该发生
                    continue;
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

/// 执行单个 hook（分派到 Shell / LLM / Builtin），不处理重试
fn execute_hook_with_provider(
    kind: &HookKind,
    context: &HookContext,
    provider: &Option<Arc<Mutex<ModelProvider>>>,
) -> Result<HookResult, String> {
    match kind {
        HookKind::Shell(shell) => execute_shell_hook(shell, context),
        HookKind::Llm(llm) => execute_llm_hook(llm, context, provider),
        HookKind::Builtin(builtin) => match (builtin.handler)(context) {
            Some(result) => Ok(result),
            None => Ok(HookResult::default()),
        },
    }
}

/// LLM hook 的 JSON 格式指令（拼接到 prompt 末尾）
const LLM_HOOK_FORMAT_INSTRUCTION: &str = r#"

---
You are a hook function. You MUST respond with ONLY a valid JSON object matching this schema (no markdown, no explanation outside JSON):
{
  "user_input": "string (optional, replace user message)",
  "assistant_output": "string (optional, replace assistant output)",
  "messages": [{"role":"user","content":"..."}] (optional, replace message list),
  "system_prompt": "string (optional, replace system prompt)",
  "tool_arguments": "string (optional, replace tool arguments JSON)",
  "tool_result": "string (optional, replace tool result)",
  "tool_error": "string (optional, replace tool error)",
  "inject_messages": [{"role":"user","content":"..."}] (optional, append messages),
  "action": "stop" or "skip" (optional, stop=abort pipeline, skip=skip current step),
  "retry_feedback": "string (optional, feedback to retry with)",
  "additional_context": "string (optional, append to system_prompt)",
  "system_message": "string (optional, show toast to user)"
}
Return {} if no modification needed."#;

/// 模板变量替换
fn render_prompt_template(template: &str, context: &HookContext) -> String {
    let mut result = template.to_string();
    result = result.replace("{{event}}", context.event.as_str());
    result = result.replace("{{cwd}}", &context.cwd);
    result = result.replace(
        "{{user_input}}",
        context.user_input.as_deref().unwrap_or(""),
    );
    result = result.replace(
        "{{assistant_output}}",
        context.assistant_output.as_deref().unwrap_or(""),
    );
    result = result.replace("{{tool_name}}", context.tool_name.as_deref().unwrap_or(""));
    result = result.replace(
        "{{tool_arguments}}",
        context.tool_arguments.as_deref().unwrap_or(""),
    );
    result = result.replace(
        "{{tool_result}}",
        context.tool_result.as_deref().unwrap_or(""),
    );
    result = result.replace("{{model}}", context.model.as_deref().unwrap_or(""));
    result
}

/// 从 LLM 输出文本中提取 JSON（找第一个 { 到最后一个 } 之间的内容）
fn extract_json_from_llm_output(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    // 从末尾找最后一个 }
    let end = text.rfind('}')?;
    if end > start {
        Some(&text[start..=end])
    } else {
        None
    }
}

/// 执行 LLM hook
///
/// 协议：
/// - 将 prompt 模板渲染后 + JSON 格式指令拼接为完整 prompt
/// - 使用当前活跃 provider（或 LlmHook.model 覆盖）调用 LLM API（非流式）
/// - 解析 LLM 输出为 HookResult JSON
/// - JSON 解析失败 → Err → 触发重试
fn execute_llm_hook(
    hook: &LlmHook,
    context: &HookContext,
    provider_opt: &Option<Arc<Mutex<ModelProvider>>>,
) -> Result<HookResult, String> {
    let provider_arc = provider_opt
        .as_ref()
        .ok_or("LLM hook 无法执行：未注入 provider")?;

    let provider = provider_arc
        .lock()
        .map_err(|e| format!("获取 provider 锁失败: {}", e))?
        .clone();

    // 如果 LlmHook 指定了 model，覆盖 provider 的 model
    let provider = if let Some(ref model) = hook.model {
        let mut p = provider;
        p.model = model.clone();
        p
    } else {
        provider
    };

    // 渲染 prompt 模板 + 拼接格式指令
    let rendered = render_prompt_template(&hook.prompt, context);
    let full_prompt = format!("{}{}", rendered, LLM_HOOK_FORMAT_INSTRUCTION);

    // 构造 API 请求消息
    let system_msg = "You are a hook function. Respond ONLY with the JSON object as instructed.";
    let user_msg = full_prompt.as_str();

    // 使用 reqwest 发送非流式请求（复用 api.rs 中的逻辑模式）
    let url = format!(
        "{}/chat/completions",
        provider.api_base.trim_end_matches('/')
    );
    let request_body = serde_json::json!({
        "model": provider.model,
        "messages": [
            {"role": "system", "content": system_msg},
            {"role": "user", "content": user_msg}
        ],
        "temperature": 0.0,
        "max_tokens": HOOK_LLM_MAX_TOKENS,
    });
    let request_str = serde_json::to_string(&request_body)
        .map_err(|e| format!("序列化 LLM hook 请求失败: {}", e))?;

    // 在新 tokio runtime 中阻塞执行
    let timeout_secs = hook.timeout;
    let rt =
        tokio::runtime::Runtime::new().map_err(|e| format!("创建 tokio runtime 失败: {}", e))?;

    rt.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .build()
            .map_err(|e| format!("创建 HTTP client 失败: {}", e))?;

        let resp = client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", provider.api_key))
            .body(request_str)
            .send()
            .await
            .map_err(|e| format!("LLM hook 请求失败: {}", e))?;

        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| format!("读取 LLM hook 响应失败: {}", e))?;

        if !status.is_success() {
            return Err(format!(
                "LLM hook API 错误: HTTP {} (body: {})",
                status,
                &body[..body.len().min(500)]
            ));
        }

        // 解析 OpenAI 兼容响应
        let parsed: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| format!("解析 LLM hook 响应 JSON 失败: {}", e))?;

        let content = parsed["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .trim();

        if content.is_empty() || content == "{}" {
            return Ok(HookResult::default());
        }

        // 从 LLM 输出中提取 JSON
        let json_str = match extract_json_from_llm_output(content) {
            Some(s) => s,
            None => {
                return Err(format!(
                    "LLM hook 输出中未找到 JSON (输出: {})",
                    &content[..content.len().min(500)]
                ));
            }
        };

        let hook_result: HookResult = serde_json::from_str(json_str).map_err(|e| {
            format!(
                "解析 LLM hook JSON 失败: {} (提取的 JSON: {})",
                e,
                &json_str[..json_str.len().min(500)]
            )
        })?;

        write_info_log(
            "execute_llm_hook",
            &format!(
                "LLM hook 完成 (prompt_len={}, model={}), action={:?}",
                hook.prompt.len(),
                provider.model,
                hook_result.action
            ),
        );

        Ok(hook_result)
    })
}

/// 执行 Shell hook 脚本
///
/// 协议：
/// - 执行方式: `sh -c "<command>"`
/// - 工作目录: 用户当前目录（目录布局下，hook 目录会前置到 PATH）
/// - 环境变量: `JCLI_HOOK_EVENT`（事件名）、`JCLI_CWD`（用户当前目录）、`JCLI_HOOK_DIR`（hook 目录）
/// - PATH: 目录布局下，hook 目录前置到 PATH，脚本可直接用文件名调用（如 `script.sh`）
/// - stdin: HookContext JSON
/// - stdout: HookResult JSON（可为空字符串/空 JSON `{}`，表示无修改）
/// - exit 0: 成功
/// - exit ≠0: 视为失败（调用方按 on_error 策略处理）
/// - 超时: kill 子进程，返回 Err
fn execute_shell_hook(hook: &ShellHook, context: &HookContext) -> Result<HookResult, String> {
    let context_json =
        serde_json::to_string(context).map_err(|e| format!("序列化 context 失败: {}", e))?;

    // cwd 始终使用用户当前目录
    let user_cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let hook_dir_str = hook
        .dir_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_default();

    let mut cmd = Command::new("sh");
    cmd.arg("-c")
        .arg(&hook.command)
        .current_dir(&user_cwd)
        .env("JCLI_HOOK_EVENT", context.event.as_str())
        .env("JCLI_CWD", user_cwd.display().to_string())
        .env("JCLI_HOOK_DIR", &hook_dir_str);

    // 目录布局下，将 hook 目录前置到 PATH，脚本可直接用文件名调用
    if let Some(ref hook_dir) = hook.dir_path {
        let existing_path = std::env::var("PATH").unwrap_or_default();
        let new_path = if existing_path.is_empty() {
            hook_dir.display().to_string()
        } else {
            format!("{}:{}", hook_dir.display(), existing_path)
        };
        cmd.env("PATH", new_path);
    }

    let mut child = cmd
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
                &format!(
                    "Hook 完成 (cmd: {}), action={:?}",
                    hook.command, result.action
                ),
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

/// 获取 hook 的名称（目录布局下的目录名）
fn hook_name(kind: &HookKind) -> Option<&str> {
    match kind {
        HookKind::Shell(shell) => shell.name.as_deref(),
        HookKind::Llm(llm) => llm.name.as_deref(),
        HookKind::Builtin(builtin) => Some(&builtin.name),
    }
}

/// 获取 hook 的显示标签（Shell 用命令，LLM 用 prompt 摘要，Builtin 用名称）
fn hook_label(kind: &HookKind) -> String {
    match kind {
        HookKind::Shell(shell) => {
            if let Some(ref name) = shell.name {
                format!("{}: {}", name, shell.command)
            } else {
                shell.command.clone()
            }
        }
        HookKind::Llm(llm) => {
            // 取 prompt 前一行或前 80 字符作为标签
            let first_line = llm
                .prompt
                .lines()
                .find(|l| !l.trim().is_empty())
                .unwrap_or(&llm.prompt);
            let prompt_preview = if first_line.len() > 80 {
                format!("{}...", &first_line[..80])
            } else {
                first_line.to_string()
            };
            if let Some(ref name) = llm.name {
                format!("[llm: {}] {}", name, prompt_preview)
            } else {
                format!("[llm: {}]", prompt_preview)
            }
        }
        HookKind::Builtin(builtin) => format!("[builtin: {}]", builtin.name),
    }
}

/// 获取 hook 类型字符串
fn hook_type_str(kind: &HookKind) -> &'static str {
    match kind {
        HookKind::Shell(_) => "bash",
        HookKind::Llm(_) => "llm",
        HookKind::Builtin(_) => "builtin",
    }
}

/// 获取 hook 的超时秒数
fn hook_timeout(kind: &HookKind) -> Option<u64> {
    match kind {
        HookKind::Shell(shell) => Some(shell.timeout),
        HookKind::Llm(llm) => Some(llm.timeout),
        HookKind::Builtin(_) => None,
    }
}

/// 获取 hook 的重试次数
fn hook_retry_count(kind: &HookKind) -> u32 {
    match kind {
        HookKind::Shell(shell) => shell.retry,
        HookKind::Llm(llm) => llm.retry,
        HookKind::Builtin(_) => 0,
    }
}

/// 获取 hook 的失败策略（用于 list 展示）
fn hook_on_error(kind: &HookKind) -> Option<OnError> {
    match kind {
        HookKind::Shell(shell) => Some(shell.on_error),
        HookKind::Llm(llm) => Some(llm.on_error),
        HookKind::Builtin(_) => None,
    }
}

/// 获取 hook 执行失败时的实际策略（Shell/LLM 按配置，Builtin 一律 Abort）
fn hook_on_error_strategy(kind: &HookKind) -> OnError {
    match kind {
        HookKind::Shell(shell) => shell.on_error,
        HookKind::Llm(llm) => llm.on_error,
        HookKind::Builtin(_) => OnError::Stop,
    }
}

/// 获取 hook 的条件过滤器
fn hook_filter(kind: &HookKind) -> Option<&HookFilter> {
    match kind {
        HookKind::Shell(shell) if !shell.filter.is_empty() => Some(&shell.filter),
        HookKind::Llm(llm) if !llm.filter.is_empty() => Some(&llm.filter),
        _ => None,
    }
}

/// 检查 hook 是否应在当前 context 下执行（无 filter 或 filter 匹配时返回 true）
fn hook_should_execute(kind: &HookKind, context: &HookContext) -> bool {
    match kind {
        HookKind::Shell(shell) => shell.filter.matches(context),
        HookKind::Llm(llm) => llm.filter.matches(context),
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
        assert_eq!(def.r#type, HookType::Bash);
    }

    #[test]
    fn test_hook_def_to_hook_kind_bash() {
        let def = HookDef {
            r#type: HookType::Bash,
            command: Some("echo test".to_string()),
            prompt: None,
            model: None,
            timeout: 5,
            retry: 0,
            on_error: OnError::Skip,
            filter: HookFilter::default(),
        };
        let kind = HookKind::from(def);
        match kind {
            HookKind::Shell(shell) => {
                assert_eq!(shell.command, "echo test");
                assert_eq!(shell.timeout, 5);
            }
            _ => panic!("应该转换为 Shell 变体"),
        }
    }

    #[test]
    fn test_hook_def_to_hook_kind_llm() {
        let def = HookDef {
            r#type: HookType::Llm,
            command: None,
            prompt: Some("检查敏感信息: {{user_input}}".to_string()),
            model: Some("gpt-4o".to_string()),
            timeout: 10, // 使用默认 timeout → 应被替换为 30
            retry: 2,
            on_error: OnError::Skip,
            filter: HookFilter::default(),
        };
        let kind = def.into_hook_kind().unwrap();
        match kind {
            HookKind::Llm(llm) => {
                assert_eq!(llm.prompt, "检查敏感信息: {{user_input}}");
                assert_eq!(llm.model.as_deref(), Some("gpt-4o"));
                assert_eq!(llm.timeout, 30); // 默认 timeout 被替换为 llm 默认值
                assert_eq!(llm.retry, 2);
            }
            _ => panic!("应该转换为 Llm 变体"),
        }
    }

    #[test]
    fn test_hook_def_llm_explicit_timeout() {
        let def = HookDef {
            r#type: HookType::Llm,
            command: None,
            prompt: Some("test prompt".to_string()),
            model: None,
            timeout: 60,
            retry: 0,
            on_error: OnError::Skip,
            filter: HookFilter::default(),
        };
        let kind = def.into_hook_kind().unwrap();
        match kind {
            HookKind::Llm(llm) => {
                assert_eq!(llm.timeout, 60); // 显式设置的超时保留
            }
            _ => panic!("应该转换为 Llm 变体"),
        }
    }

    #[test]
    fn test_hook_def_yaml_with_type() {
        let yaml = r#"
type: llm
prompt: "检查敏感信息"
model: gpt-4o
timeout: 30
retry: 2
"#;
        let def: HookDef = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(def.r#type, HookType::Llm);
        assert_eq!(def.prompt.as_deref(), Some("检查敏感信息"));
        assert_eq!(def.model.as_deref(), Some("gpt-4o"));
        assert_eq!(def.timeout, 30);
        assert_eq!(def.retry, 2);
    }

    #[test]
    fn test_hook_result_empty_json() {
        let result: HookResult = serde_json::from_str("{}").unwrap();
        assert!(!result.is_halt());
        assert!(result.messages.is_none());
        assert!(result.user_input.is_none());
    }

    #[test]
    fn test_hook_result_with_stop() {
        // action=stop 中止当前步骤
        let json = r#"{"action": "stop"}"#;
        let result: HookResult = serde_json::from_str(json).unwrap();
        assert!(result.is_stop());
    }

    #[test]
    fn test_hook_result_with_action_stop() {
        let json = r#"{"action": "stop"}"#;
        let result: HookResult = serde_json::from_str(json).unwrap();
        assert!(result.is_stop());
        assert!(!result.is_skip());
    }

    #[test]
    fn test_hook_result_with_action_skip() {
        let json = r#"{"action": "skip"}"#;
        let result: HookResult = serde_json::from_str(json).unwrap();
        assert!(result.is_skip());
        assert!(!result.is_stop());
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
            name: None,
            command: r#"echo '{"user_input": "hooked"}'"#.to_string(),
            timeout: 5,
            retry: 0,
            on_error: OnError::Skip,
            filter: HookFilter::default(),
            dir_path: None,
        };
        let ctx = HookContext {
            event: HookEvent::PreSendMessage,
            user_input: Some("original".to_string()),
            ..Default::default()
        };
        let result = execute_shell_hook(&hook, &ctx).unwrap();
        assert_eq!(result.user_input.as_deref(), Some("hooked"));
        assert!(!result.is_halt());
    }

    #[test]
    fn test_execute_shell_hook_empty_output() {
        let hook = ShellHook {
            name: None,
            command: "echo ''".to_string(),
            timeout: 5,
            retry: 0,
            on_error: OnError::Skip,
            filter: HookFilter::default(),
            dir_path: None,
        };
        let ctx = HookContext::default();
        let result = execute_shell_hook(&hook, &ctx).unwrap();
        assert!(!result.is_halt());
        assert!(result.user_input.is_none());
    }

    #[test]
    fn test_execute_shell_hook_nonzero_exit() {
        let hook = ShellHook {
            name: None,
            command: "exit 1".to_string(),
            timeout: 5,
            retry: 0,
            on_error: OnError::Skip,
            filter: HookFilter::default(),
            dir_path: None,
        };
        let ctx = HookContext::default();
        let result = execute_shell_hook(&hook, &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_shell_hook_reads_stdin() {
        let hook = ShellHook {
            name: None,
            command: r#"input=$(cat); event=$(echo "$input" | python3 -c "import sys,json; print(json.load(sys.stdin).get('event',''))" 2>/dev/null || echo ""); echo '{"user_input": "got_input"}'"#.to_string(),
            timeout: 5,
            retry: 0,
            on_error: OnError::Skip,
            filter: HookFilter::default(),
            dir_path: None,
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
        let result = execute_hook_with_provider(&kind, &ctx, &None).unwrap();
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
        let result = execute_hook_with_provider(&kind, &ctx, &None).unwrap();
        assert!(!result.is_halt());
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
                r#type: HookType::Bash,
                command: Some(r#"echo '{"user_input": "session_hooked"}'"#.to_string()),
                prompt: None,
                model: None,
                timeout: 5,
                retry: 0,
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
                r#type: HookType::Bash,
                command: Some(r#"echo '{"user_input": "session_overridden"}'"#.to_string()),
                prompt: None,
                model: None,
                timeout: 5,
                retry: 0,
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
                r#type: HookType::Bash,
                command: Some("echo test".to_string()),
                prompt: None,
                model: None,
                timeout: 5,
                retry: 0,
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
                r#type: HookType::Bash,
                command: Some(r#"echo '{"user_input": "first"}'"#.to_string()),
                prompt: None,
                model: None,
                timeout: 5,
                retry: 0,
                on_error: OnError::Skip,
                filter: HookFilter::default(),
            },
        );
        manager.register_session_hook(
            HookEvent::PreSendMessage,
            HookDef {
                r#type: HookType::Bash,
                command: Some(r#"echo '{"user_input": "second"}'"#.to_string()),
                prompt: None,
                model: None,
                timeout: 5,
                retry: 0,
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
    fn test_hook_stop_stops_chain() {
        let mut manager = HookManager::default();

        manager.register_session_hook(
            HookEvent::PreSendMessage,
            HookDef {
                r#type: HookType::Bash,
                command: Some("exit 1".to_string()),
                prompt: None,
                model: None,
                timeout: 5,
                retry: 0,
                on_error: OnError::Stop,
                filter: HookFilter::default(),
            },
        );
        manager.register_session_hook(
            HookEvent::PreSendMessage,
            HookDef {
                r#type: HookType::Bash,
                command: Some(r#"echo '{"user_input": "should_not_reach"}'"#.to_string()),
                prompt: None,
                model: None,
                timeout: 5,
                retry: 0,
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

        assert!(result.is_halt());
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
                r#type: HookType::Bash,
                command: Some("exit 1".to_string()),
                prompt: None,
                model: None,
                timeout: 5,
                retry: 0,
                on_error: OnError::Skip,
                filter: HookFilter::default(),
            },
        );
        manager.register_session_hook(
            HookEvent::PreSendMessage,
            HookDef {
                r#type: HookType::Bash,
                command: Some(r#"echo '{"user_input": "survived"}'"#.to_string()),
                prompt: None,
                model: None,
                timeout: 5,
                retry: 0,
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
        assert!(!result.is_halt());
        assert_eq!(result.user_input.as_deref(), Some("survived"));
    }

    #[test]
    fn test_on_error_stop_stops_chain() {
        // on_error=stop 时，失败的 hook 中止整条链
        let mut manager = HookManager::default();

        manager.register_session_hook(
            HookEvent::PreSendMessage,
            HookDef {
                r#type: HookType::Bash,
                command: Some("exit 1".to_string()),
                prompt: None,
                model: None,
                timeout: 5,
                retry: 0,
                on_error: OnError::Stop,
                filter: HookFilter::default(),
            },
        );
        manager.register_session_hook(
            HookEvent::PreSendMessage,
            HookDef {
                r#type: HookType::Bash,
                command: Some(r#"echo '{"user_input": "should_not_reach"}'"#.to_string()),
                prompt: None,
                model: None,
                timeout: 5,
                retry: 0,
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

        assert!(result.is_halt());
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

        let yaml_stop = r#"command: "echo test"
on_error: stop"#;
        let def: HookDef = serde_yaml::from_str(yaml_stop).unwrap();
        assert_eq!(def.on_error, OnError::Stop);
    }

    #[test]
    fn test_shell_hook_stderr_captured() {
        // 验证 stderr 输出不会导致死锁，且进程正常完成
        let hook = ShellHook {
            name: None,
            command: r#"echo '{"user_input": "ok"}'; echo "debug info" >&2"#.to_string(),
            timeout: 5,
            retry: 0,
            on_error: OnError::Skip,
            filter: HookFilter::default(),
            dir_path: None,
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
            name: None,
            command: r#"echo "something went wrong" >&2; exit 1"#.to_string(),
            timeout: 5,
            retry: 0,
            on_error: OnError::Skip,
            filter: HookFilter::default(),
            dir_path: None,
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
                r#type: HookType::Bash,
                command: Some("echo first".to_string()),
                prompt: None,
                model: None,
                timeout: 5,
                retry: 0,
                on_error: OnError::Skip,
                filter: HookFilter::default(),
            },
        );
        manager.register_session_hook(
            HookEvent::PreSendMessage,
            HookDef {
                r#type: HookType::Bash,
                command: Some("echo second".to_string()),
                prompt: None,
                model: None,
                timeout: 5,
                retry: 0,
                on_error: OnError::Stop,
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
        assert_eq!(hooks[2].on_error, Some(OnError::Stop));
    }

    #[test]
    fn test_switch_model_field_removed() {
        // 验证旧脚本返回 _switch_model 字段时不会报错（serde 静默忽略未知字段）
        let json = r#"{"user_input": "test", "_switch_model": "gpt-4"}"#;
        let result: HookResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.user_input.as_deref(), Some("test"));
    }

    #[test]
    fn test_new_hook_events_roundtrip() {
        // 验证新增的事件能正确序列化/反序列化
        for event in [
            HookEvent::Stop,
            HookEvent::PreMicroCompact,
            HookEvent::PostMicroCompact,
            HookEvent::PreAutoCompact,
            HookEvent::PostAutoCompact,
            HookEvent::PostToolExecutionFailure,
        ] {
            let s = event.as_str();
            let parsed = HookEvent::parse(s).unwrap();
            assert_eq!(event, parsed);
        }
    }

    #[test]
    fn test_hook_result_retry_feedback() {
        // action=stop + retry_feedback
        let json = r#"{"action": "stop", "retry_feedback": "请修正敏感信息"}"#;
        let result: HookResult = serde_json::from_str(json).unwrap();
        assert!(result.is_stop());
        assert_eq!(result.retry_feedback.as_deref(), Some("请修正敏感信息"));
    }

    #[test]
    fn test_hook_result_action_stop_with_retry_feedback() {
        // 新字段 action=stop + retry_feedback
        let json = r#"{"action": "stop", "retry_feedback": "请修正敏感信息"}"#;
        let result: HookResult = serde_json::from_str(json).unwrap();
        assert!(result.is_stop());
        assert_eq!(result.retry_feedback.as_deref(), Some("请修正敏感信息"));
    }

    #[test]
    fn test_hook_result_additional_context() {
        let json = r#"{"additional_context": "必须保留宪法规则"}"#;
        let result: HookResult = serde_json::from_str(json).unwrap();
        assert_eq!(
            result.additional_context.as_deref(),
            Some("必须保留宪法规则")
        );
    }

    #[test]
    fn test_hook_result_system_message() {
        let json = r#"{"system_message": "纠查官已审查"}"#;
        let result: HookResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.system_message.as_deref(), Some("纠查官已审查"));
    }

    #[test]
    fn test_hook_result_tool_error() {
        let json = r#"{"tool_error": "权限不足"}"#;
        let result: HookResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.tool_error.as_deref(), Some("权限不足"));
    }

    #[test]
    fn test_hook_context_new_fields() {
        let ctx = HookContext {
            event: HookEvent::PreAutoCompact,
            tool_error: None,
            ..Default::default()
        };
        let json = serde_json::to_string(&ctx).unwrap();
        assert!(json.contains("pre_auto_compact"));
        // skip_serializing_if 应跳过 None 字段
        assert!(!json.contains("tool_error"));
    }

    #[test]
    fn test_hook_filter_tool_matcher() {
        let filter = HookFilter {
            tool_name: None,
            tool_matcher: Some("Bash|Shell".to_string()),
            model_prefix: None,
        };
        assert!(!filter.is_empty());

        // 匹配 Bash
        let ctx = HookContext {
            event: HookEvent::PreToolExecution,
            tool_name: Some("Bash".to_string()),
            ..Default::default()
        };
        assert!(filter.matches(&ctx));

        // 匹配 Shell
        let ctx = HookContext {
            event: HookEvent::PreToolExecution,
            tool_name: Some("Shell".to_string()),
            ..Default::default()
        };
        assert!(filter.matches(&ctx));

        // 不匹配 Write
        let ctx = HookContext {
            event: HookEvent::PreToolExecution,
            tool_name: Some("Write".to_string()),
            ..Default::default()
        };
        assert!(!filter.matches(&ctx));

        // 上下文中没有 tool_name → 不匹配
        let ctx = HookContext {
            event: HookEvent::PreToolExecution,
            ..Default::default()
        };
        assert!(!filter.matches(&ctx));
    }

    #[test]
    fn test_hook_filter_tool_name_priority_over_matcher() {
        // tool_name 精确匹配优先于 tool_matcher
        let filter = HookFilter {
            tool_name: Some("Bash".to_string()),
            tool_matcher: Some("Write|Edit".to_string()),
            model_prefix: None,
        };
        let ctx = HookContext {
            event: HookEvent::PreToolExecution,
            tool_name: Some("Write".to_string()),
            ..Default::default()
        };
        // tool_name 要求精确匹配 "Bash"，不匹配 "Write"
        assert!(!filter.matches(&ctx));
    }

    #[test]
    fn test_hook_filter_tool_matcher_yaml() {
        let yaml = r#"tool_matcher: "Bash|Shell""#;
        let filter: HookFilter = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(filter.tool_matcher.as_deref(), Some("Bash|Shell"));
        assert!(filter.tool_name.is_none());
    }

    #[test]
    fn test_render_prompt_template() {
        let template = "事件: {{event}}, 输入: {{user_input}}, 工具: {{tool_name}}";
        let ctx = HookContext {
            event: HookEvent::PreSendMessage,
            user_input: Some("hello".to_string()),
            tool_name: Some("Bash".to_string()),
            ..Default::default()
        };
        let rendered = render_prompt_template(template, &ctx);
        assert!(rendered.contains("pre_send_message"));
        assert!(rendered.contains("hello"));
        assert!(rendered.contains("Bash"));
    }

    #[test]
    fn test_render_prompt_template_empty_fields() {
        let template = "输入: {{user_input}}, 输出: {{assistant_output}}";
        let ctx = HookContext::default();
        let rendered = render_prompt_template(template, &ctx);
        assert_eq!(rendered, "输入: , 输出: ");
    }

    #[test]
    fn test_extract_json_from_llm_output() {
        // 纯 JSON
        assert_eq!(
            extract_json_from_llm_output(r#"{"user_input": "test"}"#),
            Some(r#"{"user_input": "test"}"#)
        );

        // JSON 包裹在 markdown 中
        assert_eq!(
            extract_json_from_llm_output("```json\n{\"user_input\": \"test\"}\n```"),
            Some(r#"{"user_input": "test"}"#)
        );

        // JSON 前有文本
        assert_eq!(
            extract_json_from_llm_output("Here is the result: {\"action\": \"stop\"}"),
            Some(r#"{"action": "stop"}"#)
        );

        // 无 JSON
        assert_eq!(extract_json_from_llm_output("no json here"), None);
    }

    #[test]
    fn test_hook_type_yaml_parsing() {
        let yaml_bash = r#"command: "echo hello""#;
        let def: HookDef = serde_yaml::from_str(yaml_bash).unwrap();
        assert_eq!(def.r#type, HookType::Bash);

        let yaml_llm = r#"
type: llm
prompt: "check this""#;
        let def: HookDef = serde_yaml::from_str(yaml_llm).unwrap();
        assert_eq!(def.r#type, HookType::Llm);
        assert_eq!(def.prompt.as_deref(), Some("check this"));
    }

    #[test]
    fn test_hook_def_bash_missing_command() {
        let def = HookDef {
            r#type: HookType::Bash,
            command: None,
            prompt: None,
            model: None,
            timeout: 5,
            retry: 0,
            on_error: OnError::Skip,
            filter: HookFilter::default(),
        };
        assert!(def.into_hook_kind().is_err());
    }

    #[test]
    fn test_hook_def_llm_missing_prompt() {
        let def = HookDef {
            r#type: HookType::Llm,
            command: None,
            prompt: None,
            model: None,
            timeout: 5,
            retry: 0,
            on_error: OnError::Skip,
            filter: HookFilter::default(),
        };
        assert!(def.into_hook_kind().is_err());
    }

    #[test]
    fn test_hook_type_display() {
        assert_eq!(format!("{}", HookType::Bash), "bash");
        assert_eq!(format!("{}", HookType::Llm), "llm");
    }

    #[test]
    fn test_hook_entry_hook_type() {
        let mut manager = HookManager::default();

        // builtin hook → hook_type = "builtin"
        manager.register_builtin(HookEvent::PreSendMessage, "test", |_| None);

        // bash session hook → hook_type = "bash"
        manager.register_session_hook(
            HookEvent::PreSendMessage,
            HookDef {
                r#type: HookType::Bash,
                command: Some("echo test".to_string()),
                prompt: None,
                model: None,
                timeout: 5,
                retry: 0,
                on_error: OnError::Skip,
                filter: HookFilter::default(),
            },
        );

        // llm session hook → hook_type = "llm"
        manager.register_session_hook_kind(
            HookEvent::PreSendMessage,
            HookKind::Llm(LlmHook {
                name: None,
                prompt: "check content".to_string(),
                model: None,
                timeout: 30,
                retry: 1,
                on_error: OnError::Skip,
                filter: HookFilter::default(),
                dir_path: None,
            }),
        );

        let hooks = manager.list_hooks();
        assert_eq!(hooks.len(), 3);
        assert_eq!(hooks[0].hook_type, "builtin");
        assert_eq!(hooks[1].hook_type, "bash");
        assert_eq!(hooks[2].hook_type, "llm");
    }

    #[test]
    fn test_llm_hook_no_provider_returns_err() {
        let hook = LlmHook {
            name: None,
            prompt: "test".to_string(),
            model: None,
            timeout: 5,
            retry: 0,
            on_error: OnError::Skip,
            filter: HookFilter::default(),
            dir_path: None,
        };
        let ctx = HookContext::default();
        let result = execute_llm_hook(&hook, &ctx, &None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("未注入 provider"));
    }
}
