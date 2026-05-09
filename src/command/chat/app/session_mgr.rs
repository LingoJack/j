use super::chat_app::ChatApp;
use crate::command::chat::infra::sandbox::Sandbox;
use crate::command::chat::remote::protocol::WsOutbound;
use crate::command::chat::storage::{
    ChatMessage, MessageRole, PlanStatePersist, SandboxStatePersist, SessionEvent, SessionPaths,
    SubAgentSnapshotPersist, TeammateSnapshotPersist, append_event_to_path, append_session_event,
    generate_session_id, load_display_session, load_hooks_state, load_loaded_deferred_state,
    load_plan_state, load_sandbox_state, load_session_meta_file, load_skills_state,
    load_tasks_state, load_teammates_state, load_todos_state, save_hooks_state,
    save_loaded_deferred_state, save_plan_state, save_sandbox_state, save_session_meta_file,
    save_skills_state, save_subagents_state, save_tasks_state, save_teammates_state,
    save_todos_state,
};
use crate::command::chat::teammate::TeammateStatusPersist;
use crate::command::chat::tools::derived_shared::SubAgentStatus;
use crate::util::safe_lock;
use std::sync::atomic::Ordering;

impl ChatApp {
    /// 将 context_messages 中尚未持久化的新消息追加到 JSONL（按 turn 原子提交）。
    ///
    /// 从 `persisted_message_count` 开始扫描未持久化区间，只 append 到最后一个
    /// "安全切点"——该位置之前的每一条 `assistant(tool_calls)` 都已配齐对应数量的
    /// `tool_result`。半完成的 tool_call 尾部留到下次 persist，避免 jsonl 里出现
    /// 孤立的 tool_call 或 tool_result。
    pub(super) fn persist_new_messages(&mut self) {
        let start = self.persisted_message_count;
        let context = safe_lock(&self.context_messages, "persist_new_messages");
        if start >= context.len() {
            return;
        }

        let mut pending: i64 = 0;
        let mut last_safe = start;
        for (i, msg) in context[start..].iter().enumerate() {
            let abs = start + i;
            match msg.role {
                MessageRole::Assistant => {
                    if let Some(ref tcs) = msg.tool_calls {
                        pending += tcs.len() as i64;
                    } else if pending == 0 {
                        last_safe = abs + 1;
                    }
                }
                MessageRole::Tool => {
                    pending -= 1;
                    if pending <= 0 {
                        // 防御性 clamp：历史遗留的 orphan tool result 可能让 pending 变负
                        pending = 0;
                        last_safe = abs + 1;
                    }
                }
                MessageRole::User | MessageRole::System => {
                    if pending == 0 {
                        last_safe = abs + 1;
                    }
                }
            }
        }

        if last_safe > start {
            let msgs: Vec<_> = context[start..last_safe].to_vec();
            // 释放锁再写磁盘
            drop(context);
            for msg in msgs {
                append_session_event(&self.session_id, &SessionEvent::msg(msg));
            }
            self.persisted_message_count = last_safe;
        }
    }

    /// 将 display_messages 中尚未持久化的新消息追加到 display.jsonl。
    ///
    /// 与 `persist_new_messages()` 不同，display 消息按推入顺序即完整配对，
    /// 无需安全切点算法，直接全量 append。
    pub(super) fn persist_new_display_messages(&mut self) {
        let display = safe_lock(&self.display_messages, "persist_display");
        let start = self.persisted_display_count;
        if start >= display.len() {
            return;
        }
        let path = SessionPaths::new(&self.session_id).display();
        for msg in &display[start..] {
            append_event_to_path(&path, &SessionEvent::msg(msg.clone()));
        }
        self.persisted_display_count = display.len();
    }

