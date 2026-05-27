//! Span 选区高亮工具
//!
//! 提供精确到字符级别的选区高亮功能，供 Markdown 编辑器和 Chat UI 复用。

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use crate::util::text::char_width;

/// 归一化选区起点和终点，确保 start <= end。
///
/// 返回 `((sr, sc), (er, ec))`，其中 `sr <= er`，且当 `sr == er` 时 `sc <= ec`。
pub fn normalize_selection(
    anchor: (usize, usize),
    current: (usize, usize),
) -> ((usize, usize), (usize, usize)) {
    if anchor.0 < current.0 || (anchor.0 == current.0 && anchor.1 <= current.1) {
        (anchor, current)
    } else {
        (current, anchor)
    }
}

/// 计算某行与选区的交集字符范围（简化版，无视觉行折行概念）。
///
/// 适用于 Chat UI 的扁平全局行号体系。
///
/// 返回 `(start, end)`，若无交集返回 `(0, 0)`。
/// 当 `end == usize::MAX` 时表示高亮到行尾。
pub fn compute_line_selection_range(
    line_idx: usize,
    anchor: (usize, usize),
    current: (usize, usize),
) -> (usize, usize) {
    let ((sr, sc), (er, ec)) = normalize_selection(anchor, current);

    if line_idx < sr || line_idx > er {
        return (0, 0); // 无交集
    }

    let start = if line_idx == sr { sc } else { 0 };
    let end = if line_idx == er { ec } else { usize::MAX };

    (start, end)
}

/// 选区样式上下文，用于减少辅助函数的参数数量。
pub(crate) struct SelectionStyle {
    normal: Style,
    selected: Style,
    local_start: usize,
    local_end: usize,
}

/// 对已渲染的 spans 列表应用选区高亮（精确到字符级别）。
///
/// # 参数
///
/// - `spans`: 原始 spans 列表
/// - `skip_chars`: 开头跳过的字符数（如 Markdown 编辑器的行号，Chat UI 传 0）
/// - `local_start` / `local_end`: 内容部分的字符偏移（0-based, exclusive end）
/// - `sel_fg` / `sel_bg`: 选区的文字色和背景色
pub fn rebuild_spans_with_selection(
    spans: &[Span<'static>],
    skip_chars: usize,
    local_start: usize,
    local_end: usize,
    sel_fg: Color,
    sel_bg: Color,
) -> Vec<Span<'static>> {
    let ss = SelectionStyle {
        normal: Style::default(),
        selected: Style::default().fg(sel_fg).bg(sel_bg),
        local_start,
        local_end,
    };
    let mut result = Vec::with_capacity(spans.len() + 4);
    let mut chars_seen = 0usize;

    for span in spans {
        let span_chars: Vec<char> = span.content.chars().collect();
        let span_len = span_chars.len();
        let span_end = chars_seen + span_len;

        // 跳过 skip_chars 区域（如行号）
        if span_end <= skip_chars {
            result.push(span.clone());
            chars_seen = span_end;
            continue;
        }

        // 当前 span 跨越 skip_chars 边界，需分割
        if chars_seen < skip_chars && span_end > skip_chars {
            let skip_part_len = skip_chars - chars_seen;
            let skip_text: String = span_chars[..skip_part_len].iter().collect();
            result.push(Span::styled(skip_text, span.style));

            // 剩余内容作为新 span 处理
            let content_chars = &span_chars[skip_part_len..];
            let content_len = content_chars.len();
            // 相对于内容起始的偏移（内容部分从 0 开始计算）
            let c_start = 0usize;
            let c_end = content_len;
            let content_ss = SelectionStyle {
                normal: span.style,
                ..ss
            };
            append_content_spans(content_chars, c_start, c_end, &content_ss, &mut result);
            chars_seen = span_end;
            continue;
        }

        // 纯内容 span
        let content_offset = chars_seen - skip_chars;
        let c_start = content_offset;
        let c_end = content_offset + span_len;
        let content_ss = SelectionStyle {
            normal: span.style,
            ..ss
        };
        append_content_spans(&span_chars, c_start, c_end, &content_ss, &mut result);
        chars_seen = span_end;
    }

    result
}

