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
