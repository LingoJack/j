//! Visual 选区辅助函数

/// 渲染元数据（记录每个已渲染视觉行对应的逻辑行号和起止列）。
#[derive(Clone)]
pub struct RenderedVL {
    pub logical_line: usize,
    pub start_col: usize,
    pub end_col: usize,
}

/// 计算视觉行与选区 `[sr,sc)-(er,ec)` 的交集字符范围。
///
/// 返回 `(hl_start, hl_end)`——需要高亮的逻辑列范围（闭区间左、开区间右）。
/// 若无交集，返回 `(0, 0)`。
pub(super) fn visual_line_selection_range(
    meta: &RenderedVL,
    sr: usize,
    sc: usize,
    er: usize,
    ec: usize,
) -> (usize, usize) {
    let ll = meta.logical_line;
    let vl_start = meta.start_col;
    let vl_end = meta.end_col;

    // 逻辑行完全在选区中间 → 整个视觉行都高亮
    if ll > sr && ll < er {
        return (vl_start, vl_end);
    }

    // 起始行 == 结束行：视觉行与 [sc, ec) 求交集
    if ll == sr && ll == er {
        let hl_start = vl_start.max(sc);
        let hl_end = vl_end.min(ec);
        return (hl_start, hl_end);
    }

    // 仅起始行：高亮 [sc, ∞) ∩ 视觉行范围
    if ll == sr {
        let hl_start = vl_start.max(sc);
        let hl_end = vl_end;
        if hl_start < vl_end {
            return (hl_start, hl_end);
        }
        return (0, 0);
    }

    // 仅结束行：高亮 [0, ec) ∩ 视觉行范围
    if ll == er {
        let hl_start = vl_start;
        let hl_end = vl_end.min(ec);
        if vl_start < hl_end {
            return (hl_start, hl_end);
        }
        return (0, 0);
    }

    (0, 0)
}

/// 如果渲染窗口落在表格内部，扩展到整张表的源码范围。
///
/// 表格渲染不是逐行进行的，而是由表格首行一次性产出整张表；
/// 如果窗口只覆盖到表格中后段源码行，缺少首行时就拿不到任何表格输出。
pub(super) fn expand_render_range_for_tables(
    lines: &[String],
    render_start: usize,
    render_end: usize,
) -> (usize, usize) {
    if lines.is_empty() || render_start >= render_end {
        return (render_start, render_end);
    }

    let scan_end = render_end.min(lines.len());
    let mut expanded_start = render_start.min(lines.len().saturating_sub(1));
    let mut expanded_end = scan_end;

    for line_idx in render_start..scan_end {
        if let Some((table_start, table_end)) = find_table_range_in_lines(lines, line_idx) {
            expanded_start = expanded_start.min(table_start);
            expanded_end = expanded_end.max(table_end + 1);
        }
    }

    (expanded_start, expanded_end.min(lines.len()))
}

fn find_table_range_in_lines(lines: &[String], line_idx: usize) -> Option<(usize, usize)> {
    let line = lines.get(line_idx)?;
    if !is_table_row(line) {
        return None;
    }

    let mut start_idx = line_idx;
    while start_idx > 0
        && lines
            .get(start_idx - 1)
            .is_some_and(|line| is_table_row(line))
    {
        start_idx -= 1;
    }

    let mut end_idx = line_idx;
    while end_idx + 1 < lines.len()
        && lines
            .get(end_idx + 1)
            .is_some_and(|line| is_table_row(line))
    {
        end_idx += 1;
    }

    if end_idx.saturating_sub(start_idx) < 1 {
        return None;
    }

    Some((start_idx, end_idx))
}

fn is_table_row(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with('|') && trimmed.ends_with('|') && trimmed.contains('|')
}

/// 将连续的重复 visual_map 映射重新分布到实际渲染输出中。
///
/// 表格这类块会出现"源码侧多个视觉行，对应渲染侧一整段输出"的情况：
/// 首个源码视觉行产出整张表，后续源码视觉行输出为空，于是它们在 `visual_map`
/// 中都会指向同一个起点。若不重分布，滚动会卡在块首，只能依赖额外特判跳到块尾。
///
/// 这里按视口高度把这段起点分散到 `[block_start, block_end - viewport_height]`，
/// 让滚动能覆盖整段实际渲染内容，最后一个槽位稳定落在块尾可见位置。
pub(super) fn redistribute_visual_map_for_expanded_blocks(
    visual_map: &mut [usize],
    rendered_line_count: usize,
    viewport_height: usize,
) {
    if visual_map.is_empty() || viewport_height == 0 {
        return;
    }

    let mut run_start = 0;
    while run_start < visual_map.len() {
        let block_start = visual_map[run_start];
        let mut run_end = run_start + 1;
        while run_end < visual_map.len() && visual_map[run_end] == block_start {
            run_end += 1;
        }

        let run_len = run_end - run_start;
        let block_end = if run_end < visual_map.len() {
            visual_map[run_end]
        } else {
            rendered_line_count
        };
        let block_height = block_end.saturating_sub(block_start);
        let max_visible_start = block_height.saturating_sub(viewport_height);

        if run_len > 1 && max_visible_start > 0 {
            let denominator = run_len - 1;
            for (offset, slot) in visual_map[run_start..run_end].iter_mut().enumerate() {
                let distributed = offset * max_visible_start / denominator;
                *slot = block_start + distributed;
            }
        }

        run_start = run_end;
    }
}
