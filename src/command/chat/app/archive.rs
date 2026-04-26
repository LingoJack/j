use super::chat_app::ChatApp;
use super::ui_state::ChatMode;
use crate::command::chat::storage::{
    ChatMessage, SessionEvent, SessionPaths, append_event_to_path, append_session_event,
};
use crate::util::safe_lock;

impl ChatApp {
    /// 开始归档确认流程
    pub fn start_archive_confirm(&mut self) {
        use crate::command::chat::infra::archive::generate_default_archive_name;
        self.ui.archive_default_name = generate_default_archive_name();
        self.ui.archive_custom_name = String::new();
        self.ui.archive_editing_name = false;
        self.ui.archive_edit_cursor = 0;
        self.ui.mode = ChatMode::ArchiveConfirm;
    }

    /// 开始还原流程（加载归档列表）
    pub fn start_archive_list(&mut self) {
        use crate::command::chat::infra::archive::list_archives;
        self.ui.archives = list_archives();
        self.ui.archive_list_index = 0;
        self.ui.restore_confirm_needed = false;
        self.ui.mode = ChatMode::ArchiveList;
    }

    /// 执行归档
    pub fn do_archive(&mut self, name: &str) {
        use crate::command::chat::infra::archive::create_archive;

        let messages = safe_lock(&self.context_messages, "do_archive::ctx").clone();
        match create_archive(name, messages) {
            Ok(_) => {
                self.clear_session();
                self.show_toast(format!("对话已归档: {}", name), false);
            }
            Err(e) => {
                self.show_toast(e, true);
            }
        }
        self.ui.mode = ChatMode::Chat;
    }

    /// 执行还原归档
    pub fn do_restore(&mut self) {
        use crate::command::chat::infra::archive::restore_archive;

        let archive_name = self
            .ui
            .archives
            .get(self.ui.archive_list_index)
            .map(|a| a.name.clone());

        if let Some(archive_name) = archive_name {
            match restore_archive(&archive_name) {
                Ok(messages) => {
                    // 重建双通道（从加载的消息 → display + context）
                    self.rebuild_channels_from_loaded(messages);
                    self.ui.scroll_offset = u16::MAX;
                    self.ui.msg_lines_cache = None;
                    self.ui.clear_input();
                    // context 持久化
                    let ctx_msgs =
                        safe_lock(&self.context_messages, "archive_restore::ctx").clone();
                    append_session_event(
                        &self.session_id,
                        &SessionEvent::Restore { messages: ctx_msgs },
                    );
                    // display 持久化
                    let display_msgs: Vec<ChatMessage> =
                        safe_lock(&self.display_messages, "archive_restore::display").clone();
                    let display_count = display_msgs.len();
                    append_event_to_path(
                        &SessionPaths::new(&self.session_id).display(),
                        &SessionEvent::Restore {
                            messages: display_msgs,
                        },
                    );
                    self.persisted_display_count = display_count;
                    self.show_toast(format!("已还原归档: {}", archive_name), false);
                }
                Err(e) => {
                    self.show_toast(e, true);
                }
            }
        }
        self.ui.mode = ChatMode::Chat;
    }

    /// 删除选中的归档
    pub fn do_delete_archive(&mut self) {
        use crate::command::chat::infra::archive::delete_archive;

        if let Some(archive) = self.ui.archives.get(self.ui.archive_list_index) {
            match delete_archive(&archive.name) {
                Ok(_) => {
                    self.show_toast(format!("归档已删除: {}", archive.name), false);
                    self.ui.archives = crate::command::chat::infra::archive::list_archives();
                    if self.ui.archive_list_index >= self.ui.archives.len()
                        && self.ui.archive_list_index > 0
                    {
                        self.ui.archive_list_index -= 1;
                    }
                }
                Err(e) => {
                    self.show_toast(e, true);
                }
            }
        }
    }
}
