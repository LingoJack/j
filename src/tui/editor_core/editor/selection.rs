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