/// 将内容 span 按 `[local_start, local_end)` 选区范围分割并附加到 result。
fn append_content_spans(
    chars: &[char],
    c_start: usize,
    c_end: usize,
    ss: &SelectionStyle,
    result: &mut Vec<Span<'static>>,
) {
    let SelectionStyle {
        normal,
        selected,
        local_start,
        local_end,
    } = *ss;

    // 无交集
    if c_end <= local_start || c_start >= local_end {
        let text: String = chars.iter().collect();
        result.push(Span::styled(text, normal));
        return;
    }

    // 选中前的部分
    if c_start < local_start {
        let before_len = local_start - c_start;
        let text: String = chars[..before_len].iter().collect();
        result.push(Span::styled(text, normal));
    }

    // 选中的部分
    {
        let sel_begin = local_start.saturating_sub(c_start);
        let sel_finish = local_end.min(c_end).saturating_sub(c_start);
        if sel_begin < sel_finish && sel_finish <= chars.len() {
            let text: String = chars[sel_begin..sel_finish].iter().collect();
            result.push(Span::styled(text, selected));
        }
    }

    // 选中后的部分
    if c_end > local_end {
        let after_begin = local_end.saturating_sub(c_start);
        if after_begin < chars.len() {
            let text: String = chars[after_begin..].iter().collect();
            result.push(Span::styled(text, normal));
        }
    }
}

// ========== 渲染 spans → 可见内容提取 ==========
//
// 以下三个工具最早在 chat UI 内部使用，现在编辑器的鼠标选区复制也用同一套——
// 把屏幕上看到的字（不含边框、padding、链接 markdown 语法等装饰）复制下来。

/// 判断一行渲染输出是否"可选中"。
///
/// 用于跳过纯边框行、空行等非内容行。逻辑：
///   - 空 trim 视为不可选
///   - 全为空格 + box-drawing 字符（含表格 / 代码块边框）视为不可选
///
/// 注意：表格的**数据行**（如 `│ col1 │ col2 │`）含字母/数字字符，不会被
/// 误判为不可选，因此选中表格内容仍然可行。
pub fn is_selectable_line(line: &Line<'static>) -> bool {
    let full_text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
    if full_text.trim().is_empty() {
        return false;
    }
    let trimmed = full_text.trim();
    if trimmed
        .chars()
        .all(|c| "╭╮╰╯│─┌┐└┘┬┴┼┤├".contains(c) || c == ' ')
    {
        return false;
    }
    true
}

/// 判断一个 span 是否是"装饰性"的——不算可见内容。
///
/// 当前规则：
///   - 图片占位标记（以 `\x00IMG:` 开头）
///   - 纯空格（padding / 行号区域）
///   - 纯 box-drawing 字符（边框，含代码块 `│`、表格 `│┌─┐` 等）
///
/// 复制时这些 span 会被跳过，只留可见正文。
pub fn is_decorative_span(span: &Span<'static>) -> bool {
    let content = span.content.as_ref();
    if content.starts_with("\x00IMG:") {
        return true;
    }
    if content.chars().all(|c| c == ' ') {
        return true;
    }
    if content.chars().all(|c| "╭╮╰╯│─┌┐└┘┬┴┼┤├".contains(c)) {
        return true;
    }
    false
}

/// 从渲染行中提取"可见内容文本 + 内容在渲染行中的起始字符偏移"。
///
/// 装饰区分两段：
///   - 内容**之前**的装饰（行号、左侧边框、padding）→ `content_start_offset` 累加
///   - 内容**之后**的装饰（右侧边框、padding）→ 直接丢弃
///
/// 返回 `(content_text, content_start_offset)`，其中 `content_start_offset`
/// 用于把"渲染行字符偏移"还原成"内容字符偏移"：
/// `content_offset = render_offset - content_start_offset`
pub fn extract_content_from_line(line: &Line<'static>) -> (String, usize) {
    let mut content = String::new();
    let mut content_start_offset = 0usize;
    let mut in_content = false;

    for span in &line.spans {
        let span_chars = span.content.chars().count();
        if is_decorative_span(span) {
            if !in_content {
                content_start_offset += span_chars;
            }
            // 内容之后的装饰，忽略
        } else {
            in_content = true;
            content.push_str(span.content.as_ref());
        }
    }

    (content, content_start_offset)
}