    /// 保存当前 session 的所有状态到磁盘
    pub fn save_session_state(&self) {
        let sid = &self.session_id;

        // 1. Teammates
        if let Ok(mgr) = self.teammate_manager.lock() {
            let snapshots = Self::build_teammates_snapshot(&mgr);
            save_teammates_state(sid, &snapshots);
        }

        // 2. SubAgents
        let subagent_snapshots: Vec<SubAgentSnapshotPersist> = self
            .sub_agent_tracker
            .display_snapshots()
            .into_iter()
            .map(|s| {
                let status_str = match s.status {
                    SubAgentStatus::Initializing => "initializing",
                    SubAgentStatus::Thinking => "thinking",
                    SubAgentStatus::Working => "working",
                    SubAgentStatus::Retrying { .. } => "retrying",
                    SubAgentStatus::Completed => "completed",
                    SubAgentStatus::Cancelled => "cancelled",
                    SubAgentStatus::Error(_) => "error",
                };
                SubAgentSnapshotPersist {
                    id: s.id.clone(),
                    description: s.description,
                    mode: s.mode.to_string(),
                    status: status_str.to_string(),
                    current_tool: s.current_tool,
                    tool_calls_count: s.tool_calls_count,
                    current_round: s.current_round,
                    started_at_epoch: 0,
                    transcript_file: format!("subagents/{}/transcript.jsonl", s.id),
                }
            })
            .collect();
        save_subagents_state(sid, &subagent_snapshots);

        // 3. Tasks
        save_tasks_state(sid, &self.task_manager.list_tasks());

        // 4. Todos
        save_todos_state(sid, &self.todo_manager.list_todos());

        // 5. Plan
        {
            let plan_state = &self.tool_registry.plan_mode_state;
            let (active, plan_file_path) = plan_state.get_state();
            let plan_content = plan_file_path
                .as_ref()
                .and_then(|p| std::fs::read_to_string(p).ok());
            save_plan_state(
                sid,
                &PlanStatePersist {
                    active,
                    plan_file_path,
                    plan_content,
                },
            );
        }

        // 6. InvokedSkills
        if let Ok(skills) = self.invoked_skills.lock() {
            save_skills_state(sid, &skills.clone());
        }

        // 7. Session Hooks
        if let Ok(mgr) = self.hook_manager.lock() {
            let snapshot = mgr.session_hooks_snapshot();
            save_hooks_state(sid, &snapshot);
        }

        // 8. Sandbox
        save_sandbox_state(
            sid,
            &SandboxStatePersist {
                extra_safe_dirs: self.sandbox.extra_safe_dirs(),
            },
        );

        // 9. LoadTool 已加载的 deferred 工具
        if let Ok(loaded) = self.session_loaded_deferred.lock() {
            save_loaded_deferred_state(sid, &loaded.clone());
        }

        // 10. auto_approve → session.json
        if let Some(mut meta) = load_session_meta_file(sid)
            && meta.auto_approve != self.ui.auto_approve
        {
            meta.auto_approve = self.ui.auto_approve;
            save_session_meta_file(&meta);
        }
    }

    /// 构建 Teammates 持久化快照（活跃 + 已恢复但未重新启动的 teammate）
    fn build_teammates_snapshot(
        mgr: &crate::command::chat::teammate::TeammateManager,
    ) -> Vec<TeammateSnapshotPersist> {
        let mut snapshots: Vec<TeammateSnapshotPersist> = Vec::new();
        let recovered = mgr.recovered_teammates_snapshot();

        for (name, handle) in &mgr.teammates {
            let status = handle
                .status
                .lock()
                .map(|s| s.clone().into())
                .unwrap_or(TeammateStatusPersist::Cancelled);
            let pending = handle
                .broadcast_inbox
                .lock()
                .map(|m| m.clone())
                .unwrap_or_default();
            let current_tool = handle.current_tool.lock().ok().and_then(|t| t.clone());
            let final_status = if handle.running()
                && !matches!(
                    status,
                    TeammateStatusPersist::Completed
                        | TeammateStatusPersist::Cancelled
                        | TeammateStatusPersist::Error(_)
                ) {
                TeammateStatusPersist::Cancelled
            } else {
                status
            };
            let (prompt, worktree, worktree_branch, inherit_permissions) = recovered
                .get(name)
                .map(|r| {
                    (
                        r.prompt.clone(),
                        r.worktree,
                        r.worktree_branch.clone(),
                        r.inherit_permissions,
                    )
                })
                .unwrap_or_default();
            snapshots.push(TeammateSnapshotPersist {
                name: name.clone(),
                role: handle.role.clone(),
                prompt,
                worktree,
                worktree_branch,
                inherit_permissions,
                status: final_status,
                broadcast_inbox: pending,
                tool_calls_count: handle.tool_calls_count.load(Ordering::Relaxed),
                current_tool,
                work_done: handle.work_done.load(Ordering::Relaxed),
            });
        }

        // 追加已恢复但不在当前活跃列表中的 teammate
        for (name, r) in recovered {
            if !snapshots.iter().any(|s| s.name == name) {
                snapshots.push(r);
            }
        }

        snapshots
    }

