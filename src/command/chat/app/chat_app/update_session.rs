use super::ChatApp;
use crate::command::chat::app::action::CursorDirection;
use crate::command::chat::app::ui_state::ChatMode;
use crate::command::chat::remote::protocol::WsOutbound;
use crate::command::chat::storage::{
    delete_session, generate_session_id, list_sessions, load_session, session_file_path,
};

impl ChatApp {
    pub(super) fn update_archive_confirm_move_cursor(&mut self, dir: CursorDirection) {
        match dir {
            CursorDirection::Up => {
                self.ui.archive_edit_cursor = self.ui.archive_edit_cursor.saturating_sub(1);
            }
            CursorDirection::Down => {
                let char_count = self.ui.archive_custom_name.chars().count();
                if self.ui.archive_edit_cursor < char_count {
                    self.ui.archive_edit_cursor += 1;
                }
            }
        }
    }

    pub(super) fn update_archive_confirm_input_char(&mut self, c: char) {
        let chars: Vec<char> = self.ui.archive_custom_name.chars().collect();
        self.ui.archive_custom_name = chars[..self.ui.archive_edit_cursor]
            .iter()
            .chain(std::iter::once(&c))
            .chain(chars[self.ui.archive_edit_cursor..].iter())
            .collect();
        self.ui.archive_edit_cursor += 1;
    }

    pub(super) fn update_archive_confirm_delete_char(&mut self) {
        if self.ui.archive_edit_cursor > 0 {
            let chars: Vec<char> = self.ui.archive_custom_name.chars().collect();
            self.ui.archive_custom_name = chars[..self.ui.archive_edit_cursor - 1]
                .iter()
                .chain(chars[self.ui.archive_edit_cursor..].iter())
                .collect();
            self.ui.archive_edit_cursor -= 1;
        }
    }

    pub(super) fn update_list_sessions(&mut self) {
        let sessions = list_sessions();
        self.broadcast_ws(WsOutbound::SessionList { sessions });
    }

    pub(super) fn update_switch_session(&mut self, session_id: String) {
        if self.state.is_loading {
            self.broadcast_ws(WsOutbound::Error {
                message: "AI 正在回复中，无法切换会话".to_string(),
            });
        } else if self.ui.mode == ChatMode::ToolConfirm {
            self.broadcast_ws(WsOutbound::Error {
                message: "等待工具确认中，无法切换会话".to_string(),
            });
        } else {
            // 检查目标文件是否存在
            let target_path = session_file_path(&session_id);
            if !target_path.exists() {
                self.broadcast_ws(WsOutbound::Error {
                    message: "会话不存在".to_string(),
                });
            } else {
                // 保存当前会话（消息 + 状态）
                self.persist_new_messages();
                self.persist_new_display_messages();
                self.save_session_state();
                // 清除运行时状态
                self.clear_runtime_state();
                // 加载目标会话
                let messages = load_session(&session_id);
                self.session_id = session_id.clone();
                if let Ok(mut s) = self.shared_session_id.lock() {
                    *s = session_id.clone();
                }
                // 重建双通道（从加载的消息 → display + context）
                self.rebuild_channels_from_loaded(messages);
                // 恢复目标会话的状态
                self.restore_session_state();
                self.ui.scroll_offset = 0;
                self.ui.msg_lines_cache = None;
                if let Ok(mut ct) = self.context_tokens.lock() {
                    *ct = 0;
                }
                // 广播同步 + 切换通知
                let sync = self.build_sync_outbound();
                self.broadcast_ws(sync);
                self.broadcast_ws(WsOutbound::SessionSwitched { session_id });
            }
        }
    }

    pub(super) fn update_new_session(&mut self) {
        if self.state.is_loading {
            self.broadcast_ws(WsOutbound::Error {
                message: "AI 正在回复中，无法新建会话".to_string(),
            });
        } else if self.ui.mode == ChatMode::ToolConfirm {
            self.broadcast_ws(WsOutbound::Error {
                message: "等待工具确认中，无法新建会话".to_string(),
            });
        } else {
            // 保存当前会话（消息 + 状态）
            self.persist_new_messages();
            self.persist_new_display_messages();
            self.save_session_state();
            // 清除运行时状态
            self.clear_runtime_state();
            // 生成新会话
            let new_id = generate_session_id();
            self.session_id = new_id.clone();
            if let Ok(mut s) = self.shared_session_id.lock() {
                *s = new_id.clone();
            }
            self.clear_channels();
            self.persisted_message_count = 0;
            self.persisted_display_count = 0;
            self.ui.scroll_offset = 0;
            if let Ok(mut ct) = self.context_tokens.lock() {
                *ct = 0;
            }
            // 广播同步 + 切换通知
            let sync = self.build_sync_outbound();
            self.broadcast_ws(sync);
            self.broadcast_ws(WsOutbound::SessionSwitched { session_id: new_id });
        }
    }

    pub(super) fn update_load_session_list(&mut self) {
        let mut sessions = list_sessions();
        // 过滤掉当前 session
        sessions.retain(|s| s.id != self.session_id);
        self.ui.session_list = sessions;
        self.ui.session_list_index = 0;
        self.ui.session_restore_confirm = false;
    }

