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
    /// 该视觉行起点在"渲染产物 char 序列"中的索引（用于把源码 char 偏移转
    /// 换到 `render_inline` 输出的 span 序列中的 char 偏移）。
    /// 当 wrap 按"源码字符宽"折行时，本字段 == `start_col`（每个源码 char
    /// 都对应一个渲染 char）；当按"渲染后宽度"折行时，本字段 ≤ `start_col`
    /// （标记符号在渲染产物里不占 char 位置）。
    pub visible_start_char: usize,
    /// 同上，对应 `end_col`。`visible_end_char - visible_start_char` 就是该
    /// 视觉行在 `render_inline(line)` 输出中实际占用的 char 数。
    pub visible_end_char: usize,
}

impl VisualLine {
    /// 创建不折行的视觉行
    pub fn from_line(line: &str, line_num: usize) -> Self {
        let end_col = line.chars().count();
        Self {
            logical_line: line_num,
            start_col: 0,
            end_col,
            text: line.to_string(),
            display_width: display_width(line),
            visible_start_char: 0,
            visible_end_char: end_col,
        }
    }
}

/// 代码块每行围栏占用的字符宽度：左 `│ ` (2) + ` │` (2) + 右内边距 (2) = 6。
///
/// 必须等于 `renderer/code_block.rs` 里 `CODE_BLOCK_RIGHT_PADDING` (2) + 边框 4。
/// 折行预算少一列，渲染时内容就会盖过右 `│` 把整条边框顶出屏幕（窄终端尤其明显）。
const CODE_BLOCK_FRAME_WIDTH: usize = 6;

/// 折行引擎
///
/// 使用 HashMap 稀疏缓存 + 前缀和数组实现高性能折行。
/// - `line_visual_counts`: 每个逻辑行的视觉行数量（总是完整的）
/// - `prefix_sums`: 前缀和数组，用于 O(log n) 的位置查找
/// - `line_cache`: 稀疏缓存，只为视口范围内的行存储详细 VisualLine
/// - `code_block_lines`: 每行是否在代码块内部（内容行，不含围栏行），
///   代码块内行使用更窄的折行宽度以适配边框
#[derive(Debug, Clone)]
pub struct WrapEngine {
    /// 是否启用折行
    enabled: bool,
    /// 折行宽度（全局基准）
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
    /// 每行是否在代码块内部（内容行，不含围栏行）
    code_block_lines: Vec<bool>,
    /// 表格块范围列表 `(start, end)`（闭区间）。
    /// 表格首行的 `line_visual_counts[start]` 已被设为整张表的渲染高度，
    /// 续行 `(start, end]` 的 count = 0。本字段用于让 cursor 移动逻辑识别
    /// "膨胀块"，从而执行跨块跳越（避免光标停留在续行上、卡在视觉行号断层处）。
    table_blocks: Vec<(usize, usize)>,
    /// 每行的"渲染后 per-char 显示宽度"数组：
    ///   - `Some(widths)`：该行折行宽度按 widths 累加（Markdown 标记 = 0，
    ///     可见字符 = `char_width`）。长度等于 `lines[i].chars().count()`。
    ///   - `None`：该行按源码 `char_width` 累加（兼容老行为）。
    ///
    /// 数组本身长度为 0 表示完全不启用（所有行按源码宽）。
    line_visible_widths: Vec<Option<Vec<u8>>>,
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
            code_block_lines: Vec::new(),
            table_blocks: Vec::new(),
            line_visible_widths: Vec::new(),
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

    /// 主动把缓存标记为 dirty，强制下次访问时重建。
    ///
    /// 用于外部（例如 editor 在光标换行时）触发重建：折行宽度规则与
    /// "光标行 vs 其它行"耦合，光标换行 → 这两行的视觉行数都可能变。
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// 获取视觉行总数
    pub fn visual_line_count(&self) -> usize {
        self.prefix_sums.last().copied().unwrap_or(0)
    }

    /// 重建元数据（精确的视觉行计数 + 前缀和），清空详细缓存
    pub fn rebuild_cache(&mut self, lines: &[String]) {
        self.rebuild_cache_with_code_blocks(lines, &[]);
    }