    /// 从磁盘恢复当前 session_id 的所有状态
    pub fn restore_session_state(&mut self) {
        let sid = self.session_id.clone();
        let sid = sid.as_str();

        // 0. auto_approve ← session.json
        if let Some(meta) = load_session_meta_file(sid) {
            self.ui.auto_approve = meta.auto_approve;
        }

        // 1. InvokedSkills
        if let Some(skills) = load_skills_state(sid)
            && let Ok(mut map) = self.invoked_skills.lock()
        {
            *map = skills;
        }

        // 2. Tasks
        if let Some(tasks) = load_tasks_state(sid) {
            self.task_manager.replace_all(tasks);
        }

        // 3. Todos
        if let Some(todos) = load_todos_state(sid) {
            self.todo_manager.replace_all(todos);
        }

        // 4. Plan
        if let Some(plan) = load_plan_state(sid) {
            let plan_state = &self.tool_registry.plan_mode_state;
            if plan.active
                && let Some(ref path) = plan.plan_file_path
            {
                if !std::path::Path::new(path).exists()
                    && let Some(ref content) = plan.plan_content
                {
                    if let Some(parent) = std::path::Path::new(path).parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let _ = std::fs::write(path, content);
                }
                let _ = plan_state.enter(path);
            }
        }

        // 5. Session Hooks
        if let Some(hooks) = load_hooks_state(sid)
            && let Ok(mut mgr) = self.hook_manager.lock()
        {
            mgr.restore_session_hooks(&hooks);
        }

        // 6. Sandbox
        if let Some(sandbox) = load_sandbox_state(sid) {
            self.sandbox
                .restore_extra_safe_dirs(sandbox.extra_safe_dirs);
        }

        // 7. Teammates
        if let Some(teammates) = load_teammates_state(sid)
            && let Ok(mut mgr) = self.teammate_manager.lock()
        {
            mgr.set_recovered_teammates(teammates);
        }

        // 8. LoadTool 已加载的 deferred 工具
        if let Some(loaded) = load_loaded_deferred_state(sid) {
            // 从 deferred_tools 移除（运行时生效）+ 填充 session_loaded_deferred
            if let Ok(mut deferred) = self.deferred_tools.lock() {
                for name in &loaded {
                    if let Some(pos) = deferred.iter().position(|d| d == name) {
                        deferred.remove(pos);
                    }
                }
            }
            if let Ok(mut session_loaded) = self.session_loaded_deferred.lock() {
                *session_loaded = loaded;
            }
        }

        // Teammates 状态已通过 set_recovered_teammates 恢复，无需额外处理 transcript。
        // 主 transcript（transcript.jsonl）已包含所有来源的消息（Main/Teammate/SubAgent），
        // 它们在实时运行时通过 push_both/各自推送 → display_messages（UI）/ context_messages（LLM）写入，
        // 顺序正确。无需从独立 transcript 重新合成消息。
    }

    /// 清除运行时状态（session 切换前调用）
    pub fn clear_runtime_state(&mut self) {
        if let Ok(mut mgr) = self.teammate_manager.lock() {
            mgr.stop_all();
            // 给 teammate 线程短暂时间响应取消信号
            std::thread::sleep(std::time::Duration::from_millis(50));
            mgr.cleanup_finished();
            // 强制清除所有未及时结束的 teammates（detach 线程，不阻塞）
            mgr.clear_all();
            mgr.clear_recovered_teammates();
        }

        self.permission_queue.deny_all();
        self.plan_approval_queue.deny_all();

        self.sub_agent_tracker.clear_all();

        self.task_manager.replace_all(Vec::new());
        self.todo_manager.replace_all(Vec::new());

        self.tool_registry.plan_mode_state.exit();

        if let Ok(mut skills) = self.invoked_skills.lock() {
            skills.clear();
        }

        if let Ok(mut mgr) = self.hook_manager.lock() {
            mgr.clear_session_hooks();
        }

        self.sandbox = Sandbox::new();
    }

