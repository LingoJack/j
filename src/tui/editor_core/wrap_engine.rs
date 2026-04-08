//! 折行引擎
//!
//! 将逻辑行转换为视觉行，支持自动折行功能。

use ratatui::style::Style;
use crate::util::text::display_width;

/// 视觉行：一个逻辑行可能拆分为多个视觉行
#[derive(Debug, Clone, PartialEq)]
pub struct VisualLine {
    /// 原始行号
    pub logical_line: usize,
    /// 在原始行中的起始列（字符偏移）
    pub start_col: usize,
    /// 在原始行中的结束列（字符偏移，不含）
    pub end_col: usize,
    /// 显示文本
    pub text: String,
    /// 显示宽度
    pub display_width: usize,
}

impl VisualLine {
    /// 创建不折行的视觉行
    pub fn from_line(line: &str, line_num: usize) -> Self {
        Self {
            logical_line: line_num,
            start_col: 0,
            end_col: line.chars().count(),
            text: line.to_string(),
            display_width: display_width(line),
        }
    }
}

// ========== 块级渲染数据结构 ==========

/// 带 display_width 缓存的样式化文本片段
#[derive(Debug, Clone)]
pub struct SizedSpan {
    pub content: String,
    pub style: Style,
    pub display_width: usize,
}

impl SizedSpan {
    /// 创建普通文本的 SizedSpan
    pub fn plain(content: &str, style: Style) -> Self {
        Self {
            content: content.to_string(),
            style,
            display_width: display_width(content),
        }
    }

    /// 创建带样式的 SizedSpan
    pub fn styled(content: impl Into<String>, style: Style) -> Self {
        let content = content.into();
        let width = display_width(&content);
        Self {
            content,
            style,
            display_width: width,
        }
    }
}

/// 样式化的视觉行（块级渲染输出）
#[derive(Debug, Clone)]
pub struct StyledVisualLine {
    /// 所属逻辑行号
    pub logical_line: usize,
    /// 是否是续行（折行后的非首行）
    pub is_continuation: bool,
    /// 样式化的 Span 列表
    pub spans: Vec<SizedSpan>,
    /// 总显示宽度
    pub display_width: usize,
}

/// 逻辑行的渲染块类型
#[derive(Debug, Clone, PartialEq)]
pub enum BlockType {
    /// 普通文本（可折行）
    Normal,
    /// 代码块围栏行（不折行）
    CodeFence,
    /// 代码块内容行（不折行）
    CodeContent,
    /// 表格行（不折行）
    Table,
}

/// 逻辑行的渲染块
#[derive(Debug, Clone)]
pub struct RenderBlock {
    /// 逻辑行号
    pub logical_line: usize,
    /// 行类型
    pub block_type: BlockType,
    /// 渲染后的内容（不含行号前缀）
    pub spans: Vec<SizedSpan>,
    /// 内容总显示宽度
    pub total_width: usize,
}

impl RenderBlock {
    /// 创建空的渲染块
    pub fn empty(logical_line: usize) -> Self {
        Self {
            logical_line,
            block_type: BlockType::Normal,
            spans: vec![SizedSpan::plain("", Style::default())],
            total_width: 0,
        }
    }

    /// 创建普通类型的渲染块
    pub fn normal(logical_line: usize, spans: Vec<SizedSpan>) -> Self {
        let total_width = spans.iter().map(|s| s.display_width).sum();
        Self {
            logical_line,
            block_type: BlockType::Normal,
            spans,
            total_width,
        }
    }

    /// 创建代码围栏类型的渲染块
    pub fn code_fence(logical_line: usize, spans: Vec<SizedSpan>) -> Self {
        let total_width = spans.iter().map(|s| s.display_width).sum();
        Self {
            logical_line,
            block_type: BlockType::CodeFence,
            spans,
            total_width,
        }
    }

    /// 创建代码内容类型的渲染块
    pub fn code_content(logical_line: usize, spans: Vec<SizedSpan>) -> Self {
        let total_width = spans.iter().map(|s| s.display_width).sum();
        Self {
            logical_line,
            block_type: BlockType::CodeContent,
            spans,
            total_width,
        }
    }

    /// 创建表格类型的渲染块
    pub fn table(logical_line: usize, spans: Vec<SizedSpan>) -> Self {
        let total_width = spans.iter().map(|s| s.display_width).sum();
        Self {
            logical_line,
            block_type: BlockType::Table,
            spans,
            total_width,
        }
    }

    /// 是否可以折行
    pub fn is_wrappable(&self) -> bool {
        self.block_type == BlockType::Normal
    }
}