    /// 重建元数据，支持代码块行使用更窄的折行宽度。
    ///
    /// `cb_ranges` 为闭区间列表 `[(start, end), ...]`，表示代码块的内容行范围
    /// （不含围栏行 ` ``` ` 本身）。
    pub fn rebuild_cache_with_code_blocks(
        &mut self,
        lines: &[String],
        cb_ranges: &[(usize, usize)],
    ) {
        self.rebuild_cache_with_blocks(lines, cb_ranges, &[]);
    }

    /// 重建元数据，同时支持代码块（窄折行）和表格（块级渲染高度膨胀）。
    ///
    /// `table_blocks` 是 `(start_idx, end_idx, rendered_height)` 列表（闭区间）。
    /// 表格的整块渲染高度会被记到首行的 `line_visual_counts[start_idx]`，
    /// 续行（start_idx+1 ..= end_idx）记 0；这样 `prefix_sums` 与
    /// `visual_line_count()` 自动反映真实渲染坐标，光标视觉行号、滚动偏移、
    /// 视口可见范围都不再需要额外的"渲染输出 vs 源码视觉"补正。
    pub fn rebuild_cache_with_blocks(
        &mut self,
        lines: &[String],
        cb_ranges: &[(usize, usize)],
        table_blocks: &[(usize, usize, usize)],
    ) {
        // 兼容入口：不传 per-line 渲染宽度，等价于所有行按源码 char_width 折行
        self.rebuild_cache_with_blocks_and_widths(lines, cb_ranges, table_blocks, &[]);
    }

    /// 重建元数据，支持代码块、表格，以及"按渲染后显示宽度"折行。
    ///
    /// `line_visible_widths`：可选切片，长度 0 表示所有行按源码 `char_width` 算
    /// （兼容老行为）；否则长度应等于 `lines.len()`，每个元素：
    ///   - `Some(per_char_widths)`：该行用此 per-char 宽度数组累加（每个源码
    ///     char 渲染后占多少列；Markdown 标记符号 = 0）。
    ///   - `None`：该行按源码 `char_width` 累加（代码块、光标行、表格行等
    ///     不参与 inline 渲染宽度补偿的行）。
    pub fn rebuild_cache_with_blocks_and_widths(
        &mut self,
        lines: &[String],
        cb_ranges: &[(usize, usize)],
        table_blocks: &[(usize, usize, usize)],
        line_visible_widths: &[Option<Vec<u8>>],
    ) {
        self.line_cache.clear();
        self.line_visual_counts.clear();
        self.prefix_sums.clear();
        self.table_blocks.clear();

        // 构建每行是否在代码块内的 bitmap
        self.code_block_lines = vec![false; lines.len()];
        for &(start, end) in cb_ranges {
            for i in start..=end.min(lines.len().saturating_sub(1)) {
                self.code_block_lines[i] = true;
            }
        }

        // 构建表格信息：line -> (block_role, height)
        // role: 0 = 非表格, 1 = 表格首行（visual count = height）, 2 = 表格续行（visual count = 0）
        // 用 Option<usize> 表示表格首行的高度；None 表示非首行（如果在表格内则视为续行）
        let mut table_first_row_height: Vec<Option<usize>> = vec![None; lines.len()];
        let mut table_continuation: Vec<bool> = vec![false; lines.len()];
        for &(start, end, height) in table_blocks {
            if start < lines.len() {
                table_first_row_height[start] = Some(height);
                self.table_blocks
                    .push((start, end.min(lines.len().saturating_sub(1))));
            }
            let cont_end = end.min(lines.len().saturating_sub(1));
            if start < cont_end {
                for slot in &mut table_continuation[(start + 1)..=cont_end] {
                    *slot = true;
                }
            }
        }
        // 保证 table_blocks 按 start 升序，便于二分
        self.table_blocks.sort_by_key(|&(s, _)| s);

        self.line_visual_counts.reserve(lines.len());
        self.prefix_sums.reserve(lines.len() + 1);
        self.prefix_sums.push(0);

        let mut sum: usize = 0;
        for (i, line) in lines.iter().enumerate() {
            let count = if let Some(h) = table_first_row_height[i] {
                // 表格首行：吃掉整张表的渲染高度
                h
            } else if table_continuation[i] {
                // 表格续行：不贡献新视觉行
                0
            } else {
                let w = self.effective_width(i);
                let widths_for_line: Option<&[u8]> =
                    line_visible_widths.get(i).and_then(|opt| opt.as_deref());
                Self::compute_visual_line_count(line, widths_for_line, w, self.enabled)
            };
            self.line_visual_counts.push(count);
            sum += count;
            self.prefix_sums.push(sum);
        }

        // 保留 widths 以便 build_range 时构建精确缓存使用相同累加规则
        self.line_visible_widths = if line_visible_widths.is_empty() {
            Vec::new()
        } else {
            line_visible_widths.to_vec()
        };

        self.dirty = false;
    }

