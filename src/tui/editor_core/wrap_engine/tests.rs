use super::*;

#[test]
fn test_wrap_ascii() {
    let mut engine = WrapEngine::new();
    engine.set_width(10);

    let lines = vec!["Hello, World!".to_string()];
    engine.rebuild_cache(&lines);

    // "Hello, Wor" (10 chars) + "ld!" (3 chars) = 13 display width
    // ceil(13/10) = 2
    assert_eq!(engine.visual_line_count(), 2);
}

#[test]
fn test_wrap_chinese() {
    let mut engine = WrapEngine::new();
    engine.set_width(10);

    // 每个中文字符占 2 个显示宽度，6 chars = 12 display width
    let lines = vec!["测试中文折行".to_string()];
    engine.rebuild_cache(&lines);

    // ceil(12/10) = 2
    assert_eq!(engine.visual_line_count(), 2);
}

#[test]
fn test_logical_to_visual() {
    let mut engine = WrapEngine::new();
    engine.set_width(10);

    let lines = vec!["HelloWorldTest".to_string()];
    engine.rebuild_cache(&lines);
    engine.build_range(&lines, 0, lines.len());

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
    engine.build_range(&lines, 0, lines.len());

    assert_eq!(engine.visual_line_count(), 2);
    let vl = engine.get_visual_line(0).unwrap();
    assert_eq!(vl.text, "");
    assert_eq!(vl.logical_line, 0);
}

#[test]
fn test_visual_to_logical_binary_search() {
    let mut engine = WrapEngine::new();
    engine.set_width(10);

    let lines = vec![
        "Hello".to_string(),          // 1 visual line (row 0)
        "HelloWorldTest".to_string(), // 2 visual lines (row 1, 2)
        "End".to_string(),            // 1 visual line (row 3)
    ];
    engine.rebuild_cache(&lines);

    assert_eq!(engine.visual_to_logical(0).0, 0); // visual 0 -> line 0
    assert_eq!(engine.visual_to_logical(1).0, 1); // visual 1 -> line 1
    assert_eq!(engine.visual_to_logical(2).0, 1); // visual 2 -> line 1 (续行)
    assert_eq!(engine.visual_to_logical(3).0, 2); // visual 3 -> line 2
}

#[test]
fn test_sparse_cache() {
    let mut engine = WrapEngine::new();
    engine.set_width(10);

    let lines: Vec<String> = (0..1000).map(|i| format!("Line {}", i)).collect();
    engine.rebuild_cache(&lines);

    // 只构建第 500-510 行
    engine.build_range(&lines, 500, 510);

    // 第 505 行应该有缓存
    let cached = engine.get_cached_lines(505);
    assert!(!cached.is_empty());

    // 第 0 行不应该有缓存
    let cached = engine.get_cached_lines(0);
    assert!(cached.is_empty());

    // 但 visual_line_count 仍然正确
    assert_eq!(engine.visual_line_count(), 1000);
}

#[test]
fn test_compute_count_matches_wrap_line() {
    // 验证 compute_visual_line_count 与 wrap_line 产生一致的结果
    let mut engine = WrapEngine::new();
    engine.set_width(10);

    // 13 chars: "Hello, Wor" (10) + "ld!" (3) = 2 visual lines
    let line = "Hello, World!";
    let lines = vec![line.to_string()];
    engine.rebuild_cache(&lines);
    engine.build_range(&lines, 0, 1);

    let vlines = engine.get_cached_lines(0);
    assert_eq!(vlines.len(), engine.line_visual_counts[0]);
    assert_eq!(vlines.len(), 2);

    // 更长的文本，确保多行折行时一致
    let long_line = "Rust tests are currently inline unit tests under cfg blocks";
    let lines2 = vec![long_line.to_string()];
    engine.rebuild_cache(&lines2);
    engine.build_range(&lines2, 0, 1);

    let vlines2 = engine.get_cached_lines(0);
    assert_eq!(vlines2.len(), engine.line_visual_counts[0]);

    // 验证拼接后不丢字
    let reconstructed: String = vlines2.iter().map(|vl| vl.text.as_str()).collect();
    assert_eq!(reconstructed, long_line);
}

