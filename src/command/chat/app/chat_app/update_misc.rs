use super::ChatApp;
use crate::command::chat::app::action::CursorDirection;
use crate::command::chat::app::types::ToolExecStatus;
use crate::command::chat::app::ui_state::ChatMode;
use crate::command::chat::constants::{FINE_SCROLL_LINES, INPUT_BUFFER_MAX_LEN, PAGE_SCROLL_LINES};
use crate::command::chat::remote::protocol::{ToolConfirmInfo, WsOutbound};
use crate::command::chat::storage::{MessageRole, save_agent_config};
use crate::util::safe_lock;

impl ChatApp {
    // ========== Chat 输入和文本编辑 ==========

    pub(super) fn update_insert_char(&mut self, ch: char) {
        if self.ui.input_text().len() < INPUT_BUFFER_MAX_LEN {
            self.ui.input_buffer.insert_char(ch);
        }
    }

    pub(super) fn update_move_cursor(&mut self, dir: CursorDirection) {
        match dir {
            CursorDirection::Up => {
                self.ui.input_buffer.move_cursor_up();
            }
            CursorDirection::Down => {
                self.ui.input_buffer.move_cursor_down();
            }
        }
    }

    // ========== 弹窗交互 ==========

    pub(super) fn update_at_popup_activate(&mut self) {
        self.ui.at_popup_active = true;
        self.ui.at_popup_filter.clear();
        self.ui.at_popup_selected = 0;
    }

    pub(super) fn update_at_popup_filter(&mut self, text: String) {
        self.ui.at_popup_filter = text;
        self.ui.at_popup_selected = 0;
    }

    pub(super) fn update_at_popup_navigate(&mut self, dir: CursorDirection) {
        match dir {
            CursorDirection::Up => {
                if self.ui.at_popup_selected > 0 {
                    self.ui.at_popup_selected -= 1;
                }
            }
            CursorDirection::Down => {
                self.ui.at_popup_selected += 1;
            }
        }
    }

    pub(super) fn update_file_popup_activate(&mut self) {
        self.ui.file_popup_active = true;
        self.ui.file_popup_filter.clear();
        self.ui.file_popup_selected = 0;
    }

    pub(super) fn update_file_popup_filter(&mut self, text: String) {
        self.ui.file_popup_filter = text;
        self.ui.file_popup_selected = 0;
    }

    pub(super) fn update_skill_popup_activate(&mut self) {
        self.ui.skill_popup_active = true;
        self.ui.skill_popup_filter.clear();
        self.ui.skill_popup_selected = 0;
    }

    pub(super) fn update_skill_popup_filter(&mut self, text: String) {
        self.ui.skill_popup_filter = text;
        self.ui.skill_popup_selected = 0;
    }

    pub(super) fn update_skill_popup_navigate(&mut self, dir: CursorDirection) {
        match dir {
            CursorDirection::Up => {
                if self.ui.skill_popup_selected > 0 {
                    self.ui.skill_popup_selected -= 1;
                }
            }
            CursorDirection::Down => {
                self.ui.skill_popup_selected += 1;
            }
        }
    }

    // ========== 流式生命周期 ==========

    pub(super) fn update_stream_chunk(&mut self) {
        self.state.retry_hint = None;
        if self.ui.auto_scroll {
            self.ui.scroll_offset = usize::MAX;
        }
        if self.ws_bridge.is_some() {
            let content = safe_lock(&self.state.streaming_content, "ws_stream_chunk").clone();
            self.broadcast_ws(WsOutbound::StreamChunk { content });
        }
    }

    pub(super) fn update_stream_done(&mut self) {
        self.state.retry_hint = None;
        if self.ws_bridge.is_some() {
            if let Some(last_msg) =
                crate::util::safe_lock(&self.display_messages, "StreamDone::ws_broadcast").last()
                && last_msg.role == MessageRole::Assistant
            {
                self.broadcast_ws(WsOutbound::Message {
                    role: MessageRole::Assistant.to_string(),
                    content: last_msg.content.clone(),
                });
            }
            self.broadcast_ws(WsOutbound::Status {
                state: "idle".to_string(),
            });
        }
        self.finish_loading(false, false, false);
    }

