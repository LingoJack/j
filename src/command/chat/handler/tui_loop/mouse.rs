//! 鼠标事件辅助函数

use crate::command::chat::app::{Action, ChatApp, ChatMode, ConfigTab, CursorDirection};
use crate::command::chat::ui::chat::copy_selection_to_clipboard;
use ratatui::layout::Rect;

/// 根据当前 ChatMode (及 ConfigTab) 将鼠标滚轮事件路由到对应的导航 Action。
pub(super) fn mouse_scroll_action(app: &ChatApp, dir: CursorDirection) -> Action {
    match app.ui.mode {
        ChatMode::Config if !app.ui.config_editing => {
            // 不同 ConfigTab 使用各自的导航索引和 Action
            match app.ui.config_tab {
                ConfigTab::Session => Action::SessionListNavigate(dir),
                ConfigTab::Archive => Action::ArchiveListNavigate(dir),
                ConfigTab::Teammates => Action::TeammatesNavigate(dir),
                ConfigTab::Tools | ConfigTab::Skills | ConfigTab::Hooks | ConfigTab::Commands => {
                    Action::ToggleMenuNavigate(dir)
                }
                _ => Action::ConfigNavigate(dir),
            }
        }
        ChatMode::SelectModel => Action::ModelSelectNavigate(dir),
        ChatMode::SelectTheme => Action::ThemeSelectNavigate(dir),
        ChatMode::ArchiveList => Action::ArchiveListNavigate(dir),
        _ => Action::Scroll(dir),
    }
}

/// 处理配置界面的鼠标左键点击事件。
///
/// 返回 `Some(action)` 表示需要执行的 Action，返回 `None` 表示点击未命中任何可交互区域。
pub(super) fn config_mouse_click(app: &mut ChatApp, col: u16, row: u16) -> Option<Action> {
    // 不可编辑状态下才处理点击
    if app.ui.config_editing {
        return None;
    }

    // ── 1. 检测 Tab 栏点击 ──
    if let Some(tab_bar_y) = app.ui.config_tab_bar_y
        && row == tab_bar_y
    {
        for hitbox in &app.ui.config_tab_hitboxes {
            if col >= hitbox.start_col && col < hitbox.end_col {
                if app.ui.config_tab != hitbox.tab {
                    return Some(Action::ConfigSwitchTabTo(hitbox.tab));
                }
                return None; // 点击当前已激活的 Tab，不做任何操作
            }
        }
        return None; // 点击了 Tab 栏但不在任何 Tab 上
    }

    // ── 2. 检测 Model tab 左侧 Provider 列表点击 ──
    if app.ui.config_tab == ConfigTab::Model
        && let Some(provider_area) = app.ui.config_provider_area
        && is_point_in_rect(col, row, provider_area)
    {
        let provider_lines = &app.ui.config_provider_lines;
        if provider_lines.is_empty() {
            return None;
        }
        let inner_y = (row - provider_area.y) as usize;
        // Provider 列表无滚动偏移，每项占一行
        let clicked_idx = match provider_lines.binary_search(&inner_y) {
            Ok(idx) => idx,
            Err(0) => return None,
            Err(idx) => idx - 1,
        };
        let current = app.ui.config_provider_idx;
        if clicked_idx == current && !app.ui.model_in_fields {
            // 再次点击已选中的 Provider（且已在 Provider 层级）：视为确认进入字段编辑
            return Some(Action::ModelToggleLevel);
        }
        return Some(Action::ConfigProviderSelect(clicked_idx));
    }

    // ── 3. 检测列表项点击 ──
    let list_area = app.ui.config_list_area?;
    if !is_point_in_rect(col, row, list_area) {
        return None;
    }

    // 计算列表内的内容行号
    // list_area 无顶部 border（header_block 只有 TOP|LEFT|RIGHT，list_block 只有 BOTTOM|LEFT|RIGHT）
    // 所以列表内容从 list_area.y 开始
    let inner_y = (row - list_area.y) as usize;
    let content_y = inner_y + app.ui.config_scroll_offset as usize;

    // 在 field_line_indices 中查找：每个可交互项占据连续的若干行，
    // field_line_indices[i] 是第 i 个项的起始行号
    let field_lines = &app.ui.config_field_lines;
    if field_lines.is_empty() {
        return None;
    }

    // 二分查找：找到最后一个起始行 <= content_y 的项
    let clicked_idx = match field_lines.binary_search(&content_y) {
        Ok(idx) => idx,        // 精确命中某项的起始行
        Err(0) => return None, // content_y 小于第一项，点在空白区
        Err(idx) => {
            // idx 是第一个 > content_y 的位置，所以 idx-1 是最后一个 <= content_y 的
            let candidate = idx - 1;
            // 检查 content_y 是否在 candidate 的范围内
            // 如果有下一项，范围是 [candidate_start, next_start)；否则到列表末尾
            let candidate_start = field_lines[candidate];
            if content_y < candidate_start {
                return None;
            }
            candidate
        }
    };

    // 获取当前选中索引，判断是否为"再次点击已选中项"
    let current_idx = match app.ui.config_tab {
        ConfigTab::Session => app.ui.session_list_index,
        ConfigTab::Archive => app.ui.archive_list_index,
        ConfigTab::Teammates => app.ui.teammate_list_index,
        ConfigTab::Global if app.ui.compact_exempt_sublist => app.ui.compact_exempt_idx,
        ConfigTab::Model
        | ConfigTab::Tools
        | ConfigTab::Skills
        | ConfigTab::Hooks
        | ConfigTab::Commands
        | ConfigTab::Global => app.ui.config_field_idx,
    };

    if clicked_idx == current_idx {
        // 再次点击已选中项：触发 Enter 操作
        return Some(config_enter_action(app));
    }

    // 单击新项：选中它
    Some(config_select_action(app, clicked_idx))
}

