use super::*;
use ratatui::style::Color;

/// 测试用主题
fn test_theme() -> EditorTheme {
    EditorTheme {
        bg_primary: Color::Reset,
        bg_input: Color::Reset,
        code_bg: Color::DarkGray,
        cursor_fg: Color::Black,
        cursor_bg: Color::Cyan,
        text_normal: Color::White,
        text_dim: Color::DarkGray,
        text_bold: Color::White,
        md_h1: Color::Cyan,
        md_h2: Color::Green,
        md_h3: Color::Yellow,
        md_h4: Color::Magenta,
        md_heading_sep: Color::DarkGray,
        md_link: Color::Blue,
        md_list_bullet: Color::Yellow,
        md_blockquote_bar: Color::Cyan,
        md_blockquote_bg: Color::DarkGray,
        md_blockquote_text: Color::Gray,
        md_inline_code_fg: Color::Magenta,
        md_inline_code_bg: Color::DarkGray,
        md_rule: Color::DarkGray,
        code_border: Color::DarkGray,
        table_header: Color::White,
        table_body: Color::White,
        code_default: Color::White,
        code_keyword: Color::Magenta,
        code_string: Color::Green,
        code_comment: Color::DarkGray,
        code_number: Color::Yellow,
        code_type: Color::Yellow,
        code_primitive: Color::Cyan,
        code_macro: Color::LightCyan,
        code_lifetime: Color::LightMagenta,
        code_attribute: Color::LightBlue,
        code_shell_var: Color::LightCyan,
        label_ai: Color::Green,
    }
}

#[test]
fn test_search() {
    let mut search = SearchState::new();
    let lines = vec!["hello world".to_string(), "hello universe".to_string()];

    let count = search.search("hello", &lines);
    assert_eq!(count, 2);
    assert_eq!(search.match_count(), 2);
}

#[test]
fn test_navigation() {
    let mut search = SearchState::new();
    let lines = vec!["aaa bbb aaa".to_string()];

    search.search("aaa", &lines);

    let m = search.current_match().unwrap();
    assert_eq!(m.start, 0);

    search.next_match();
    let m = search.current_match().unwrap();
    assert_eq!(m.start, 8);

    search.prev_match();
    let m = search.current_match().unwrap();
    assert_eq!(m.start, 0);
}

#[test]
fn test_search_chinese() {
    let mut search = SearchState::new();
    let lines = vec!["你好世界，你好宇宙".to_string()];

    let count = search.search("你好", &lines);
    assert_eq!(count, 2);

    let m = search.current_match().unwrap();
    assert_eq!(m.start, 0);
    assert_eq!(m.end, 2);

    search.next_match();
    let m = search.current_match().unwrap();
    assert_eq!(m.start, 5);
    assert_eq!(m.end, 7);
}

#[test]
fn test_highlight_line_with_offset() {
    let mut search = SearchState::new();
    // 逻辑行 "hello world hello" 被折行成两个视觉行
    let lines = vec!["hello world hello".to_string()];
    search.search("hello", &lines);
    assert_eq!(search.match_count(), 2);

    let theme = test_theme();

    // 模拟第二个视觉行片段 "d hello"，char_offset = 10
    let spans = search.highlight_line(0, "d hello", &theme, 10);
    // 应该高亮 "hello" (local 2..7)
    assert!(spans.len() >= 2);
}

#[test]
fn test_highlight_chinese_no_panic() {
    let mut search = SearchState::new();
    let lines = vec!["测试中文搜索功能".to_string()];
    search.search("中文", &lines);

    let theme = test_theme();
    // 传入完整行，不应 panic
    let spans = search.highlight_line(0, "测试中文搜索功能", &theme, 0);
    assert!(!spans.is_empty());

    // 传入片段（模拟折行），也不应 panic
    let spans = search.highlight_line(0, "搜索功能", &theme, 4);
    assert!(!spans.is_empty());
}

#[test]
fn test_highlight_chinese_correctness() {
    let mut search = SearchState::new();
    let lines = vec!["测试中文搜索功能".to_string()];
    search.search("中文", &lines);

    assert_eq!(search.match_count(), 1);
    let m = search.current_match().unwrap();
    assert_eq!(m.start, 2);
    assert_eq!(m.end, 4);

    let theme = test_theme();

    // 完整行高亮
    let spans = search.highlight_line(0, "测试中文搜索功能", &theme, 0);
    // 应该有 3 段：测试(普通) + 中文(高亮) + 搜索功能(普通)
    assert_eq!(spans.len(), 3, "应有3段span，实际: {:?}", spans);
    assert_eq!(spans[0].content, "测试");
    assert_eq!(spans[1].content, "中文");
    assert_eq!(spans[2].content, "搜索功能");

    // 验证高亮 span 的背景色是黄色
    assert_eq!(spans[1].style.bg, Some(Color::Yellow));
}