    pub(super) fn update_stream_error(&mut self, e: &crate::command::chat::error::ChatError) {
        self.state.retry_hint = None;
        let msg = e.display_message();
        self.broadcast_ws(WsOutbound::Error {
            message: format!("请求失败: {}", msg),
        });
        self.broadcast_ws(WsOutbound::Status {
            state: "idle".to_string(),
        });
        self.show_toast(format!("请求失败: {}", msg), true);
        let preserve_streamed_reply = matches!(
            e,
            crate::command::chat::error::ChatError::ToolCallProtocolMismatch(_)
        );
        self.finish_loading(true, false, preserve_streamed_reply);
    }

    pub(super) fn update_stream_cancelled(&mut self) {
        self.state.retry_hint = None;
        self.broadcast_ws(WsOutbound::Status {
            state: "idle".to_string(),
        });
        self.finish_loading(false, true, false);
    }

    pub(super) fn update_stream_retrying(
        &mut self,
        attempt: u32,
        max_attempts: u32,
        delay_ms: u64,
        error: String,
    ) {
        let delay_s = delay_ms.div_ceil(1000);
        self.state.retry_hint = Some(format!(
            "⟳ 重试 {}/{} · {}s · {}",
            attempt, max_attempts, delay_s, error
        ));
    }

    pub(super) fn update_stream_compacting(&mut self) {
        self.state.retry_hint = Some("📦 压缩上下文中...".to_string());
    }

    pub(super) fn update_stream_compacted(&mut self) {
        self.state.retry_hint = None;
        {
            let display =
                crate::util::safe_lock(&self.display_messages, "StreamCompacted::sync_display");
            self.display_read_offset = display.len();
            let shared =
                crate::util::safe_lock(&self.context_messages, "StreamCompacted::sync_ctx");
            self.context_read_offset = shared.len();
        }
        self.ui.msg_lines_cache = None;
    }

    // ========== 模式切换和导航 ==========

    pub(super) fn update_enter_mode(&mut self, mode: ChatMode) {
        // 广播工具确认请求到远程
        if mode == ChatMode::ToolConfirm && self.ws_bridge.is_some() {
            let tools: Vec<ToolConfirmInfo> = self
                .tool_executor
                .active_tool_calls
                .iter()
                .filter(|tc| matches!(tc.status, ToolExecStatus::PendingConfirm))
                .map(|tc| ToolConfirmInfo {
                    id: tc.tool_call_id.clone(),
                    name: tc.tool_name.clone(),
                    arguments: tc.arguments.clone(),
                    confirm_message: tc.confirm_message.clone(),
                })
                .collect();
            if !tools.is_empty() {
                self.broadcast_ws(WsOutbound::ToolConfirmRequest { tools });
            }
            self.broadcast_ws(WsOutbound::Status {
                state: "tool_confirm".to_string(),
            });
        }
        // 广播 Agent 权限确认请求到远程
        if mode == ChatMode::AgentPermConfirm
            && self.ws_bridge.is_some()
            && let Some(ref req) = self.ui.pending_agent_perm
        {
            self.broadcast_ws(WsOutbound::AgentPermRequest {
                agent_name: req.name.clone(),
                tool_name: req.tool_name.clone(),
                arguments: req.confirm_msg.clone(),
            });
            self.broadcast_ws(WsOutbound::Status {
                state: "agent_perm_confirm".to_string(),
            });
        }
        // 广播 Plan 审批请求到远程
        if mode == ChatMode::PlanApprovalConfirm
            && self.ws_bridge.is_some()
            && let Some(ref req) = self.ui.pending_plan_approval
        {
            self.broadcast_ws(WsOutbound::PlanApprovalRequest {
                agent_name: req.agent_name.clone(),
                plan_summary: req.plan_content.clone(),
            });
            self.broadcast_ws(WsOutbound::Status {
                state: "plan_approval".to_string(),
            });
        }
        if mode == ChatMode::Browse {
            self.ui.browse_filter.clear();
            self.ui.browse_role_filter = None;
        }
        self.ui.mode = mode;
    }

    pub(super) fn update_exit_to_chat(&mut self) {
        self.ui.mode = ChatMode::Chat;
        self.ui.browse_filter.clear();
        self.ui.browse_role_filter = None;
    }