    pub(super) fn update_session_list_navigate(&mut self, dir: CursorDirection) {
        let count = self.ui.session_list.len();
        if count > 0 {
            match dir {
                CursorDirection::Up => {
                    self.ui.session_list_index = if self.ui.session_list_index == 0 {
                        count - 1
                    } else {
                        self.ui.session_list_index - 1
                    };
                }
                CursorDirection::Down => {
                    self.ui.session_list_index = (self.ui.session_list_index + 1) % count;
                }
            }
        }
    }

    pub(super) fn update_restore_session(&mut self) {
        if self.ui.session_list.is_empty() {
            return;
        }
        let idx = self.ui.session_list_index;
        if let Some(meta) = self.ui.session_list.get(idx) {
            let target_id = meta.id.clone();
            // 保存当前会话（消息 + 状态）
            self.persist_new_messages();
            self.persist_new_display_messages();
            self.save_session_state();
            // 清除运行时状态
            self.clear_runtime_state();
            // 加载目标会话
            let messages = load_session(&target_id);
            self.session_id = target_id.clone();
            if let Ok(mut s) = self.shared_session_id.lock() {
                *s = target_id;
            }
            // 重建双通道（从加载的消息 → display + context）
            self.rebuild_channels_from_loaded(messages);
            // 恢复目标会话的状态
            self.restore_session_state();
            self.ui.scroll_offset = usize::MAX;
            self.ui.msg_lines_cache = None;
            self.ui.session_restore_confirm = false;
            if let Ok(mut ct) = self.context_tokens.lock() {
                *ct = 0;
            }
            self.ui.mode = ChatMode::Chat;
            self.show_toast("会话已恢复".to_string(), false);
        }
    }

    pub(super) fn update_delete_session(&mut self) {
        if self.ui.session_list.is_empty() {
            return;
        }
        let idx = self.ui.session_list_index;
        if let Some(meta) = self.ui.session_list.get(idx) {
            let id = meta.id.clone();
            if delete_session(&id) {
                self.ui.session_list.remove(idx);
                if self.ui.session_list_index >= self.ui.session_list.len()
                    && self.ui.session_list_index > 0
                {
                    self.ui.session_list_index -= 1;
                }
                self.show_toast("会话已删除".to_string(), false);
            } else {
                self.show_toast("删除失败".to_string(), true);
            }
        }
    }

    pub(super) fn update_new_session_from_list(&mut self) {
        // 保存当前会话（消息 + 状态）
        self.persist_new_messages();
        self.persist_new_display_messages();
        self.save_session_state();
        self.clear_runtime_state();
        // 生成新会话
        let new_id = generate_session_id();
        self.session_id = new_id.clone();
        if let Ok(mut s) = self.shared_session_id.lock() {
            *s = new_id;
        }
        self.clear_channels();
        self.persisted_message_count = 0;
        self.persisted_display_count = 0;
        self.ui.scroll_offset = 0;
        self.ui.msg_lines_cache = None;
        if let Ok(mut ct) = self.context_tokens.lock() {
            *ct = 0;
        }
        self.ui.mode = ChatMode::Chat;
        self.show_toast("已新建会话".to_string(), false);
    }

    pub(super) fn update_respawn_teammate(&mut self, name: String) {
        // 从 recovered_teammates 取出保存的 prompt/role，重新创建 teammate
        let snapshot = if let Ok(mgr) = self.teammate_manager.lock() {
            mgr.get_recovered_teammate(&name)
        } else {
            None
        };
        if let Some(_snapshot) = snapshot {
            if let Ok(mut mgr) = self.teammate_manager.lock() {
                mgr.remove_recovered_teammate(&name);
            }
            // TODO: 完整的 RespawnTeammate 需要调用 CreateTeammateTool 的逻辑
            // 当前简化实现：提示用户手动重新创建
            self.show_toast(
                format!("Teammate '{}' 的上下文已恢复，请通过 AI 重新创建", name),
                false,
            );
        } else {
            self.show_toast(format!("未找到 teammate '{}'", name), true);
        }
    }

    /// Session 列表：鼠标点击选中指定索引
    pub(super) fn update_session_list_select(&mut self, idx: usize) {
        let count = self.ui.session_list.len();
        if count > 0 && idx < count {
            self.ui.session_list_index = idx;
        }
    }

    pub(super) fn update_archive_list_navigate(&mut self, dir: CursorDirection) {
        let count = self.ui.archives.len();
        if count > 0 {
            match dir {
                CursorDirection::Up => {
                    self.ui.archive_list_index = if self.ui.archive_list_index == 0 {
                        count - 1
                    } else {
                        self.ui.archive_list_index - 1
                    };
                }
                CursorDirection::Down => {
                    self.ui.archive_list_index = if self.ui.archive_list_index >= count - 1 {
                        0
                    } else {
                        self.ui.archive_list_index + 1
                    };
                }
            }
        }
    }

    /// 归档列表：鼠标点击选中指定索引
    pub(super) fn update_archive_list_select(&mut self, idx: usize) {
        let count = self.ui.archives.len();
        if count > 0 && idx < count {
            self.ui.archive_list_index = idx;
        }
    }
}
