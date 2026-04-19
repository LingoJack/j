use super::chat_app::ChatApp;
use crate::command::chat::constants::{ROLE_ASSISTANT, ROLE_USER};

impl ChatApp {
    /// 返回浏览模式下符合当前过滤条件的消息索引列表
    pub fn browse_filtered_indices(&self) -> Vec<usize> {
        let filter_lower = self.ui.browse_filter.to_lowercase();
        self.state
            .session
            .messages
            .iter()
            .enumerate()
            .filter(|(_, m)| {
                match &self.ui.browse_role_filter {
                    Some(r) if r == "ai" && m.role != ROLE_ASSISTANT => return false,
                    Some(r) if r == "user" && m.role != ROLE_USER => return false,
                    _ => {}
                }
                if !filter_lower.is_empty() {
                    return m.content.to_lowercase().contains(&filter_lower);
                }
                true
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// 跳转到过滤列表中最近的消息
    pub(super) fn browse_jump_to_first_match(&mut self) {
        let filtered = self.browse_filtered_indices();
        if filtered.is_empty() {
            return;
        }
        if filtered.contains(&self.ui.browse_msg_index) {
            return;
        }
        let target = filtered
            .iter()
            .rev()
            .find(|&&i| i <= self.ui.browse_msg_index)
            .copied()
            .unwrap_or(filtered[0]);
        self.ui.browse_msg_index = target;
        self.ui.browse_scroll_offset = 0;
    }
}