    pub(super) fn update_scroll(&mut self, dir: CursorDirection) {
        match dir {
            CursorDirection::Up => self.scroll_up(),
            CursorDirection::Down => self.scroll_down(),
        }
    }

    pub(super) fn update_page_scroll(&mut self, dir: CursorDirection) {
        match dir {
            CursorDirection::Up => {
                for _ in 0..PAGE_SCROLL_LINES {
                    self.scroll_up();
                }
            }
            CursorDirection::Down => {
                for _ in 0..PAGE_SCROLL_LINES {
                    self.scroll_down();
                }
            }
        }
    }

    pub(super) fn update_browse_navigate(&mut self, dir: CursorDirection) {
        let filtered = self.browse_filtered_indices();
        if filtered.is_empty() {
            self.ui.mode = ChatMode::Chat;
            self.ui.msg_lines_cache = None;
            return;
        }
        let current_in_filtered = filtered.iter().position(|&i| i == self.ui.browse_msg_index);
        match dir {
            CursorDirection::Up => {
                let new_idx = match current_in_filtered {
                    Some(pos) if pos > 0 => filtered[pos - 1],
                    Some(_) => filtered[filtered.len() - 1],
                    None => *filtered.last().expect("browse mode 下 filtered 不应为空"),
                };
                self.ui.browse_msg_index = new_idx;
                self.ui.browse_scroll_offset = 0;
                self.ui.msg_lines_cache = None;
            }
            CursorDirection::Down => {
                let new_idx = match current_in_filtered {
                    Some(pos) if pos < filtered.len() - 1 => filtered[pos + 1],
                    Some(_) => filtered[0],
                    None => filtered[0],
                };
                self.ui.browse_msg_index = new_idx;
                self.ui.browse_scroll_offset = 0;
                self.ui.msg_lines_cache = None;
            }
        }
    }

    pub(super) fn update_browse_fine_scroll(&mut self, dir: CursorDirection) {
        match dir {
            CursorDirection::Up => {
                self.ui.browse_scroll_offset = self
                    .ui
                    .browse_scroll_offset
                    .saturating_sub(FINE_SCROLL_LINES);
            }
            CursorDirection::Down => {
                self.ui.browse_scroll_offset = self
                    .ui
                    .browse_scroll_offset
                    .saturating_add(FINE_SCROLL_LINES);
            }
        }
    }

    pub(super) fn update_browse_copy_message(&mut self) {
        use crate::command::chat::render::cache::copy_to_clipboard;
        let copy_result = {
            let display = crate::util::safe_lock(&self.display_messages, "BrowseCopyMessage");
            display.get(self.ui.browse_msg_index).map(|msg| {
                let content = msg.content.clone();
                let role_label = if msg.role == MessageRole::Assistant {
                    "AI"
                } else if msg.role == MessageRole::User {
                    "用户"
                } else {
                    "系统"
                };
                (content, role_label.to_string())
            })
        };
        if let Some((content, role_label)) = copy_result {
            let filtered = self.browse_filtered_indices();
            let pos_in_filtered = filtered.iter().position(|&i| i == self.ui.browse_msg_index);
            let extra = if let Some(pos) = pos_in_filtered {
                format!(" ({}/{})", pos + 1, filtered.len())
            } else {
                String::new()
            };
            if copy_to_clipboard(&content) {
                self.show_toast(format!("已复制{}消息{}", role_label, extra), false);
            } else {
                self.show_toast("复制到剪切板失败", true);
            }
        }
    }

    pub(super) fn update_browse_input_char(&mut self, c: char) {
        self.ui.browse_filter.push(c);
        self.browse_jump_to_first_match();
        self.ui.msg_lines_cache = None;
    }

    pub(super) fn update_browse_delete_char(&mut self) {
        self.ui.browse_filter.pop();
        self.browse_jump_to_first_match();
        self.ui.msg_lines_cache = None;
    }

    pub(super) fn update_browse_clear_filter(&mut self) {
        self.ui.browse_filter.clear();
        self.ui.browse_role_filter = None;
        self.ui.msg_lines_cache = None;
    }