/// 根据当前 ConfigTab 返回"选中指定索引"的 Action。
pub(super) fn config_select_action(app: &ChatApp, idx: usize) -> Action {
    match app.ui.config_tab {
        ConfigTab::Session => Action::SessionListSelect(idx),
        ConfigTab::Archive => Action::ArchiveListSelect(idx),
        ConfigTab::Teammates => Action::TeammatesSelect(idx),
        ConfigTab::Global if app.ui.compact_exempt_sublist => Action::CompactExemptSelect(idx),
        _ => Action::ConfigFieldSelect(idx),
    }
}

/// 根据当前 ConfigTab 返回"确认/进入"的 Action。
pub(super) fn config_enter_action(app: &ChatApp) -> Action {
    match app.ui.config_tab {
        ConfigTab::Session | ConfigTab::Archive | ConfigTab::Teammates => Action::ConfigEnter,
        ConfigTab::Tools | ConfigTab::Skills | ConfigTab::Hooks | ConfigTab::Commands => {
            Action::ToggleMenuToggle
        }
        _ => Action::ConfigEnter,
    }
}

/// 判断点 (col, row) 是否在 Rect 内部（不含 border）。
pub(super) fn is_point_in_rect(col: u16, row: u16, rect: Rect) -> bool {
    col >= rect.x && col < rect.x + rect.width && row >= rect.y && row < rect.y + rect.height
}

/// 执行右键菜单的复制操作。
///
/// 优先级：有选区时复制选区内容，无选区时复制整条消息。
pub(super) fn execute_context_menu_copy(app: &mut ChatApp) {
    // 优先复制选区
    if app.ui.mouse_selection.is_some() {
        copy_selection_to_clipboard(app);
        app.ui.mouse_selection = None;
        return;
    }

    // 无选区：通过 global_line 定位消息并复制整条消息内容
    let global_line = match &app.ui.context_menu {
        Some(m) => m.global_line,
        None => return,
    };

    let cached = match app.ui.msg_lines_cache.as_ref() {
        Some(c) => c,
        None => return,
    };

    // 二分查找 global_line 所属消息索引
    let msg_index = {
        let mut lo = 0usize;
        let mut hi = cached.msg_start_lines.len();
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let (_, start) = cached.msg_start_lines[mid];
            if start <= global_line {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        // lo 是第一个 start > global_line 的位置，所属消息是 lo - 1
        lo.saturating_sub(1)
    };

    // 从 display_messages 获取消息内容
    let content = {
        let display = crate::util::safe_lock(&app.display_messages, "ContextMenuCopy");
        display.get(msg_index).map(|msg| msg.content.clone())
    };

    if let Some(content) = content
        && !content.is_empty()
    {
        use crate::command::chat::render::cache::copy_to_clipboard;
        if copy_to_clipboard(&content) {
            app.show_toast("已复制到剪贴板", false);
        } else {
            app.show_toast("复制到剪贴板失败", true);
        }
    }
}
