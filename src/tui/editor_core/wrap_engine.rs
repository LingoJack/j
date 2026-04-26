//! 折行引擎
//!
//! 将逻辑行转换为视觉行，支持自动折行功能。

use crate::util::text::{char_width, display_width};

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

/// 折行引擎
///
/// 使用 HashMap 稀疏缓存 + 前缀和数组实现高性能折行。
/// - `line_visual_counts`: 每个逻辑行的视觉行数量（总是完整的）
/// - `prefix_sums`: 前缀和数组，用于 O(log n) 的位置查找
/// - `line_cache`: 稀疏缓存，只为视口范围内的行存储详细 VisualLine
#[derive(Debug, Clone)]
pub struct WrapEngine {
    /// 是否启用折行
    enabled: bool,
    /// 折行宽度
    width: usize,
    /// 稀疏视觉行缓存：逻辑行号 -> Vec<VisualLine>
    line_cache: std::collections::HashMap<usize, Vec<VisualLine>>,
    /// 每个逻辑行的视觉行数量（总是完整的）
    line_visual_counts: Vec<usize>,
    /// 前缀和：prefix_sums[i] = line_visual_counts[0..i] 之和
    /// prefix_sums.len() == line_visual_counts.len() + 1
    /// prefix_sums[0] = 0, prefix_sums[n] = 总视觉行数
    prefix_sums: Vec<usize>,
    /// 缓存是否需要更新
    dirty: bool,
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
            line_cache: std::collections::HashMap::new(),
            line_visual_counts: Vec::new(),
            prefix_sums: vec![0],
            dirty: true,
        }
    }

    /// 是否启用折行
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// 设置是否启用折行
    pub fn set_enabled(&mut self, enabled: bool) {
        if self.enabled != enabled {
            self.enabled = enabled;
            self.dirty = true;
        }
    }

    /// 设置折行宽度
    pub fn set_width(&mut self, width: usize) {
        let width = width.max(10);
        if self.width != width {
            self.width = width;
            self.dirty = true;
        }
    }

    /// 检查缓存是否需要更新
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// 获取视觉行总数
    pub fn visual_line_count(&self) -> usize {
        self.prefix_sums.last().copied().unwrap_or(0)
    }

    /// 重建元数据（精确的视觉行计数 + 前缀和），清空详细缓存
    pub fn rebuild_cache(&mut self, lines: &[String]) {
        self.line_cache.clear();
        self.line_visual_counts.clear();
        self.prefix_sums.clear();

        self.line_visual_counts.reserve(lines.len());
        self.prefix_sums.reserve(lines.len() + 1);
        self.prefix_sums.push(0);

        let mut sum: usize = 0;
        for line in lines {
            let count = self.compute_visual_line_count(line);
            self.line_visual_counts.push(count);
            sum += count;
            self.prefix_sums.push(sum);
        }

        self.dirty = false;
    }

    /// 精确计算一个逻辑行的视觉行数量（与 wrap_line 算法一致）
    fn compute_visual_line_count(&self, line: &str) -> usize {
        if !self.enabled {
            return 1;
        }
        let chars: Vec<char> = line.chars().collect();
        if chars.is_empty() {
            return 1;
        }
        let mut count: usize = 1;
        let mut current_width: usize = 0;
        for ch in &chars {
            let ch_width = char_width(*ch);
            if current_width + ch_width > self.width && current_width > 0 {
                count += 1;
                current_width = 0;
            }
            current_width += ch_width;
        }
        count
    }

    /// 为指定范围的逻辑行构建详细视觉行缓存（只构建未缓存的行）
    pub fn build_range(&mut self, lines: &[String], start: usize, end: usize) {
        let end = end.min(lines.len());
        for (i, line) in lines.iter().enumerate().skip(start).take(end - start) {
            if !self.line_cache.contains_key(&i) {
                let vlines = self.wrap_line(line, i);
                self.line_cache.insert(i, vlines);
            }
        }
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
            let ch_width = char_width(ch);

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

    /// 通过二分查找将视觉行号映射到逻辑行号（O(log n)）
    fn visual_to_logical_line(&self, visual_row: usize) -> usize {
        if self.prefix_sums.len() <= 1 {
            return 0;
        }
        let max_logical = self.line_visual_counts.len().saturating_sub(1);
        match self.prefix_sums.binary_search(&visual_row) {
            Ok(i) => i.min(max_logical),
            Err(i) => i.saturating_sub(1).min(max_logical),
        }
    }

    /// 逻辑位置 -> 视觉行索引（O(log n) 或 O(1)）
    pub fn logical_to_visual(&self, logical_line: usize, logical_col: usize) -> usize {
        if logical_line >= self.line_visual_counts.len() {
            return self.visual_line_count().saturating_sub(1);
        }

        let base = self.prefix_sums[logical_line];

        if !self.enabled || self.width == 0 {
            return base;
        }

        let count = self.line_visual_counts[logical_line];
        if count <= 1 {
            return base;
        }

        // 优先使用精确缓存
        if let Some(vlines) = self.line_cache.get(&logical_line) {
            for (i, vl) in vlines.iter().enumerate() {
                if logical_col < vl.end_col || i == vlines.len() - 1 {
                    return base + i;
                }
            }
            return base + vlines.len().saturating_sub(1);
        }

        // 估算：基于列位置和宽度
        let sub = logical_col / self.width.max(1);
        base + sub.min(count.saturating_sub(1))
    }

    /// 视觉行索引 -> 逻辑位置（O(log n)）
    pub fn visual_to_logical(&self, visual_row: usize) -> (usize, usize) {
        let logical = self.visual_to_logical_line(visual_row);
        let base = self.prefix_sums.get(logical).copied().unwrap_or(0);
        let sub = visual_row.saturating_sub(base);

        // 使用精确缓存
        if let Some(vlines) = self.line_cache.get(&logical)
            && let Some(vl) = vlines.get(sub)
        {
            return (logical, vl.start_col);
        }

        // 估算 start_col
        let start_col = sub * self.width;
        (logical, start_col)
    }

    /// 获取指定视觉行（需要先 build_range 构建缓存）
    pub fn get_visual_line(&self, visual_row: usize) -> Option<&VisualLine> {
        let logical = self.visual_to_logical_line(visual_row);
        let base = self.prefix_sums.get(logical).copied().unwrap_or(0);
        let sub = visual_row.saturating_sub(base);
        self.line_cache.get(&logical)?.get(sub)
    }

    /// 获取指定逻辑行的缓存视觉行（返回切片引用）
    pub fn get_cached_lines(&self, logical_line: usize) -> &[VisualLine] {
        self.line_cache
            .get(&logical_line)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// 获取指定逻辑行在前缀和中的视觉偏移（O(1)）
    pub fn visual_offset_of(&self, logical_line: usize) -> usize {
        self.prefix_sums.get(logical_line).copied().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests;