#[test]
fn table_height_inflates_first_row() {
    let mut engine = WrapEngine::new();
    engine.set_width(80);

    let lines: Vec<String> = vec![
        "前言".to_string(),
        "| a | b |".to_string(),
        "|---|---|".to_string(),
        "| 1 | 2 |".to_string(),
        "| 3 | 4 |".to_string(),
        "结尾".to_string(),
    ];
    // 假设表格 (1..=4) 渲染高度 = 8
    let table_blocks = vec![(1usize, 4usize, 8usize)];
    engine.rebuild_cache_with_blocks(&lines, &[], &table_blocks);

    // 视觉行数：前言 1 + 表格 8 + 续行 0+0+0 + 结尾 1 = 10
    assert_eq!(engine.visual_line_count(), 10);
    assert_eq!(engine.line_visual_counts[0], 1);
    assert_eq!(engine.line_visual_counts[1], 8);
    assert_eq!(engine.line_visual_counts[2], 0);
    assert_eq!(engine.line_visual_counts[3], 0);
    assert_eq!(engine.line_visual_counts[4], 0);
    assert_eq!(engine.line_visual_counts[5], 1);
}

#[test]
fn table_block_for_visual_row_finds_block() {
    let mut engine = WrapEngine::new();
    engine.set_width(80);

    let lines: Vec<String> = vec![
        "前言".to_string(),
        "| a | b |".to_string(),
        "|---|---|".to_string(),
        "| 1 | 2 |".to_string(),
        "结尾".to_string(),
    ];
    let table_blocks = vec![(1usize, 3usize, 5usize)]; // 表格 5 行渲染
    engine.rebuild_cache_with_blocks(&lines, &[], &table_blocks);

    // 视觉行 0：前言（不在表格里）
    assert_eq!(engine.table_block_for_visual_row(0), None);
    // 视觉行 1..=5：表格的渲染膨胀区
    for v in 1..=5 {
        assert_eq!(
            engine.table_block_for_visual_row(v),
            Some((1, 3)),
            "visual_row={} 应落在表格块内",
            v
        );
    }
    // 视觉行 6：结尾
    assert_eq!(engine.table_block_for_visual_row(6), None);
}

// ====== 渲染宽度折行（per-char visible widths） ======

/// 没传 visible_widths 时，行为应与老 API 完全一致。
#[test]
fn rebuild_without_widths_equals_old_behavior() {
    let mut a = WrapEngine::new();
    a.set_width(10);
    let mut b = WrapEngine::new();
    b.set_width(10);

    let lines = vec!["**bold** rest of line".to_string()];
    a.rebuild_cache_with_blocks(&lines, &[], &[]);
    b.rebuild_cache_with_blocks_and_widths(&lines, &[], &[], &[]);

    assert_eq!(a.visual_line_count(), b.visual_line_count());
    assert_eq!(a.line_visual_counts, b.line_visual_counts);
}

/// 给 `**bold**` 喂上"标记 = 0"的 widths 后，源码 13 char、渲染 9 列，
/// 宽度 10 时 1 行；不喂 widths 时（源码宽度按 char_width）会折成 2 行。
#[test]
fn bold_marks_zero_width_changes_wrap_count() {
    // 源码：`**bold** rest`  → 13 char，源码总宽 = 13
    // 渲染：`bold rest`        →  9 col
    let line = "**bold** rest".to_string();
    let chars: Vec<char> = line.chars().collect();
    // per-char widths：4 个 `*` = 0，其余 = 1
    let widths: Vec<u8> = chars
        .iter()
        .map(|c| if *c == '*' { 0 } else { 1 })
        .collect();
    let lines_slice = std::slice::from_ref(&line);

    // 不带 widths：源码宽 13 > 10 → 折 2 行
    let mut a = WrapEngine::new();
    a.set_width(10);
    a.rebuild_cache_with_blocks(lines_slice, &[], &[]);
    assert_eq!(a.visual_line_count(), 2, "源码宽 13 > 10 应折 2 行");

    // 带 widths：渲染宽 9 ≤ 10 → 不折
    let mut b = WrapEngine::new();
    b.set_width(10);
    b.rebuild_cache_with_blocks_and_widths(lines_slice, &[], &[], &[Some((widths, 0))]);
    assert_eq!(b.visual_line_count(), 1, "渲染宽 9 ≤ 10 应只占 1 行");
}

/// 某行传 None 时按源码宽算（即"光标所在行"的行为）。
#[test]
fn none_entry_falls_back_to_source_char_width() {
    let line = "**bold** rest".to_string();
    let mut engine = WrapEngine::new();
    engine.set_width(10);
    engine.rebuild_cache_with_blocks_and_widths(&[line], &[], &[], &[None]);
    // None → 等同源码宽 13 → 折 2 行
    assert_eq!(engine.visual_line_count(), 2);
}

