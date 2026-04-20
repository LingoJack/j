//! 列表组件

use ratatui::{
    style::{Color, Style},
    text::Line,
};

/// 配置面板列表组件
///
/// 每个 item 行使用 panel 背景色（bg_primary），间距行不加背景色，
/// 利用色差在视觉上产生"行内 padding"效果，比空行更紧凑。
/// 同时维护 `field_line_indices`，用于滚动定位。
pub struct ItemList<'a> {
    lines: Vec<Line<'a>>,
    field_line_indices: Vec<usize>,
    item_bg: Color,
}

impl<'a> ItemList<'a> {
    /// 创建空列表，传入 item 背景色
    pub fn new(item_bg: Color) -> Self {
        Self {
            lines: Vec::new(),
            field_line_indices: Vec::new(),
            item_bg,
        }
    }

    /// 添加一个列表项（自动带背景色 + 空行间距）
    pub fn push(&mut self, line: Line<'a>) {
        if !self.lines.is_empty() {
            self.lines.push(Line::from(""));
        }
        self.field_line_indices.push(self.lines.len());
        self.lines.push(self.with_bg(line));
    }

    /// 添加一行非 item 内容（如分组标题、分隔线），不触发间距逻辑
    pub fn push_raw(&mut self, line: Line<'a>) {
        self.lines.push(line);
    }

    /// 消费 self，返回 (lines, field_line_indices)
    pub fn into_parts(self) -> (Vec<Line<'a>>, Vec<usize>) {
        (self.lines, self.field_line_indices)
    }

    /// 给一行的所有 span 加上 item 背景色
    fn with_bg(&self, line: Line<'a>) -> Line<'a> {
        line.patch_style(Style::default().bg(self.item_bg))
    }
}

impl Default for ItemList<'_> {
    fn default() -> Self {
        Self {
            lines: Vec::new(),
            field_line_indices: Vec::new(),
            item_bg: Color::Reset,
        }
    }
}