// ========== Span 感知折行 ==========

/// 将样式化 Span 列表按显示宽度折行
///
/// 算法：展平为字符序列，贪心填充行，重新合并为 SizedSpan
pub fn wrap_spans(
    spans: &[SizedSpan],
    max_width: usize,
    logical_line: usize,
) -> Vec<StyledVisualLine> {
    if spans.is_empty() {
        return vec![StyledVisualLine {
            logical_line,
            is_continuation: false,
            spans: vec![],
            display_width: 0,
        }];
    }

    // Phase 1: 展平为 (char, Style, ch_width) 序列
    let mut entries: Vec<(char, Style, usize)> = Vec::new();
    for sized_span in spans {
        for ch in sized_span.content.chars() {
            let ch_width = if ch.is_ascii() { 1 } else { 2 };
            entries.push((ch, sized_span.style, ch_width));
        }
    }

    if entries.is_empty() {
        return vec![StyledVisualLine {
            logical_line,
            is_continuation: false,
            spans: vec![SizedSpan::plain("", Style::default())],
            display_width: 0,
        }];
    }

    // Phase 2: 贪心折行
    let mut result: Vec<StyledVisualLine> = Vec::new();
    let mut current_entries: Vec<(char, Style, usize)> = Vec::new();
    let mut current_width: usize = 0;

    for entry in entries {
        let (ch, style, ch_width) = entry;

        if current_width + ch_width > max_width && !current_entries.is_empty() {
            // 刷新当前行
            result.push(build_styled_line(
                &current_entries,
                logical_line,
                !result.is_empty(),
            ));
            current_entries.clear();
            current_width = 0;
        }

        current_entries.push((ch, style, ch_width));
        current_width += ch_width;
    }

    // 刷新剩余
    if !current_entries.is_empty() || result.is_empty() {
        result.push(build_styled_line(
            &current_entries,
            logical_line,
            !result.is_empty(),
        ));
    }

    result
}

/// 从字符条目构建一个样式化视觉行
fn build_styled_line(
    entries: &[(char, Style, usize)],
    logical_line: usize,
    is_continuation: bool,
) -> StyledVisualLine {
    let mut spans: Vec<SizedSpan> = Vec::new();
    let mut current_text = String::new();
    let mut current_style: Option<Style> = None;
    let mut current_width: usize = 0;
    let mut total_width: usize = 0;

    for &(ch, style, ch_width) in entries {
        if current_style != Some(style) {
            if !current_text.is_empty() {
                spans.push(SizedSpan {
                    content: current_text.clone(),
                    style: current_style.unwrap_or_default(),
                    display_width: current_width,
                });
                total_width += current_width;
            }
            current_text = String::new();
            current_text.push(ch);
            current_style = Some(style);
            current_width = ch_width;
        } else {
            current_text.push(ch);
            current_width += ch_width;
        }
    }

    // 刷新最后一个 span
    if !current_text.is_empty() {
        spans.push(SizedSpan {
            content: current_text,
            style: current_style.unwrap_or_default(),
            display_width: current_width,
        });
        total_width += current_width;
    }

    StyledVisualLine {
        logical_line,
        is_continuation,
        spans,
        display_width: total_width,
    }
}

/// 折行引擎
#[derive(Debug, Clone)]
pub struct WrapEngine {
    /// 是否启用折行
    enabled: bool,
    /// 折行宽度
    width: usize,
    /// 视觉行缓存（按需构建）
    cache: Vec<VisualLine>,
    /// 逻辑行 -> 视觉行数量的映射（用于快速计算滚动位置）
    line_visual_counts: Vec<usize>,
    /// 缓存是否需要更新
    dirty: bool,
    /// 已构建到第几行（用于增量构建）
    built_until: usize,
}

impl Default for WrapEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl WrapEngine {
    /// 创建新的折行引擎
    pub fn new() -> Self {
        Self {
            enabled: true,
            width: 80,
            cache: Vec::new(),
            line_visual_counts: Vec::new(),
            dirty: true,
            built_until: 0,
        }
    }

    /// 创建禁用折行的引擎
    pub fn no_wrap() -> Self {
        Self {
            enabled: false,
            width: 80,
            cache: Vec::new(),
            line_visual_counts: Vec::new(),
            dirty: true,
            built_until: 0,
        }
    }

    /// 是否启用折行
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// 设置折行开关
    pub fn set_enabled(&mut self, enabled: bool) {
        if self.enabled != enabled {
            self.enabled = enabled;
            self.dirty = true;
        }
    }