/// 折行时同时维护"渲染端 char 索引"，让 renderer 端能正确切渲染产物。
///
/// 用例：`**hello world** more text` (25 char 源码)，渲染 `hello world more text` (21
/// char 渲染产物)。宽度 10，按渲染算第一个 vl 应该覆盖渲染端的前 ~10 char。
#[test]
fn visible_char_indices_track_render_position() {
    let line = "**hello world** more text".to_string();
    // 4 个 `*` 标记 → 0，其余 → 1（ASCII）
    let widths: Vec<u8> = line.chars().map(|c| if c == '*' { 0 } else { 1 }).collect();
    let lines_slice = std::slice::from_ref(&line);

    let mut engine = WrapEngine::new();
    engine.set_width(10);
    engine.rebuild_cache_with_blocks_and_widths(lines_slice, &[], &[], &[Some((widths, 0))]);
    engine.build_range(lines_slice, 0, 1);

    let vlines = engine.get_cached_lines(0);
    // 源码 25 char，4 个 `*` 不计宽，渲染端总宽 21；宽度 10 → 至少 3 个 vl
    assert!(
        vlines.len() >= 2,
        "应该被折成多个视觉行，实际：{}",
        vlines.len()
    );

    // 所有 vl 拼起来应覆盖整行源码（start_col 连续、end_col 收尾）
    assert_eq!(vlines[0].start_col, 0);
    let last = vlines.last().unwrap();
    assert_eq!(last.end_col, line.chars().count());

    // visible_start_char / visible_end_char 也应连续，且最终覆盖整个渲染产物
    let total_visible: usize = line.chars().filter(|c| *c != '*').count();
    assert_eq!(vlines[0].visible_start_char, 0);
    assert_eq!(
        last.visible_end_char, total_visible,
        "末视觉行的 visible_end_char 应覆盖渲染产物末尾"
    );

    // 每个 vl 的 visible 段宽度 ≤ wrap_width
    for (i, vl) in vlines.iter().enumerate() {
        let span = vl.visible_end_char - vl.visible_start_char;
        assert!(span <= 10, "vl[{}] 渲染段长 {} 超过 wrap_width 10", i, span);
    }
}

/// build_range 构建详细缓存时也应使用 widths（保持 visual_line_count 一致）。
#[test]
fn build_range_respects_widths() {
    let line = "**bold** rest".to_string();
    let widths: Vec<u8> = line.chars().map(|c| if c == '*' { 0 } else { 1 }).collect();
    let lines_slice = std::slice::from_ref(&line);

    let mut engine = WrapEngine::new();
    engine.set_width(10);
    engine.rebuild_cache_with_blocks_and_widths(lines_slice, &[], &[], &[Some((widths, 0))]);
    engine.build_range(lines_slice, 0, 1);

    let vlines = engine.get_cached_lines(0);
    assert_eq!(
        vlines.len(),
        1,
        "build_range 构建的精确缓存视觉行数应与 line_visual_counts 一致"
    );
    // 视觉行覆盖整行源码
    assert_eq!(vlines[0].start_col, 0);
    assert_eq!(vlines[0].end_col, line.chars().count());
}

/// 代码块内行（带 `-4` 边框补偿）即使在 visible_widths 里给了 Some 也应当
/// 按代码块路径走 —— 但当前实现里 visible_widths 是逐 char 累加；代码块
/// 不应被填 `Some(widths)`（editor.rs 那一层会保证传 None）。
/// 这条测试模拟"editor 正确地对代码块行传 None"的契约。
#[test]
fn code_block_line_keeps_border_compensation() {
    let line = "**bold** rest".to_string();
    let cb_ranges = vec![(0usize, 0usize)]; // 把行 0 当作代码块
    let mut engine = WrapEngine::new();
    engine.set_width(14); // 代码块行减去围栏+右内边距 6 = 有效宽 max(10, 8) = 10
    engine.rebuild_cache_with_blocks_and_widths(&[line], &cb_ranges, &[], &[None]);
    // 源码 13 char、有效宽 10（受 `.max(10)` 兜底）→ 折 2 行
    assert_eq!(engine.visual_line_count(), 2);
}

/// 光标换行触发的 dirty 机制：mark_dirty 后 is_dirty == true，rebuild 后清掉
#[test]
fn mark_dirty_then_rebuild_clears_dirty() {
    let mut engine = WrapEngine::new();
    engine.set_width(10);
    engine.rebuild_cache(&["hello".to_string()]);
    assert!(!engine.is_dirty());

    engine.mark_dirty();
    assert!(engine.is_dirty());

    engine.rebuild_cache(&["hello".to_string()]);
    assert!(!engine.is_dirty());
}

