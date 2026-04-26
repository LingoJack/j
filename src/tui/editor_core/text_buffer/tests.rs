use super::*;

#[test]
fn test_basic_insert() {
    let mut buf = TextBuffer::new();
    buf.insert_char('H');
    buf.insert_char('i');
    assert_eq!(buf.to_string(), "Hi");
    assert_eq!(buf.cursor(), (0, 2));
}

#[test]
fn test_newline() {
    let mut buf = TextBuffer::new();
    buf.insert_str("Hello\nWorld");
    assert_eq!(buf.lines().len(), 2);
    assert_eq!(buf.lines()[0], "Hello");
    assert_eq!(buf.lines()[1], "World");
}

#[test]
fn test_cursor_movement() {
    let mut buf = TextBuffer::from_content("Hello\nWorld");
    buf.move_cursor_end();
    assert_eq!(buf.cursor(), (0, 5));
    buf.move_cursor_down();
    assert_eq!(buf.cursor(), (1, 5));
    buf.move_cursor_head();
    assert_eq!(buf.cursor(), (1, 0));
}

#[test]
fn test_delete() {
    let mut buf = TextBuffer::from_content("Hello");
    buf.set_cursor(0, 1);
    buf.delete_char();
    assert_eq!(buf.to_string(), "Hllo");
}

#[test]
fn test_word_movement() {
    let mut buf = TextBuffer::from_content("hello world test");
    buf.move_cursor_word_forward();
    assert_eq!(buf.cursor(), (0, 6));
    buf.move_cursor_word_forward();
    assert_eq!(buf.cursor(), (0, 12));
    buf.move_cursor_word_back();
    assert_eq!(buf.cursor(), (0, 6));
}

#[test]
fn test_chinese_insert() {
    let mut buf = TextBuffer::new();
    buf.insert_char('你');
    buf.insert_char('好');
    buf.insert_char('世');
    buf.insert_char('界');
    assert_eq!(buf.to_string(), "你好世界");
    assert_eq!(buf.cursor(), (0, 4));
}

#[test]
fn test_chinese_delete() {
    let mut buf = TextBuffer::from_content("你好世界");
    buf.set_cursor(0, 2);
    buf.delete_char();
    assert_eq!(buf.to_string(), "你好界");
    assert_eq!(buf.cursor(), (0, 2));

    buf.backspace();
    assert_eq!(buf.to_string(), "你界");
    assert_eq!(buf.cursor(), (0, 1));
}

#[test]
fn test_chinese_insert_mid() {
    let mut buf = TextBuffer::from_content("你好世界");
    buf.set_cursor(0, 2);
    buf.insert_char('的');
    assert_eq!(buf.to_string(), "你好的世界");
}

#[test]
fn test_chinese_newline() {
    let mut buf = TextBuffer::from_content("你好世界");
    buf.set_cursor(0, 2);
    buf.insert_newline();
    assert_eq!(buf.lines().len(), 2);
    assert_eq!(buf.lines()[0], "你好");
    assert_eq!(buf.lines()[1], "世界");
    assert_eq!(buf.cursor(), (1, 0));
}

#[test]
fn test_chinese_delete_line_by_end() {
    let mut buf = TextBuffer::from_content("你好世界");
    buf.set_cursor(0, 2);
    buf.delete_line_by_end();
    assert_eq!(buf.to_string(), "你好");
}

#[test]
fn test_chinese_delete_word() {
    let mut buf = TextBuffer::from_content("你好 世界 测试");
    buf.set_cursor(0, 0);
    buf.delete_word();
    assert_eq!(buf.to_string(), " 世界 测试");
}

#[test]
fn test_chinese_word_movement() {
    let mut buf = TextBuffer::from_content("你好 世界 测试");
    buf.move_cursor_word_forward();
    assert_eq!(buf.cursor(), (0, 3));
    buf.move_cursor_word_forward();
    assert_eq!(buf.cursor(), (0, 6));
    buf.move_cursor_word_back();
    assert_eq!(buf.cursor(), (0, 3));
}