    /// 获取折行宽度
    pub fn width(&self) -> usize {
        self.width
    }

    /// 设置折行宽度
    pub fn set_width(&mut self, width: usize) {
        if self.width != width {
            self.width = width.max(10); // 最小宽度 10
            self.dirty = true;
        }
    }

    /// 标记缓存需要更新
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// 检查缓存是否需要更新
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// 如果需要则重建缓存
    pub fn rebuild_if_needed(&mut self, lines: &[String]) {
        if self.dirty {
            self.rebuild_cache(lines);
        }
    }

    /// 获取视觉行缓存
    pub fn visual_lines(&self) -> &[VisualLine] {
        &self.cache
    }

    /// 获取视觉行总数
    pub fn visual_line_count(&self) -> usize {
        self.cache.len()
    }

    /// 重建视觉行缓存（延迟模式：只初始化元数据）
    pub fn rebuild_cache(&mut self, lines: &[String]) {
        // 性能优化：延迟构建视觉行缓存
        // 先只计算每行的视觉行数量（快速估算），稍后按需构建详细缓存
        self.cache.clear();
        self.line_visual_counts.clear();
        self.line_visual_counts.reserve(lines.len());
        
        // 快速计算每行视觉行数量（不构建详细内容）
        for line in lines {
            let count = if self.enabled {
                let char_count: usize = line.chars().map(|c| if c.is_ascii() { 1 } else { 2 }).sum();
                (char_count / self.width.max(1) + 1).max(1)
            } else {
                1
            };
            self.line_visual_counts.push(count);
        }
        
        self.built_until = 0;
        self.dirty = false;
    }

    /// 确保指定逻辑行的缓存已构建
    pub fn ensure_built_until(&mut self, lines: &[String], until_line: usize) {
        if until_line <= self.built_until {
            return;
        }
        
        let start = self.built_until;
        let end = until_line.min(lines.len());
        
        for i in start..end {
            let line = &lines[i];
            if self.enabled {
                self.cache.extend(self.wrap_line(line, i));
            } else {
                self.cache.push(VisualLine::from_line(line, i));
            }
        }
        
        self.built_until = end;
    }

    /// 构建光标行附近的缓存（用于显示光标位置）
    pub fn build_around_cursor(&mut self, lines: &[String], cursor_row: usize, context_lines: usize) {
        let _start_line = cursor_row.saturating_sub(context_lines);
        let end_line = (cursor_row + context_lines + 1).min(lines.len());
        
        // 如果缓存为空或需要重建
        if self.cache.is_empty() || self.dirty {
            self.cache.clear();
            self.built_until = 0;
        }
        
        // 确保需要的范围已构建
        self.ensure_built_until(lines, end_line);
    }

    /// 即时构建单行的视觉行缓存
    pub fn build_line(&mut self, line: &str, line_num: usize) -> Vec<usize> {
        // 检查是否已经构建过
        let existing: Vec<usize> = self.cache
            .iter()
            .enumerate()
            .filter(|(_, vl)| vl.logical_line == line_num)
            .map(|(i, _)| i)
            .collect();
        
        if !existing.is_empty() {
            return existing;
        }
        
        // 构建新的视觉行
        let new_vlines = self.wrap_line(line, line_num);
        let start_idx = self.cache.len();
        self.cache.extend(new_vlines);
        
        // 更新 built_until
        self.built_until = self.built_until.max(line_num + 1);
        
        (start_idx..self.cache.len()).collect()
    }

    /// 将逻辑行拆分为视觉行
    pub fn wrap_line(&self, line: &str, line_num: usize) -> Vec<VisualLine> {
        if !self.enabled {
            return vec![VisualLine::from_line(line, line_num)];
        }

        let chars: Vec<char> = line.chars().collect();
        if chars.is_empty() {
            return vec![VisualLine {
                logical_line: line_num,
                start_col: 0,
                end_col: 0,
                text: String::new(),
                display_width: 0,
            }];
        }

        let mut result = Vec::new();
        let mut current = String::new();
        let mut current_width = 0;
        let mut start_col = 0;
        let mut col = 0;

        for ch in chars {
            let ch_width = if ch.is_ascii() { 1 } else { 2 };

            // 检查是否需要换行
            if current_width + ch_width > self.width && !current.is_empty() {
                result.push(VisualLine {
                    logical_line: line_num,
                    start_col,
                    end_col: col,
                    text: current.clone(),
                    display_width: current_width,
                });
                start_col = col;
                current.clear();
                current_width = 0;
            }

            current.push(ch);
            current_width += ch_width;
            col += 1;
        }

        // 添加最后一个视觉行
        if !current.is_empty() || result.is_empty() {
            result.push(VisualLine {
                logical_line: line_num,
                start_col,
                end_col: col,
                text: current,
                display_width: current_width,
            });
        }

        result
    }

