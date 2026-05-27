//! Visual 选区辅助函数

/// 渲染元数据（记录每个已渲染视觉行对应的逻辑行号和起止列）。
#[derive(Clone)]
pub struct RenderedVL {
    pub logical_line: usize,
    pub start_col: usize,
    pub end_col: usize,
}

/// 计算视觉行与选区的交集字符范围。
///
/// 选区语义：character-wise visual mode 的两端均闭合——光标块所在的字符也算选中。
/// 调用前请先用 `inclusive_end_col(buffer, er, ec)` 把 `ec` 扩展为半开区间右端
/// （这里参数 `ec` 已经是扩展后的值），这样高亮范围与 `get_selection_text`
/// 复制到剪贴板的内容一致。
///
/// 返回 `(hl_start, hl_end)`——需要高亮的逻辑列范围（半开区间）。
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

/// 把"光标列 `ec`"扩展为"半开区间右端"——也就是把光标字符纳入选区。
///
/// 用于在调用 `visual_line_selection_range` / `get_selection_text` 之前
/// 把 raw cursor col 转成 inclusive end col。当 `ec >= line_len`（光标越过
/// 行末）时不再加 1，避免越界。
pub(super) fn inclusive_end_col(line_len: usize, ec: usize) -> usize {
    if ec >= line_len { ec } else { ec + 1 }
}