    /// 清空对话（创建新会话）
    pub fn clear_session(&mut self) {
        self.persist_new_messages();
        self.persist_new_display_messages();
        self.save_session_state();
        self.clear_runtime_state();
        let new_id = generate_session_id();
        self.session_id = new_id.clone();
        if let Ok(mut s) = self.shared_session_id.lock() {
            *s = new_id.clone();
        }
        self.clear_channels();
        self.persisted_message_count = 0;
        self.persisted_display_count = 0;
        self.ui.scroll_offset = 0;
        self.ui.msg_lines_cache = None;
        if let Ok(mut ct) = self.context_tokens.lock() {
            *ct = 0;
        }
        let sync = self.build_sync_outbound();
        self.broadcast_ws(sync);
        self.broadcast_ws(WsOutbound::SessionSwitched { session_id: new_id });
        self.show_toast("已创建新对话", false);
    }

    /// 从加载的 context 消息重建双通道（会话恢复/切换/归档还原后调用）。
    ///
    /// 优先从 display.jsonl 读取 display 消息（独立格式，支持 tool_calls 结构体）；
    /// 旧 session 无 display.jsonl 时，从 context 格式反推（strip XML 包裹）。
    pub fn rebuild_channels_from_loaded(&mut self, messages: Vec<ChatMessage>) {
        // 尝试从 display.jsonl 加载
        let display = load_display_session(&self.session_id);

        let display = if display.is_empty() && !messages.is_empty() {
            // fallback：旧 session，从 context 格式反推 display
            let mut display: Vec<ChatMessage> = Vec::with_capacity(messages.len());
            for msg in &messages {
                if let Some(ref sender) = msg.sender_name {
                    let clean_content = strip_agent_xml_tag(sender, &msg.content);
                    let mut display_msg = msg.clone();
                    display_msg.content = clean_content;
                    display.push(display_msg);
                } else {
                    display.push(msg.clone());
                }
            }
            display
        } else {
            display
        };

        // context 通道：直接 move messages（零 clone）
        {
            let mut cm = safe_lock(&self.context_messages, "rebuild_channels::context");
            *cm = messages;
        }
        {
            let mut dm = safe_lock(&self.display_messages, "rebuild_channels::display");
            *dm = display;
        }

        let ctx_len = safe_lock(&self.context_messages, "rebuild_channels::len").len();
        let disp_len = safe_lock(&self.display_messages, "rebuild_channels::disp_len").len();
        self.persisted_message_count = ctx_len;
        self.display_read_offset = disp_len;
        self.context_read_offset = ctx_len;
        self.persisted_display_count = disp_len;
        self.ui.msg_lines_cache = None;
    }

    /// 清空双通道 + offset（新建会话时调用）
    pub fn clear_channels(&mut self) {
        safe_lock(&self.display_messages, "clear_channels::display").clear();
        safe_lock(&self.context_messages, "clear_channels::context").clear();
        self.display_read_offset = 0;
        self.context_read_offset = 0;
        self.ui.msg_lines_cache = None;
    }

    /// 向双通道同时推送消息（display 用于 UI 渲染，context 用于 LLM + 持久化）。
    pub fn push_both_channels(&self, msg: ChatMessage) {
        safe_lock(&self.display_messages, "push_both_channels::display").push(msg.clone());
        safe_lock(&self.context_messages, "push_both_channels::context").push(msg);
    }
}

/// 去除 `<Sender>content</Sender>` XML 包裹，返回干净文本。
/// 如果 content 不以 `<Sender>` 开头，原样返回。
fn strip_agent_xml_tag(sender: &str, content: &str) -> String {
    let open_tag = format!("<{}>", sender);
    let close_tag = format!("</{}>", sender);
    if content.starts_with(&open_tag) && content.ends_with(&close_tag) {
        let inner = &content[open_tag.len()..content.len() - close_tag.len()];
        inner.to_string()
    } else {
        content.to_string()
    }
}