    /// 获取指定行的有效折行宽度（代码块内行减去围栏 + 右内边距宽度）
    fn effective_width(&self, line_idx: usize) -> usize {
        if self.code_block_lines.get(line_idx) == Some(&true) {
            self.width.saturating_sub(CODE_BLOCK_FRAME_WIDTH).max(10)
        } else {
            self.width
        }
    }

    /// 精确计算一个逻辑行的视觉行数量（指定宽度，可选 per-char 渲染宽度）。
    ///
    /// `visible_widths`:
    ///   - `Some(widths)`：按渲染后宽度累加（Markdown 标记符号 = 0），
    ///     widths.len() 应等于 `line.chars().count()`，超出长度的 char fallback 到
    ///     `char_width`；
    ///   - `None`：按源码 `char_width(ch)` 累加（兼容老行为）。
    fn compute_visual_line_count(
        line: &str,
        visible_widths: Option<&[u8]>,
        width: usize,
        enabled: bool,
    ) -> usize {
        if !enabled {
            return 1;
        }
        let chars: Vec<char> = line.chars().collect();
        if chars.is_empty() {
            return 1;
        }
        let mut count: usize = 1;
        let mut current_width: usize = 0;
        for (idx, ch) in chars.iter().enumerate() {
            let ch_width = char_width_for(*ch, visible_widths, idx);
            if current_width + ch_width > width && current_width > 0 {
                count += 1;
                current_width = 0;
            }
            current_width += ch_width;
        }
        count
    }

    /// 兼容旧名：按源码字符宽算视觉行数。
    #[allow(dead_code)]
    fn compute_visual_line_count_with_width(line: &str, width: usize, enabled: bool) -> usize {
        Self::compute_visual_line_count(line, None, width, enabled)
    }

    /// 为指定范围的逻辑行构建详细视觉行缓存（只构建未缓存的行）
    pub fn build_range(&mut self, lines: &[String], start: usize, end: usize) {
        let end = end.min(lines.len());
        for (i, line) in lines.iter().enumerate().skip(start).take(end - start) {
            if !self.line_cache.contains_key(&i) {
                let w = self.effective_width(i);
                let widths = self
                    .line_visible_widths
                    .get(i)
                    .and_then(|opt| opt.as_deref());
                let vlines = Self::wrap_line_inner(line, i, widths, w, self.enabled);
                self.line_cache.insert(i, vlines);
            }
        }
    }

    /// 将逻辑行拆分为视觉行（使用行级折行宽度）
    #[allow(dead_code)]
    pub fn wrap_line(&self, line: &str, line_num: usize) -> Vec<VisualLine> {
        let w = self.effective_width(line_num);
        let widths = self
            .line_visible_widths
            .get(line_num)
            .and_then(|opt| opt.as_deref());
        Self::wrap_line_inner(line, line_num, widths, w, self.enabled)
    }