/// 给定一组渲染行（Line）和 spans，把屏幕 X 列号（按显示宽度）映射为
/// "渲染行内的字符偏移"——即 `line.spans` 拼接后的第几个字符。
///
/// 用于鼠标点击：屏幕 X → 渲染行内字符位置。
pub fn spans_to_char_offset(spans: &[Span<'static>], screen_col: usize) -> usize {
    let mut acc_width = 0usize;
    let mut char_offset = 0usize;

    for span in spans {
        for ch in span.content.chars() {
            if acc_width >= screen_col {
                return char_offset;
            }
            acc_width += char_width(ch);
            char_offset += 1;
        }
    }
    char_offset
}

/// 从渲染行按"渲染坐标的字符偏移"切出可见正文，跳过装饰 span。
///
/// 这是 `extract_selection_text` 的单行实现：相比 `extract_content_from_line`
/// 假设"装饰只在最前面一段连续出现"，本函数支持装饰可以分布在行内任意位置
/// （例如代码块的 `[行号][│][ ][正文][ ][│]`、表格的 `│ col1 │ col2 │` 等）。
///
/// 实现思路：按字符遍历每个 span。对每个字符：
///   - 当前 span 是装饰的 → 跳过，但 render 位移仍然 +1
///   - 当前 span 是正文 → 如果 render 位移落在 [start, end) 内就纳入结果
///
/// 这样无论装饰在前 / 中 / 后都能正确扣除。
fn extract_visible_chars_in_range(
    line: &Line<'static>,
    render_start: usize,
    render_end: usize,
    skip_prefix_chars: usize,
) -> String {
    let mut out = String::new();
    let mut render_pos: usize = 0;

    for span in &line.spans {
        let is_deco = is_decorative_span(span);
        for ch in span.content.chars() {
            // 跳过 skip_prefix_chars 前缀（用于编辑器行号 gutter——即使行号
            // 内含数字不被 is_decorative_span 识别，调用方也能强制跳过）
            let in_skip_prefix = render_pos < skip_prefix_chars;
            // 装饰字符 / skip 区里的字符都不计入正文
            if !is_deco && !in_skip_prefix && render_pos >= render_start && render_pos < render_end
            {
                out.push(ch);
            }
            render_pos += 1;
        }
    }
    out
}

/// 根据"渲染坐标"选区从一组渲染行中提取可见内容，行间用 `\n` 拼接。
///
/// 参数：
/// - `lines`: 渲染行切片；`anchor.0` / `current.0` 是相对该切片的局部下标。
/// - `anchor` / `current`: `(渲染行号, 渲染行内字符偏移)`。
/// - `skip_prefix_chars`: 每行起始处需要无视的装饰字符数（编辑器行号 gutter）。
///
/// 表格 / 代码块 / 链接拖选都可以走这个函数：边框 / padding / 链接的
/// `[`、`](url)` 都是装饰 span，不会被复制。
pub fn extract_selection_text(
    lines: &[Line<'static>],
    anchor: (usize, usize),
    current: (usize, usize),
    skip_prefix_chars: usize,
) -> String {
    let ((sr, sc), (er, ec)) = normalize_selection(anchor, current);
    let mut result = String::new();

    for gline in sr..=er {
        let line = match lines.get(gline) {
            Some(l) => l,
            None => continue,
        };
        if !is_selectable_line(line) {
            continue;
        }

        let render_start = if gline == sr { sc } else { 0 };
        let render_end = if gline == er { ec } else { usize::MAX };
        let slice =
            extract_visible_chars_in_range(line, render_start, render_end, skip_prefix_chars);
        if slice.is_empty() {
            continue;
        }
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str(&slice);
    }

    result
}
