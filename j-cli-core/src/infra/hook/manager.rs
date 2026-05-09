use crate::infra::hook::definition::*;
use crate::infra::hook::executor::execute_hook_with_provider;
use crate::infra::hook::types::*;
use crate::util::log::{write_error_log, write_info_log};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// ========== HookMetrics / HookEntry ==========

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

// ========== HookManager ==========

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
    pub(crate) metrics: Mutex<HashMap<String, HookMetrics>>,
    /// 当前活跃的 LLM provider（LLM hook 执行时使用）
    pub(crate) provider: Option<Arc<Mutex<crate::storage::ModelProvider>>>,
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
    pub fn session_hooks_snapshot(&self) -> Vec<crate::storage::SessionHookPersist> {
        let mut result = Vec::new();
        for (event, hooks) in &self.session_hooks {
            for kind in hooks {
                match kind {
                    HookKind::Shell(sh) => {
                        result.push(crate::storage::SessionHookPersist {
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
                        result.push(crate::storage::SessionHookPersist {
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
    pub fn restore_session_hooks(
        &mut self,
        hooks: &[crate::storage::SessionHookPersist],
    ) {
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
    pub fn set_provider(
        &mut self,
        provider: Arc<Mutex<crate::storage::ModelProvider>>,
    ) {
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
        let all_hooks = collect_hooks_for_event(self, event);
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
                        merge_hook_result_into(&hook_result, &mut context, &mut final_result);
                        had_modification = true;

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
                    if let Some(action) = handle_hook_error(hook_ref.kind, &label) {
                        return Some(action);
                    }
                }
                Some(HookOutcome::Err(e)) => {
                    write_error_log(
                        "HookManager::execute",
                        &format!("Hook 最终失败 ({}): {}", label, e),
                    );
                    if let Some(action) = handle_hook_error(hook_ref.kind, &label) {
                        return Some(action);
                    }
                }
                None => {
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

// ========== 辅助函数 ==========

/// Hook 引用（附带来源标记）
struct HookRef<'a> {
    kind: &'a HookKind,
    source: &'static str,
    session_index: Option<usize>,
}

/// 收集指定事件的所有 hook（内置→用户→项目→session）
fn collect_hooks_for_event(manager: &HookManager, event: HookEvent) -> Vec<HookRef<'_>> {
    let mut all_hooks: Vec<HookRef<'_>> = Vec::new();

    if let Some(hooks) = manager.builtin_hooks.get(&event) {
        for h in hooks.iter() {
            all_hooks.push(HookRef {
                kind: h,
                source: HOOK_SOURCE_BUILTIN,
                session_index: None,
            });
        }
    }
    if let Some(hooks) = manager.user_hooks.get(&event) {
        for h in hooks.iter() {
            all_hooks.push(HookRef {
                kind: h,
                source: HOOK_SOURCE_USER,
                session_index: None,
            });
        }
    }
    if let Some(hooks) = manager.project_hooks.get(&event) {
        for h in hooks.iter() {
            all_hooks.push(HookRef {
                kind: h,
                source: HOOK_SOURCE_PROJECT,
                session_index: None,
            });
        }
    }
    if let Some(hooks) = manager.session_hooks.get(&event) {
        for (idx, h) in hooks.iter().enumerate() {
            all_hooks.push(HookRef {
                kind: h,
                source: HOOK_SOURCE_SESSION,
                session_index: Some(idx),
            });
        }
    }

    all_hooks
}

/// 将 hook 执行结果合并到 context（链式传递）和 final_result（最终返回）
fn merge_hook_result_into(
    hook_result: &HookResult,
    context: &mut HookContext,
    final_result: &mut HookResult,
) {
    if let Some(ref msgs) = hook_result.messages {
        context.messages = Some(msgs.clone());
        final_result.messages = context.messages.clone();
    }
    if let Some(ref sp) = hook_result.system_prompt {
        context.system_prompt = Some(sp.clone());
        final_result.system_prompt = context.system_prompt.clone();
    }
    if let Some(ref ui) = hook_result.user_input {
        context.user_input = Some(ui.clone());
        final_result.user_input = context.user_input.clone();
    }
    if let Some(ref ao) = hook_result.assistant_output {
        context.assistant_output = Some(ao.clone());
        final_result.assistant_output = context.assistant_output.clone();
    }
    if let Some(ref ta) = hook_result.tool_arguments {
        context.tool_arguments = Some(ta.clone());
        final_result.tool_arguments = context.tool_arguments.clone();
    }
    if let Some(ref tr) = hook_result.tool_result {
        context.tool_result = Some(tr.clone());
        final_result.tool_result = context.tool_result.clone();
    }
    if let Some(ref inject) = hook_result.inject_messages {
        let existing = final_result.inject_messages.get_or_insert_with(Vec::new);
        existing.extend(inject.clone());
    }
    if let Some(ref rf) = hook_result.retry_feedback {
        final_result.retry_feedback = Some(rf.clone());
    }
    if let Some(ref ac) = hook_result.additional_context {
        final_result.additional_context = Some(ac.clone());
    }
    if let Some(ref sm) = hook_result.system_message {
        final_result.system_message = Some(sm.clone());
    }
    if let Some(ref te) = hook_result.tool_error {
        final_result.tool_error = Some(te.clone());
    }
}

/// 处理 hook 执行失败：按 on_error 策略返回 Stop 或 None（Skip，继续执行下一个）
/// 返回 `Some(HookResult)` 表示应中止链，返回 `None` 表示跳过继续
fn handle_hook_error(kind: &HookKind, _label: &str) -> Option<HookResult> {
    match hook_on_error_strategy(kind) {
        OnError::Stop => Some(HookResult {
            action: Some(HookAction::Stop),
            ..Default::default()
        }),
        OnError::Skip => None,
    }
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

/// 获取 hook 的名称（目录布局下的目录名）
pub(crate) fn hook_name(kind: &HookKind) -> Option<&str> {
    match kind {
        HookKind::Shell(shell) => shell.name.as_deref(),
        HookKind::Llm(llm) => llm.name.as_deref(),
        HookKind::Builtin(builtin) => Some(&builtin.name),
    }
}

/// 获取 hook 的显示标签（Shell 用命令，LLM 用 prompt 摘要，Builtin 用名称）
pub(crate) fn hook_label(kind: &HookKind) -> String {
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
            let prompt_preview = if first_line.len()
                > crate::constants::HOOK_PROMPT_PREVIEW_MAX_LEN
            {
                format!(
                    "{}...",
                    &first_line[..crate::constants::HOOK_PROMPT_PREVIEW_MAX_LEN]
                )
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
pub(crate) fn hook_type_str(kind: &HookKind) -> &'static str {
    match kind {
        HookKind::Shell(_) => "bash",
        HookKind::Llm(_) => "llm",
        HookKind::Builtin(_) => "builtin",
    }
}

/// 获取 hook 的超时秒数
pub(crate) fn hook_timeout(kind: &HookKind) -> Option<u64> {
    match kind {
        HookKind::Shell(shell) => Some(shell.timeout),
        HookKind::Llm(llm) => Some(llm.timeout),
        HookKind::Builtin(_) => None,
    }
}

/// 获取 hook 的重试次数
pub(crate) fn hook_retry_count(kind: &HookKind) -> u32 {
    match kind {
        HookKind::Shell(shell) => shell.retry,
        HookKind::Llm(llm) => llm.retry,
        HookKind::Builtin(_) => 0,
    }
}

/// 获取 hook 的失败策略（用于 list 展示）
pub(crate) fn hook_on_error(kind: &HookKind) -> Option<OnError> {
    match kind {
        HookKind::Shell(shell) => Some(shell.on_error),
        HookKind::Llm(llm) => Some(llm.on_error),
        HookKind::Builtin(_) => None,
    }
}

/// 获取 hook 执行失败时的实际策略（Shell/LLM 按配置，Builtin 一律 Abort）
pub(crate) fn hook_on_error_strategy(kind: &HookKind) -> OnError {
    match kind {
        HookKind::Shell(shell) => shell.on_error,
        HookKind::Llm(llm) => llm.on_error,
        HookKind::Builtin(_) => OnError::Stop,
    }
}

/// 获取 hook 的条件过滤器
pub(crate) fn hook_filter(kind: &HookKind) -> Option<&HookFilter> {
    match kind {
        HookKind::Shell(shell) if !shell.filter.is_empty() => Some(&shell.filter),
        HookKind::Llm(llm) if !llm.filter.is_empty() => Some(&llm.filter),
        HookKind::Shell(_) | HookKind::Llm(_) | HookKind::Builtin(_) => None,
    }
}

/// 检查 hook 是否应在当前 context 下执行（无 filter 或 filter 匹配时返回 true）
pub(crate) fn hook_should_execute(kind: &HookKind, context: &HookContext) -> bool {
    match kind {
        HookKind::Shell(shell) => shell.filter.matches(context),
        HookKind::Llm(llm) => llm.filter.matches(context),
        HookKind::Builtin(_) => true,
    }
}