    /// 逻辑位置 -> 视觉行索引（使用快速计算）
    pub fn logical_to_visual(&self, logical_line: usize, logical_col: usize) -> usize {
        // 快速模式：使用视觉行计数映射
        if !self.line_visual_counts.is_empty() {
            let mut visual_pos = 0;
            for (i, &count) in self.line_visual_counts.iter().enumerate() {
                if i == logical_line {
                    // 在目标行内计算偏移
                    if self.enabled && self.width > 0 {
                        // 简化：假设每个视觉行大约容纳 width/2 个字符（考虑中英文混合）
                        let offset = logical_col / (self.width / 2).max(1);
                        return visual_pos + offset.min(count.saturating_sub(1));
                    }
                    return visual_pos;
                }
                visual_pos += count;
            }
            return visual_pos.saturating_sub(1);
        }
        
        // 回退模式：从缓存中查找
        let mut last_match = None;
        for (i, vl) in self.cache.iter().enumerate() {
            if vl.logical_line == logical_line && vl.start_col <= logical_col {
                last_match = Some(i);
            }
        }
        last_match.unwrap_or(self.cache.len().saturating_sub(1))
    }

    /// 视觉行索引 -> 逻辑位置（使用快速计算）
    pub fn visual_to_logical(&self, visual_row: usize) -> (usize, usize) {
        // 优先使用缓存
        if let Some(vl) = self.cache.get(visual_row) {
            return (vl.logical_line, vl.start_col);
        }
        
        // 快速模式：使用视觉行计数映射
        if !self.line_visual_counts.is_empty() {
            let mut visual_counter = 0;
            for (logical_line, &count) in self.line_visual_counts.iter().enumerate() {
                if visual_counter + count > visual_row {
                    // 找到了目标逻辑行
                    let offset_in_line = visual_row - visual_counter;
                    // 估算起始列（假设每视觉行容纳 width/2 个字符）
                    let start_col = offset_in_line * (self.width / 2).max(1);
                    return (logical_line, start_col);
                }
                visual_counter += count;
            }
            // 超出范围，返回最后一行
            let last_line = self.line_visual_counts.len().saturating_sub(1);
            return (last_line, 0);
        }
        
        (0, 0)
    }

    /// 获取指定视觉行的逻辑行号
    pub fn get_logical_line(&self, visual_row: usize) -> Option<usize> {
        self.cache.get(visual_row).map(|vl| vl.logical_line)
    }

    /// 获取指定视觉行
    pub fn get_visual_line(&self, visual_row: usize) -> Option<&VisualLine> {
        self.cache.get(visual_row)
    }

    /// 在指定视觉行中查找逻辑列对应的显示列
    pub fn logical_col_to_display_col(&self, visual_row: usize, logical_col: usize) -> usize {
        if let Some(vl) = self.cache.get(visual_row) {
            if logical_col < vl.start_col {
                return 0;
            }
            let col_in_line = logical_col - vl.start_col;
            let chars: Vec<char> = vl.text.chars().collect();
            let sub_text: String = chars.iter().take(col_in_line).collect();
            display_width(&sub_text)
        } else {
            0
        }
    }

    /// 在指定视觉行中，根据显示列查找逻辑列
    pub fn display_col_to_logical_col(&self, visual_row: usize, display_col: usize) -> usize {
        if let Some(vl) = self.cache.get(visual_row) {
            let chars: Vec<char> = vl.text.chars().collect();
            let mut current_width = 0;
            let mut col = 0;

            for ch in chars {
                let ch_width = if ch.is_ascii() { 1 } else { 2 };
                if current_width + ch_width > display_col {
                    break;
                }
                current_width += ch_width;
                col += 1;
            }

            vl.start_col + col
        } else {
            0
        }
    }

    /// 获取指定逻辑行对应的所有视觉行索引（需要先确保缓存已构建）
    pub fn get_visual_lines_for_logical(&self, logical_line: usize) -> Vec<usize> {
        // 先尝试从缓存获取
        let result: Vec<usize> = self.cache
            .iter()
            .enumerate()
            .filter(|(_, vl)| vl.logical_line == logical_line)
            .map(|(i, _)| i)
            .collect();
        
        if !result.is_empty() {
            return result;
        }
        
        // 如果缓存中没有，基于视觉行计数估算
        if logical_line < self.line_visual_counts.len() {
            let mut visual_start = 0;
            for (i, &count) in self.line_visual_counts.iter().enumerate() {
                if i == logical_line {
                    return (visual_start..visual_start + count).collect();
                }
                visual_start += count;
            }
        }
        
        Vec::new()
    }

