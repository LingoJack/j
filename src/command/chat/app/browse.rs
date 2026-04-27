use super::chat_app::ChatApp;
use crate::command::chat::storage::{DisplayType, MessageRole};
use crate::util::safe_lock;

impl ChatApp {
    /// 返回浏览模式下符合当前过滤条件的消息索引列表
    ///
    /// 默认过滤掉 tool group（`ToolCallRequest` / `ToolResult`），
    /// 只在用户可读的文本消息之间导航。
    pub fn browse_filtered_indices(&self) -> Vec<usize> {
        let filter_lower = self.ui.browse_filter.to_lowercase();
        let display = safe_lock(&self.display_messages, "browse_filtered_indices");
        display
            .iter()
            .enumerate()
            .filter(|(_, m)| {
                // 过滤掉 tool group：工具调用请求和工具执行结果
                let dt = m.display_type();
                if dt == DisplayType::ToolCallRequest || dt == DisplayType::ToolResult {
                    return false;
                }
                match &self.ui.browse_role_filter {
                    Some(r) if r == "ai" && m.role != MessageRole::Assistant => return false,
                    Some(r) if r == "user" && m.role != MessageRole::User => return false,
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
