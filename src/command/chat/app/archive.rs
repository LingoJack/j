use super::chat_app::ChatApp;
use super::ui_state::ChatMode;
use crate::command::chat::storage::{SessionEvent, append_session_event};

impl ChatApp {
    /// 开始归档确认流程
    pub fn start_archive_confirm(&mut self) {
        use crate::command::chat::archive::generate_default_archive_name;
        self.ui.archive_default_name = generate_default_archive_name();
        self.ui.archive_custom_name = String::new();
        self.ui.archive_editing_name = false;
        self.ui.archive_edit_cursor = 0;
        self.ui.mode = ChatMode::ArchiveConfirm;
    }

    /// 开始还原流程（加载归档列表）
    pub fn start_archive_list(&mut self) {
        use crate::command::chat::archive::list_archives;
        self.ui.archives = list_archives();
        self.ui.archive_list_index = 0;
        self.ui.restore_confirm_needed = false;
        self.ui.mode = ChatMode::ArchiveList;
    }

    /// 执行归档
    pub fn do_archive(&mut self, name: &str) {
        use crate::command::chat::archive::create_archive;

        match create_archive(name, self.state.session.messages.clone()) {
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
        use crate::command::chat::archive::restore_archive;

        let archive_name = self
            .ui
            .archives
            .get(self.ui.archive_list_index)
            .map(|a| a.name.clone());

        if let Some(archive_name) = archive_name {
            match restore_archive(&archive_name) {
                Ok(messages) => {
                    self.state.session.messages = messages.clone();
                    self.ui.scroll_offset = u16::MAX;
                    self.ui.msg_lines_cache = None;
                    self.ui.clear_input();
                    append_session_event(&self.session_id, &SessionEvent::Restore { messages });
                    self.persisted_message_count = self.state.session.messages.len();
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
        use crate::command::chat::archive::delete_archive;

        if let Some(archive) = self.ui.archives.get(self.ui.archive_list_index) {
            match delete_archive(&archive.name) {
                Ok(_) => {
                    self.show_toast(format!("归档已删除: {}", archive.name), false);
                    self.ui.archives = crate::command::chat::archive::list_archives();
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