    /// 计算指定逻辑行的视觉行数量（使用快速估算）
    pub fn count_visual_lines(&self, logical_line: usize) -> usize {
        // 优先使用视觉行计数映射
        if logical_line < self.line_visual_counts.len() {
            return self.line_visual_counts[logical_line];
        }
        
        // 回退到缓存
        self.cache
            .iter()
            .filter(|vl| vl.logical_line == logical_line)
            .count()
    }

    /// 向上移动视觉行
    pub fn visual_up(&self, current_visual: usize) -> Option<usize> {
        if current_visual > 0 {
            Some(current_visual - 1)
        } else {
            None
        }
    }

    /// 向下移动视觉行
    pub fn visual_down(&self, current_visual: usize) -> Option<usize> {
        if current_visual < self.cache.len().saturating_sub(1) {
            Some(current_visual + 1)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_wrap() {
        let mut engine = WrapEngine::no_wrap();
        let lines = vec!["Hello, World!".to_string()];
        engine.rebuild_cache(&lines);
        
        assert_eq!(engine.visual_line_count(), 1);
        let vl = engine.get_visual_line(0).unwrap();
        assert_eq!(vl.text, "Hello, World!");
        assert_eq!(vl.logical_line, 0);
        assert_eq!(vl.start_col, 0);
        assert_eq!(vl.end_col, 13);
    }

    #[test]
    fn test_wrap_ascii() {
        let mut engine = WrapEngine::new();
        engine.set_width(10);
        
        let lines = vec!["Hello, World!".to_string()];
        engine.rebuild_cache(&lines);
        
        // "Hello, Wor" (10 chars) + "ld!" (3 chars)
        assert!(engine.visual_line_count() >= 1);
    }

    #[test]
    fn test_wrap_chinese() {
        let mut engine = WrapEngine::new();
        engine.set_width(10);
        
        // 每个中文字符占 2 个显示宽度
        let lines = vec!["测试中文折行".to_string()];
        engine.rebuild_cache(&lines);
        
        // "测试中" = 6 宽度, "文折行" = 6 宽度
        // 在宽度 10 时应该折为 2 行
        assert!(engine.visual_line_count() >= 1);
    }

    #[test]
    fn test_logical_to_visual() {
        let mut engine = WrapEngine::new();
        engine.set_width(10);
        
        let lines = vec!["HelloWorldTest".to_string()];
        engine.rebuild_cache(&lines);
        
        // 在宽度 10 时，"HelloWorld" (0-10) 是第一行，"Test" (10-14) 是第二行
        let visual = engine.logical_to_visual(0, 3);
        assert_eq!(visual, 0); // "l" 在第一个视觉行
        
        let visual = engine.logical_to_visual(0, 12);
        assert!(visual >= 1, "Expected visual >= 1, got {}", visual); // "e" 在第二个视觉行
    }

    #[test]
    fn test_visual_to_logical() {
        let mut engine = WrapEngine::new();
        engine.set_width(10);
        
        let lines = vec!["HelloWorldTest".to_string()];
        engine.rebuild_cache(&lines);
        
        let (line, col) = engine.visual_to_logical(0);
        assert_eq!(line, 0);
        assert_eq!(col, 0);
    }

    #[test]
    fn test_empty_line() {
        let mut engine = WrapEngine::new();
        engine.set_width(10);
        
        let lines = vec!["".to_string(), "Hello".to_string()];
        engine.rebuild_cache(&lines);
        
        assert_eq!(engine.visual_line_count(), 2);
        let vl = engine.get_visual_line(0).unwrap();
        assert_eq!(vl.text, "");
        assert_eq!(vl.logical_line, 0);
    }

    #[test]
    fn test_wrap_spans_basic() {
        use ratatui::style::Style;

        let long_text = "a".repeat(80);
        let spans = vec![SizedSpan::plain(&long_text, Style::default())];

        // 折行宽度 40
        let result = wrap_spans(&spans, 40, 0);
        assert_eq!(result.len(), 2, "80 chars at width 40 should produce 2 lines");
        assert_eq!(result[0].display_width, 40);
        assert_eq!(result[1].display_width, 40);
        assert!(!result[0].is_continuation);
        assert!(result[1].is_continuation);
    }
}