    pub(super) fn update_browse_toggle_role(&mut self) {
        self.ui.browse_role_filter = match &self.ui.browse_role_filter {
            None => Some("ai".to_string()),
            Some(r) if r == "ai" => Some("user".to_string()),
            _ => None,
        };
        self.browse_jump_to_first_match();
        self.ui.msg_lines_cache = None;
    }

    // ========== 流式控制 ==========

    pub(super) fn update_cancel_stream(&mut self) {
        self.cancel_stream();
        self.permission_queue.deny_all();
        self.plan_approval_queue.deny_all();
        if let Some(req) = self.ui.pending_agent_perm.take() {
            req.resolve(false);
        }
        if let Some(req) = self.ui.pending_plan_approval.take() {
            req.resolve(crate::command::chat::app::types::PlanDecision::Reject);
        }
        if matches!(
            self.ui.mode,
            ChatMode::AgentPermConfirm | ChatMode::PlanApprovalConfirm
        ) {
            self.ui.mode = ChatMode::Chat;
        }
    }

    // ========== UI 管理 ==========

    pub(super) fn update_save_config(&mut self) {
        // UI 已直接修改 agent_config，直接持久化即可
        // （不再从 deferred_tools 同步，因为 deferred_tools 包含 LoadTool 的会话级修改）
        let _ = save_agent_config(&self.state.agent_config);
        self.ui.mode = ChatMode::Chat;
    }

    // ========== 快速操作 ==========

    pub(super) fn update_copy_last_ai_reply(&mut self) {
        use crate::command::chat::render::cache::copy_to_clipboard;
        let last_ai_content = safe_lock(&self.context_messages, "CopyLastAiReply")
            .iter()
            .rev()
            .find(|m| m.role == MessageRole::Assistant)
            .map(|m| m.content.clone());
        if let Some(content) = last_ai_content {
            if copy_to_clipboard(&content) {
                self.show_toast("已复制最后一条 AI 回复", false);
            } else {
                self.show_toast("复制到剪切板失败", true);
            }
        } else {
            self.show_toast("暂无 AI 回复可复制", true);
        }
    }

    pub(super) fn update_open_log_windows(&mut self) {
        use crate::constants::{AGENT_DIR, AGENT_LOG_DIR, AGENT_LOG_ERROR, AGENT_LOG_INFO};
        let log_dir = crate::config::YamlConfig::data_dir()
            .join(AGENT_DIR)
            .join(AGENT_LOG_DIR);
        let info_log = log_dir.join(AGENT_LOG_INFO);
        let error_log = log_dir.join(AGENT_LOG_ERROR);
        let info_cmd = format!("tail -f '{}'; exit", info_log.to_string_lossy())
            .replace('\\', "\\\\")
            .replace('"', "\\\"");
        let error_cmd = format!("tail -f '{}'; exit", error_log.to_string_lossy())
            .replace('\\', "\\\\")
            .replace('"', "\\\"");
        let apple_script = format!(
            "tell application \"Terminal\"\n\
                do script \"{}\"\n\
                do script \"{}\"\n\
                activate\n\
            end tell",
            info_cmd, error_cmd
        );
        let _ = std::process::Command::new("osascript")
            .arg("-e")
            .arg(&apple_script)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }

    // ========== 应用控制 ==========

    pub(super) fn update_toggle_expand_tools(&mut self) {
        self.ui.expand_tools = !self.ui.expand_tools;
        self.ui.msg_lines_cache = None;
        self.ui.auto_scroll = true;
        self.ui.scroll_offset = usize::MAX;
        self.show_toast(
            if self.ui.expand_tools {
                "展开工具详情"
            } else {
                "折叠工具详情"
            },
            false,
        );
    }

    pub(super) fn update_toggle_auto_approve(&mut self) {
        self.ui.auto_approve = !self.ui.auto_approve;
        let sid = self.session_id.clone();
        if let Some(mut meta) = crate::command::chat::storage::load_session_meta_file(&sid) {
            meta.auto_approve = self.ui.auto_approve;
            crate::command::chat::storage::save_session_meta_file(&meta);
        }
        self.show_toast(
            if self.ui.auto_approve {
                "Bypass 模式已开启"
            } else {
                "Bypass 模式已关闭"
            },
            false,
        );
    }
}
