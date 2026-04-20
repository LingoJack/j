use super::chat_app::ChatApp;
use crate::command::chat::infra::sandbox::Sandbox;
use crate::command::chat::remote::protocol::WsOutbound;
use crate::command::chat::storage::{
    ChatMessage, MessageRole, PlanStatePersist, SandboxStatePersist, SessionEvent, SessionPaths,
    SubAgentSnapshotPersist, TeammateSnapshotPersist, append_session_event, generate_session_id,
    load_hooks_state, load_plan_state, load_sandbox_state, load_skills_state, load_tasks_state,
    load_teammates_state, load_todos_state, read_transcript_with_timestamps, sanitize_filename,
    save_hooks_state, save_plan_state, save_sandbox_state, save_skills_state, save_subagents_state,
    save_tasks_state, save_teammates_state, save_todos_state,
};
use crate::command::chat::teammate::TeammateStatusPersist;
use crate::command::chat::tools::derived_shared::SubAgentStatus;
use std::sync::atomic::Ordering;

impl ChatApp {
    /// 将 session.messages 中尚未持久化的新消息追加到 JSONL
    pub(super) fn persist_new_messages(&mut self) {
        let start = self.persisted_message_count;
        let msgs: Vec<_> = self.state.session.messages[start..].to_vec();
        for msg in msgs {
            append_session_event(&self.session_id, &SessionEvent::msg(msg));
        }
        self.persisted_message_count = self.state.session.messages.len();
    }

    /// 保存当前 session 的所有状态到磁盘
    pub fn save_session_state(&self) {
        let sid = &self.session_id;

        // 1. Teammates
        if let Ok(mgr) = self.teammate_manager.lock() {
            let mut final_snapshots: Vec<TeammateSnapshotPersist> = Vec::new();
            let recovered = mgr.recovered_teammates_snapshot();

            for (name, handle) in &mgr.teammates {
                let status = handle
                    .status
                    .lock()
                    .map(|s| s.clone().into())
                    .unwrap_or(TeammateStatusPersist::Cancelled);
                let pending = handle
                    .pending_user_messages
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
                final_snapshots.push(TeammateSnapshotPersist {
                    name: name.clone(),
                    role: handle.role.clone(),
                    prompt,
                    worktree,
                    worktree_branch,
                    inherit_permissions,
                    status: final_status,
                    pending_user_messages: pending,
                    tool_calls_count: handle.tool_calls_count.load(Ordering::Relaxed),
                    current_tool,
                    work_done: handle.work_done.load(Ordering::Relaxed),
                });
            }

            for (name, r) in recovered {
                if !final_snapshots.iter().any(|s| s.name == name) {
                    final_snapshots.push(r);
                }
            }

            save_teammates_state(sid, &final_snapshots);
        }

        // 2. SubAgents
        let subagent_snapshots: Vec<SubAgentSnapshotPersist> = self
            .sub_agent_tracker
            .display_snapshots()
            .into_iter()
            .map(|s| {
                let status_str = match s.status {
                    SubAgentStatus::Initializing => "initializing",
                    SubAgentStatus::Working => "working",
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
    }

    /// 从磁盘恢复当前 session_id 的所有状态
    pub fn restore_session_state(&mut self) {
        let sid = self.session_id.clone();
        let sid = sid.as_str();

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
        let teammate_names: Vec<String> = if let Some(teammates) = load_teammates_state(sid) {
            let names: Vec<String> = teammates.iter().map(|t| t.name.clone()).collect();
            if let Ok(mut mgr) = self.teammate_manager.lock() {
                mgr.set_recovered_teammates(teammates);
            }
            names
        } else {
            Vec::new()
        };

        self.restore_teammate_transcripts(sid, &teammate_names);
    }

    /// 从 teammate 独立 JSONL 中恢复 `<Name>` 显示条目。
    fn restore_teammate_transcripts(&mut self, sid: &str, teammate_names: &[String]) {
        if teammate_names.is_empty() {
            return;
        }
        for name in teammate_names {
            let prefix_marker = format!("<{}>", name);
            let path = SessionPaths::new(sid).teammate_transcript(&sanitize_filename(name));
            if !path.exists() {
                continue;
            }
            let transcript = read_transcript_with_timestamps(&path);

            let mut synthesized: Vec<String> = Vec::new();
            for (msg, _ts) in &transcript {
                if msg.role != MessageRole::Assistant {
                    continue;
                }
                if !msg.content.is_empty() {
                    synthesized.push(format!("<{}> {}", name, msg.content));
                }
                if let Some(tcs) = &msg.tool_calls {
                    for tc in tcs {
                        if tc.name != "SendMessage" {
                            synthesized.push(format!("<{}> [调用工具 {}]", name, tc.name));
                        }
                    }
                }
            }

            if synthesized.is_empty() {
                continue;
            }

            let mut new_messages: Vec<ChatMessage> = Vec::new();
            let mut synth_iter = synthesized.iter().peekable();
            for msg in &self.state.session.messages {
                if msg.role == MessageRole::Assistant && msg.content.starts_with(&prefix_marker) {
                    if synth_iter.peek().is_some()
                        && !new_messages.iter().any(|m| {
                            m.role == MessageRole::Assistant
                                && m.content.starts_with(&prefix_marker)
                        })
                    {
                        for s in &synthesized {
                            new_messages.push(ChatMessage::text(MessageRole::Assistant, s.clone()));
                        }
                    }
                } else {
                    new_messages.push(msg.clone());
                }
            }
            if !new_messages
                .iter()
                .any(|m| m.role == MessageRole::Assistant && m.content.starts_with(&prefix_marker))
            {
                for s in &synthesized {
                    new_messages.push(ChatMessage::text(MessageRole::Assistant, s.clone()));
                }
            }

            self.state.session.messages = new_messages;
            self.ui.msg_lines_cache = None;
        }
    }

    /// 清除运行时状态（session 切换前调用）
    pub fn clear_runtime_state(&mut self) {
        if let Ok(mut mgr) = self.teammate_manager.lock() {
            mgr.stop_all();
            mgr.cleanup_finished();
            mgr.clear_recovered_teammates();
        }

        self.permission_queue.deny_all();
        self.plan_approval_queue.deny_all();

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
        self.save_session_state();
        self.clear_runtime_state();
        let new_id = generate_session_id();
        self.session_id = new_id.clone();
        if let Ok(mut s) = self.shared_session_id.lock() {
            *s = new_id.clone();
        }
        self.state.session.messages.clear();
        self.persisted_message_count = 0;
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
}