    /// 将逻辑行按指定宽度拆分为视觉行（支持可选 per-char 渲染宽度）。
    ///
    /// `visible_widths` 语义与 [`compute_visual_line_count`] 一致：`Some` 表示
    /// 按 Markdown 渲染后宽度累加（标记符号 = 0），`None` 表示按源码字符宽。
    ///
    /// 注意：折行点仍然落在源码 char 边界上，`start_col`/`end_col` 仍是源码
    /// 字符索引，光标 / 选区 / 鼠标定位的契约不变。`display_width` 记录的是
    /// 实际累加宽度（即用同一规则下当前视觉行的"显示宽度"），用于渲染端
    /// 对齐右边框。
    fn wrap_line_inner(
        line: &str,
        line_num: usize,
        visible_widths: Option<&[u8]>,
        width: usize,
        enabled: bool,
    ) -> Vec<VisualLine> {
        if !enabled {
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
                visible_start_char: 0,
                visible_end_char: 0,
            }];
        }

        let mut result = Vec::new();
        let mut current = String::new();
        let mut current_width = 0;
        let mut start_col = 0;
        let mut col = 0;
        // 跟踪"渲染产物 char 序列"中的位置：
        // - 当 visible_widths == None：每个源码 char 都是一个渲染 char（visible_pos == col）
        // - 当 visible_widths == Some：仅 visible_widths[idx] > 0 的源码 char 在渲染产物里占位
        let mut visible_pos: usize = 0;
        let mut visible_start: usize = 0;

        for (idx, ch) in chars.iter().enumerate() {
            let ch_width = char_width_for(*ch, visible_widths, idx);

            if current_width + ch_width > width && !current.is_empty() {
                result.push(VisualLine {
                    logical_line: line_num,
                    start_col,
                    end_col: col,
                    text: current.clone(),
                    display_width: current_width,
                    visible_start_char: visible_start,
                    visible_end_char: visible_pos,
                });
                start_col = col;
                visible_start = visible_pos;
                current.clear();
                current_width = 0;
            }

            current.push(*ch);
            current_width += ch_width;
            col += 1;
            // 渲染端 char index 推进：
            // 有 widths 时，仅当该 char 在渲染产物里可见（width > 0 或被解析为可见正文）
            // 才占一个 char；无 widths 时（按源码宽折行），每个源码 char 都对应一个渲染 char。
            if visible_widths.is_some() {
                if ch_width > 0 {
                    visible_pos += 1;
                }
            } else {
                visible_pos += 1;
            }
        }

        if !current.is_empty() || result.is_empty() {
            result.push(VisualLine {
                logical_line: line_num,
                start_col,
                end_col: col,
                text: current,
                display_width: current_width,
                visible_start_char: visible_start,
                visible_end_char: visible_pos,
            });
        }

        result
    }

    /// 兼容旧名：按源码字符宽折行。
    #[allow(dead_code)]
    fn wrap_line_with_width(
        line: &str,
        line_num: usize,
        width: usize,
        enabled: bool,
    ) -> Vec<VisualLine> {
        Self::wrap_line_inner(line, line_num, None, width, enabled)
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

        // 估算：基于列位置和行级有效宽度
        let w = self.effective_width(logical_line);
        let sub = logical_col / w.max(1);
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
        let w = self.effective_width(logical);
        let start_col = sub * w;
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

    /// 检查指定逻辑行是否在代码块内部
    pub fn is_code_block_line(&self, line_idx: usize) -> bool {
        self.code_block_lines.get(line_idx) == Some(&true)
    }

    /// 获取指定逻辑行的有效折行宽度
    #[allow(dead_code)]
    pub fn line_wrap_width(&self, line_idx: usize) -> usize {
        self.effective_width(line_idx)
    }

    /// 如果 `logical_line` 落在某个表格块内，返回该块的 `(start, end)`（闭区间）。
    pub fn table_block_for_line(&self, logical_line: usize) -> Option<(usize, usize)> {
        // table_blocks 按 start 升序；二分找最后一个 start <= logical_line。
        let pos = self
            .table_blocks
            .partition_point(|&(s, _)| s <= logical_line);
        if pos == 0 {
            return None;
        }
        let (s, e) = self.table_blocks[pos - 1];
        if logical_line >= s && logical_line <= e {
            Some((s, e))
        } else {
            None
        }
    }

    /// 如果某个视觉行号 `visual_row` 落在某个膨胀表格块的内部（即不是首行那一格、
    /// 没有对应缓存的 vline），返回该块的 `(start, end)`。
    ///
    /// 用于 cursor 移动逻辑识别"目标视觉行落进了膨胀区"的情形。
    pub fn table_block_for_visual_row(&self, visual_row: usize) -> Option<(usize, usize)> {
        for &(start, end) in &self.table_blocks {
            let block_start = self.prefix_sums.get(start).copied()?;
            let block_end_exclusive = self
                .prefix_sums
                .get(end + 1)
                .copied()
                .unwrap_or_else(|| self.visual_line_count());
            if visual_row >= block_start && visual_row < block_end_exclusive {
                return Some((start, end));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests;

/// 取第 `idx` 个 char 在折行累加时的"等效宽度"：
///   - 若 `visible_widths` 提供了对应位置：取该值（u8 → usize，Markdown 标记 = 0）
///   - 否则回退到 `char_width(ch)`（CJK 等宽字符按显示宽算）
#[inline]
fn char_width_for(ch: char, visible_widths: Option<&[u8]>, idx: usize) -> usize {
    if let Some(ws) = visible_widths
        && let Some(w) = ws.get(idx)
    {
        return *w as usize;
    }
    char_width(ch)
}