/// 复现"宽度小折行吞字符"bug：list 行 `- xxx` 在窄宽度下，
/// visible_end_char 比渲染产物实际 char 数多算（block prefix `- ` 被消费）。
///
/// 渲染产物 = "这是一段比较长的列表项目文字内容" (16 chars)，
/// 但 wrap 算的 visible_end_char 会包含 `- ` 两个 char，多算 2。
#[test]
fn block_prefix_visible_pos_matches_render_output() {
    use crate::util::text::char_width;
    // 模拟 list 行，宽度 10 列。
    // block_prefix_source_widths 给每个 char 算 char_width（包括 `- `），
    // 并返回 prefix_chars = 2（`- ` 在源码中占 2 char）。
    let line = "- 这是一段比较长的列表项目文字内容".to_string();
    let widths: Vec<u8> = line
        .chars()
        .map(|c| char_width(c).min(u8::MAX as usize) as u8)
        .collect();
    let lines_slice = std::slice::from_ref(&line);

    let mut engine = WrapEngine::new();
    engine.set_width(10);
    // 修复前：prefix_chars = 0（不知道前缀），visible_end_char 多算 2
    // 修复后：prefix_chars = 2，wrap_engine 跳过前缀 char 的 visible_pos 推进
    engine.rebuild_cache_with_blocks_and_widths(lines_slice, &[], &[], &[Some((widths, 2))]);
    engine.build_range(lines_slice, 0, 1);

    let vlines = engine.get_cached_lines(0);
    assert!(vlines.len() >= 2, "应该被折成多个视觉行");

    // 渲染产物长度（pulldown-cmark 消费 `- `，剩下 16 个中文 char）
    let render_output_len: usize = line.chars().count().saturating_sub(2);

    // 末视觉行的 visible_end_char 应等于渲染产物长度，而不是源码 char 数
    let last = vlines.last().unwrap();
    assert_eq!(
        last.visible_end_char, render_output_len,
        "末视觉行 visible_end_char 应等于渲染产物长度，但多算了 block prefix 的 char 数 —— 这就是宽度小折行吞字符的根因"
    );
}

/// 验证 block prefix 行折行后所有 vl 的 visible_start_char / visible_end_char
/// 首尾相接，且覆盖整个渲染产物（无丢失、无重复）。
///
/// 这是"宽度小折行吞字符"bug 的防回归测试：修复前续行的
/// visible_start_char 比前一段 visible_end_char 多算前缀 char 数，
/// 导致 extract_span_range 切片时跳过渲染产物开头的字符。
#[test]
fn block_prefix_wrap_continuity_preserved() {
    use crate::util::text::char_width;
    // heading 行 `### ` 前缀 4 char，后面是一段较长的中文正文
    let line = "### 这是一段比较长的标题文字内容用于测试折行连续性".to_string();
    let widths: Vec<u8> = line
        .chars()
        .map(|c| char_width(c).min(u8::MAX as usize) as u8)
        .collect();
    let lines_slice = std::slice::from_ref(&line);

    let mut engine = WrapEngine::new();
    engine.set_width(12);
    // heading `### ` 在源码中占 4 char，渲染产物中不含
    engine.rebuild_cache_with_blocks_and_widths(lines_slice, &[], &[], &[Some((widths, 4))]);
    engine.build_range(lines_slice, 0, 1);

    let vlines = engine.get_cached_lines(0);
    assert!(vlines.len() >= 2, "应该被折成多个视觉行");

    // 渲染产物长度 = 源码 char 数 - 前缀 char 数
    let render_output_len = line.chars().count() - 4;

    // 验证所有 vl 首尾相接
    for i in 0..vlines.len() {
        if i == 0 {
            assert_eq!(
                vlines[i].visible_start_char, 0,
                "第一段 visible_start_char 应为 0"
            );
        } else {
            assert_eq!(
                vlines[i].visible_start_char,
                vlines[i - 1].visible_end_char,
                "vl[{}] 的 visible_start_char 应等于 vl[{}] 的 visible_end_char",
                i,
                i - 1
            );
        }
    }
    // 末段覆盖整个渲染产物
    assert_eq!(
        vlines.last().unwrap().visible_end_char,
        render_output_len,
        "末段 visible_end_char 应覆盖整个渲染产物"
    );
}
