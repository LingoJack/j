use super::*;
use crate::command::chat::render::theme::{Theme, ThemeName};
use crate::markdown::ir::Inline;
use crate::markdown::render::table::wrap_cell_inlines;
use crate::util::text::char_width;
use ratatui::style::{Modifier, Style};

/// 计算一行 Line 的实际显示宽度（基于 spans 中所有 content 的字符宽度之和）
fn line_display_width(line: &Line<'_>) -> usize {
    line.spans
        .iter()
        .map(|s| s.content.chars().map(char_width).sum::<usize>())
        .sum()
}

/// 验证窄终端下表格竖线不错位：每行实际宽度不超过 max_width
#[test]
fn narrow_terminal_table_no_overflow() {
    let theme = Theme::from_name(&ThemeName::default());
    // 多列表格，包含中文宽字符内容
    let md = r"| 列1 | 列2 | 列3 |
|-----|-----|-----|
| 中文字符 | 测试内容 | 第三列数据 |";

    // 窄终端（20 字符），列宽被压缩，宽字符可能导致溢出
    let max_width = 20usize;
    let lines = markdown_to_lines(md, max_width, &theme);

    // 验证每行实际显示宽度不超过 max_width
    for line in &lines {
        let w = line_display_width(line);
        assert!(
            w <= max_width,
            "行宽度 {} 超过 max_width {}: {:?}",
            w,
            max_width,
            line.spans.iter().map(|s| &s.content).collect::<Vec<_>>()
        );
    }
}

/// 验证极窄终端（10 字符）下表格渲染不溢出
#[test]
fn very_narrow_terminal_table_no_overflow() {
    let theme = Theme::from_name(&ThemeName::default());
    let md = r"| A | B | C |
|---|---|---|
| 中文 | 测试 | 数据 |";

    let max_width = 10usize;
    let lines = markdown_to_lines(md, max_width, &theme);

    for line in &lines {
        let w = line_display_width(line);
        assert!(
            w <= max_width,
            "行宽度 {} 超过 max_width {}: {:?}",
            w,
            max_width,
            line.spans.iter().map(|s| &s.content).collect::<Vec<_>>()
        );
    }
}

/// 验证 `wrap_cell_inlines` 返回的子行宽度不超过 max_width（允许为 2，因为 max(2) 提升）
#[test]
fn wrap_cell_styled_width_constraint() {
    let theme = Theme::from_name(&ThemeName::default());
    let base = Style::default();
    let code = Style::default();

    // 测试纯中文内容，每个字符宽度为 2
    let inlines = vec![Inline::Text("中文字符测试".to_string())];
    // 极窄列宽（1），会被提升到 max(2)
    let max_width = 1usize;
    let wrapped = wrap_cell_inlines(&inlines, max_width, base, code, &theme);

    // 由于 max_width = max(1, 2) = 2，每个子行最多容纳一个中文字符
    for (_spans, w) in &wrapped {
        // 每行最多 2（一个中文字符），但可能截断后更少
        assert!(
            *w <= 2,
            "wrap_cell_inlines 返回的行宽度 {} 超过 max(2): {:?}",
            w,
            wrapped
        );
    }

    // 验证所有子行的内容拼接后总宽度等于原文本宽度
    let total_w: usize = wrapped.iter().map(|(_, w)| *w).sum();
    let expected_w: usize = "中文字符测试".chars().map(char_width).sum();
    assert_eq!(
        total_w, expected_w,
        "所有子行宽度之和 {} != 原文本宽度 {}",
        total_w, expected_w
    );
}

/// 验证截断逻辑正确工作：当 col_widths[i] 小于字符宽度时，内容被截断
#[test]
fn truncation_when_column_width_too_small() {
    let theme = Theme::from_name(&ThemeName::default());
    // 单列表格，包含一个宽度为 2 的中文字符
    let md = "| 中 |\n|---|\n| 文 |";

    // 极窄终端（5 字符），列宽会被压缩
    let max_width = 5usize;
    let lines = markdown_to_lines(md, max_width, &theme);

    // 验证每行不溢出
    for line in &lines {
        let w = line_display_width(line);
        assert!(
            w <= max_width,
            "行宽度 {} 超过 max_width {}: {:?}",
            w,
            max_width,
            line.spans.iter().map(|s| &s.content).collect::<Vec<_>>()
        );
    }
}

/// 验证表格中行内代码样式正确保留
#[test]
fn table_inline_code_style_preserved() {
    let theme = Theme::from_name(&ThemeName::default());
    // 表格包含行内代码
    let md = "| 列1 | 列2 |\n|-----|-----|\n| `code` | 普通 |";

    let max_width = 40usize;
    let lines = markdown_to_lines(md, max_width, &theme);

    // 打印所有行内容用于调试
    for (i, line) in lines.iter().enumerate() {
        eprintln!(
            "Line {}: {:?}",
            i,
            line.spans
                .iter()
                .map(|s| (&s.content, s.style))
                .collect::<Vec<_>>()
        );
    }

    // 检查是否有 span 包含 "code"
    let has_code_content: bool = lines
        .iter()
        .flat_map(|line| &line.spans)
        .any(|s| s.content.contains("code"));

    assert!(
        has_code_content,
        "表格渲染结果中应包含 'code' 内容: {:?}",
        lines
            .iter()
            .flat_map(|l| &l.spans)
            .map(|s| &s.content)
            .collect::<Vec<_>>()
    );

    // 检查 "code" span 具有行内代码样式（有背景色）
    let code_spans: Vec<_> = lines
        .iter()
        .flat_map(|line| &line.spans)
        .filter(|s| s.content == "code")
        .collect();

    assert!(!code_spans.is_empty(), "应存在 content='code' 的 span");

    for cs in &code_spans {
        assert!(
            cs.style.bg.is_some(),
            "'code' span 应有背景色（行内代码样式）: {:?}",
            cs.style
        );
    }
}

#[test]
fn markdown_tabs_are_normalized_before_rendering() {
    let theme = Theme::from_name(&ThemeName::default());
    let lines = markdown_to_lines("foo\tbar\r\nbaz", 20, &theme);

    for line in &lines {
        for span in &line.spans {
            assert!(
                !span.content.contains('\t') && !span.content.contains('\r'),
                "span should not contain raw control chars: {:?}",
                span.content
            );
        }
    }

    let rendered: Vec<String> = lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect()
        })
        .collect();
    assert_eq!(rendered, vec!["foo    bar baz".to_string()]);
}

#[test]
fn markdown_ansi_and_control_chars_are_sanitized_before_rendering() {
    let theme = Theme::from_name(&ThemeName::default());
    let lines = markdown_to_lines("foo\x1b[31mbar\x1b[0m\x07baz", 40, &theme);

    for line in &lines {
        for span in &line.spans {
            assert!(
                !span.content.contains('\x1b')
                    && !span.content.contains('\x07')
                    && !span.content.contains("[31m")
                    && !span.content.contains("[0m"),
                "span should not contain leaked ANSI/control fragments: {:?}",
                span.content
            );
        }
    }

    let rendered: Vec<String> = lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect()
        })
        .collect();
    assert_eq!(rendered, vec!["foobarbaz".to_string()]);
}

/// 验证宽终端下行内代码样式正确渲染
#[test]
fn table_inline_code_wide_terminal() {
    let theme = Theme::from_name(&ThemeName::default());
    let md = "| 命令 | 说明 |\n|------|------|\n| `git status` | 查看状态 |\n| `cargo build` | 编译项目 |";

    let max_width = 60usize;
    let lines = markdown_to_lines(md, max_width, &theme);

    eprintln!("=== wide terminal test ===");
    for (i, line) in lines.iter().enumerate() {
        eprintln!(
            "Line {}: {:?}",
            i,
            line.spans
                .iter()
                .map(|s| (&s.content, s.style))
                .collect::<Vec<_>>()
        );
    }

    // 检查 git status 和 cargo build 都有代码样式
    let code_contents = ["git status", "cargo build"];
    for expected in code_contents {
        let found = lines
            .iter()
            .flat_map(|line| &line.spans)
            .any(|s| s.content == expected && s.style.bg.is_some());
        assert!(found, "应有 content='{}' 且有背景色的 span", expected);
    }
}

/// 验证窄终端下行内代码仍保留样式
#[test]
fn table_inline_code_narrow_terminal() {
    let theme = Theme::from_name(&ThemeName::default());
    let md = "| A | B |\n|---|---|\n| `code` | 文本 |";

    let max_width = 15usize;
    let lines = markdown_to_lines(md, max_width, &theme);

    eprintln!("=== narrow terminal test ===");
    for (i, line) in lines.iter().enumerate() {
        eprintln!(
            "Line {}: {:?}",
            i,
            line.spans
                .iter()
                .map(|s| (&s.content, s.style))
                .collect::<Vec<_>>()
        );
    }

    // 在窄终端下，"code" 可能被截断，但只要存在就应有代码样式
    let code_spans: Vec<_> = lines
        .iter()
        .flat_map(|line| &line.spans)
        .filter(|s| s.content.contains("code"))
        .collect();

    for cs in &code_spans {
        assert!(
            cs.style.bg.is_some(),
            "'code' span 应有背景色: content={}, style={:?}",
            cs.content,
            cs.style
        );
    }
}

/// 验证复杂表格（类似 hook.md）中行内代码样式正确渲染
#[test]
fn table_complex_inline_code_like_hook_md() {
    let theme = Theme::from_name(&ThemeName::default());
    // 模拟 hook.md 中的表格结构，包含大量行内代码
    let md = r"| 事件 | 触发时机 | 可读字段 | 可写字段 |
|------|----------|----------|----------|
| `pre_send_message` | 用户发送消息前 | `user_input`, `messages` | `user_input`, `action=stop`, `retry_feedback` |
| `post_send_message` | 用户发送消息后 | `user_input`, `messages` | 仅通知，返回值被忽略 |
| `pre_llm_request` | LLM API 请求前 | `messages`, `system_prompt`, `model` | `messages`, `system_prompt`, `inject_messages` |";

    let max_width = 80usize;
    let lines = markdown_to_lines(md, max_width, &theme);

    eprintln!("=== hook.md style table test ===");
    for (i, line) in lines.iter().enumerate() {
        eprintln!(
            "Line {}: {:?}",
            i,
            line.spans
                .iter()
                .map(|s| (&s.content, s.style))
                .collect::<Vec<_>>()
        );
    }

    // 检查所有行内代码都有背景色
    let code_spans: Vec<_> = lines
        .iter()
        .flat_map(|line| &line.spans)
        .filter(|s| {
            // 行内代码内容：包含下划线的事件名、字段名等
            let content = &s.content;
            content.contains("pre_send_message")
                || content.contains("post_send_message")
                || content.contains("pre_llm_request")
                || content.contains("user_input")
                || content.contains("messages")
                || content.contains("system_prompt")
                || content.contains("action")
                || content.contains("retry_feedback")
                || content.contains("inject_messages")
                || content.contains("model")
        })
        .collect();

    eprintln!("Found {} code-like spans", code_spans.len());
    for cs in &code_spans {
        eprintln!(
            "  content='{}', has_bg={}",
            cs.content,
            cs.style.bg.is_some()
        );
    }

    // 所有这些内容都应有背景色（行内代码样式）
    for cs in &code_spans {
        assert!(
            cs.style.bg.is_some(),
            "行内代码 '{}' 应有背景色: {:?}",
            cs.content,
            cs.style
        );
    }
}

/// 直接使用 hook.md 中实际的表格内容测试行内代码渲染
#[test]
fn table_hook_md_actual_content() {
    let theme = Theme::from_name(&ThemeName::default());
    // 来自 hook.md 第 100-103 行的表格
    let md = r"| 事件 | 触发时机 | 可读字段 | 可写字段 |
|------|----------|----------|----------|
| `pre_send_message` | 用户发送消息前 | `user_input`, `messages` | `user_input`, `action=stop`, `retry_feedback` |
| `post_send_message` | 用户发送消息后 | `user_input`, `messages` | 仅通知，返回值被忽略 |";

    // 模拟 help 页面的 content_width（终端宽度 80 - 4 = 76）
    let max_width = 76usize;
    let lines = markdown_to_lines(md, max_width, &theme);

    eprintln!("=== hook.md actual table test ===");
    for (i, line) in lines.iter().enumerate() {
        eprintln!(
            "Line {}: {:?}",
            i,
            line.spans
                .iter()
                .map(|s| (&s.content, s.style))
                .collect::<Vec<_>>()
        );
    }

    // 检查所有有代码样式的 span（有背景色）
    let code_spans: Vec<_> = lines
        .iter()
        .flat_map(|line| &line.spans)
        .filter(|s| s.style.bg.is_some())
        .collect();

    eprintln!(
        "Found {} spans with background color (code style)",
        code_spans.len()
    );
    for cs in &code_spans {
        eprintln!("  code: '{}'", cs.content);
    }

    // 验证存在代码样式的 span
    assert!(!code_spans.is_empty(), "表格中应有代码样式的 span");

    // 验证关键内容存在（可能被截断，所以用 contains）
    let code_content: String = code_spans.iter().map(|s| s.content.to_string()).collect();
    assert!(
        code_content.contains("pre_send") || code_content.contains("post_send"),
        "应有 pre_send 或 post_send 相关的代码内容"
    );
    assert!(
        code_content.contains("user_input"),
        "应有 user_input 代码内容"
    );
}

// ════════════════════════════════════════════════════════════════
// 回归测试：核心 Markdown 渲染场景
// 如果以下测试失败，说明 markdown 渲染管线被意外修改
// ════════════════════════════════════════════════════════════════

#[test]
fn renders_plain_text() {
    let theme = Theme::from_name(&ThemeName::default());
    let lines = markdown_to_lines("hello world", 80, &theme);
    // 至少有一行包含 hello world
    let content: String = lines
        .iter()
        .flat_map(|l| l.spans.iter().map(|s| s.content.to_string()))
        .collect();
    assert!(
        content.contains("hello world"),
        "纯文本应包含 'hello world'"
    );
}

#[test]
fn renders_bold_text() {
    let theme = Theme::from_name(&ThemeName::default());
    let lines = markdown_to_lines("this is **bold** text", 80, &theme);
    let all_spans: Vec<_> = lines.iter().flat_map(|l| l.spans.iter()).collect();
    let bold_span = all_spans.iter().find(|s| s.content.contains("bold"));
    assert!(bold_span.is_some(), "应有包含 'bold' 的 span");
    let span = bold_span.unwrap();
    assert!(
        span.style.add_modifier.contains(Modifier::BOLD),
        "'bold' span 应有 BOLD 修饰"
    );
}

#[test]
fn renders_code_block_with_borders() {
    let theme = Theme::from_name(&ThemeName::default());
    let md = "```rust\nfn main() {}\n```";
    let lines = markdown_to_lines(md, 80, &theme);
    // 应有顶边框和底边框
    let border_lines: Vec<_> = lines
        .iter()
        .flat_map(|l| l.spans.iter())
        .filter(|s| s.content.contains('┌') || s.content.contains('└'))
        .collect();
    assert!(!border_lines.is_empty(), "代码块应有边框字符 ┌ / └");
    // 应包含代码内容
    let content: String = lines
        .iter()
        .flat_map(|l| l.spans.iter().map(|s| s.content.to_string()))
        .collect();
    assert!(content.contains("fn main()"), "代码块应包含代码内容");
}

#[test]
fn renders_inline_code() {
    let theme = Theme::from_name(&ThemeName::default());
    let lines = markdown_to_lines("use `cargo test` to run", 80, &theme);
    let code_span = lines
        .iter()
        .flat_map(|l| l.spans.iter())
        .find(|s| s.content.contains("cargo test"));
    assert!(code_span.is_some(), "应有包含 'cargo test' 的 span");
    assert!(code_span.unwrap().style.bg.is_some(), "行内代码应有背景色");
}

#[test]
fn renders_heading_with_prefix() {
    let theme = Theme::from_name(&ThemeName::default());
    let lines = markdown_to_lines("# Title", 80, &theme);
    let content: String = lines
        .iter()
        .flat_map(|l| l.spans.iter().map(|s| s.content.to_string()))
        .collect();
    assert!(content.contains("◆"), "H1 应有 ◆ 前缀");
    assert!(content.contains("Title"), "H1 应包含标题内容");
}

#[test]
fn renders_ordered_list_with_bold_items() {
    let theme = Theme::from_name(&ThemeName::default());
    // 精确复现用户报告的场景：有序列表 + 粗体中文
    let md = "日报已成功写入。总结一下今天写入的 3 条技术日报：\n\n1. **编辑器中文宽字符鼠标点击定位修复** — 修复 char_idx_at_display_col\n2. **Chat UI 鼠标拖拽选区与复制** — 新增完整的鼠标选区功能\n3. **编辑器鼠标定位重构** — screen_to_logical 改用渲染行元数据映射";

    let lines = markdown_to_lines(md, 80, &theme);

    eprintln!("=== ordered list test ===");
    for (i, line) in lines.iter().enumerate() {
        eprintln!(
            "Line {}: {:?}",
            i,
            line.spans.iter().map(|s| &s.content).collect::<Vec<_>>()
        );
    }

    // 应有序号 1. 2. 3.
    let all_text: String = lines
        .iter()
        .flat_map(|l| l.spans.iter().map(|s| s.content.to_string()))
        .collect();

    assert!(all_text.contains("1."), "应有序号 '1.'");
    assert!(all_text.contains("2."), "应有序号 '2.'");
    assert!(all_text.contains("3."), "应有序号 '3.'");

    // 应有粗体内容
    assert!(
        all_text.contains("编辑器中文宽字符鼠标点击定位修复"),
        "应有第 1 项内容"
    );
    assert!(
        all_text.contains("Chat UI 鼠标拖拽选区与复制"),
        "应有第 2 项内容"
    );
    assert!(all_text.contains("编辑器鼠标定位重构"), "应有第 3 项内容");
}

#[test]
fn renders_list_with_bullet() {
    let theme = Theme::from_name(&ThemeName::default());
    let md = "- item one\n- item two";
    let lines = markdown_to_lines(md, 80, &theme);
    let bullets: Vec<_> = lines
        .iter()
        .flat_map(|l| l.spans.iter())
        .filter(|s| s.content.contains('•'))
        .collect();
    assert!(
        bullets.len() >= 2,
        "应有至少 2 个列表子弹符号 •，实际 {}",
        bullets.len()
    );
}

#[test]
fn handles_chinese_quotes_bold() {
    let theme = Theme::from_name(&ThemeName::default());
    // \u{201C} = " \u{201D} = "
    let md = "**\u{201C}中文引号内容\u{201D}**";
    let lines = markdown_to_lines(md, 80, &theme);
    let bold_spans: Vec<_> = lines
        .iter()
        .flat_map(|l| l.spans.iter())
        .filter(|s| s.style.add_modifier.contains(Modifier::BOLD))
        .collect();
    assert!(!bold_spans.is_empty(), "中文引号内的 ** 加粗应生效");
    let content: String = bold_spans.iter().map(|s| s.content.to_string()).collect();
    assert!(content.contains("中文引号内容"), "加粗内容应包含中文字符");
}

#[test]
fn renders_empty_input_returns_empty_or_wrapped() {
    let theme = Theme::from_name(&ThemeName::default());
    let lines = markdown_to_lines("", 80, &theme);
    // 空输入不应 panic
    assert!(
        lines
            .iter()
            .all(|l| l.spans.is_empty() || l.spans.iter().all(|s| s.content.is_empty())),
        "空输入的所有行应为空或仅含空 span"
    );
}

/// 回归测试：复现 AI 回复中第一个表格未渲染的 bug
/// 场景：AI 回复包含多个表格，第一个表格出现在段落文字之后，
/// 使用 `|---|` 单列分隔符格式。当整个内容被一次性解析时，表格应正确渲染。
#[test]
fn table_after_paragraph_with_single_col_separator() {
    let theme = Theme::from_name(&ThemeName::default());

    // 模拟 AI 的完整回复内容
    let md = r##"渲染模块位于 `src/command/chat/render/cache/` 下，包含以下文件：

| 文件 | 职责 |
|---|
| `cache.rs` | 主入口：`build_message_lines_incremental` 函数 |
| `tool_call_render.rs` | **工具调用请求**渲染 |
| `tool_result_render.rs` | **工具执行结果**渲染 |

### 2. 数据模型

| Category | 工具 | 图标 |
|---|---|---|
| `File` | Read, Write | 📄 |
| `Search` | Grep | 🔍 |
"##;

    let lines = markdown_to_lines(md, 100, &theme);

    eprintln!("=== table_after_paragraph test ===");
    for (i, line) in lines.iter().enumerate() {
        eprintln!(
            "Line {}: {:?}",
            i,
            line.spans.iter().map(|s| &s.content).collect::<Vec<_>>()
        );
    }

    // 验证至少有一个表格被渲染（通过检测表格边框字符 ┌ 或 └）
    let has_table_border: bool = lines
        .iter()
        .flat_map(|l| l.spans.iter())
        .any(|s| s.content.contains('┌') || s.content.contains('└'));

    assert!(
        has_table_border,
        "应至少有一个表格被渲染（检测到 ┌ 或 └ 边框字符），实际输出: {:?}",
        lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.to_string()))
            .collect::<Vec<_>>()
    );

    // 统计表格数量（每个表格有一个 ┌ 和一个 └）
    let top_borders: Vec<_> = lines
        .iter()
        .flat_map(|l| l.spans.iter())
        .filter(|s| s.content.contains('┌'))
        .collect();

    // 应该有 2 个表格
    assert_eq!(
        top_borders.len(),
        2,
        "应渲染 2 个表格，实际渲染了 {} 个",
        top_borders.len()
    );
}

/// 回归测试：流式渲染切分后，tail 中的表格能正确渲染
/// 场景：find_stable_boundary 将内容切分后，tail 部分以 | 开头（表格开头）
#[test]
fn table_in_tail_after_stable_boundary_cut() {
    let theme = Theme::from_name(&ThemeName::default());

    // 模拟 find_stable_boundary 切分后的 tail 内容
    // 在流式渲染中，boundary 在段落后的 \n\n 处，tail 以表格开头
    let tail_content =
        "| 文件 | 职责 |\n|---|\n| `cache.rs` | 主入口 |\n| `tool_call_render.rs` | 工具渲染 |";

    let lines = markdown_to_lines(tail_content, 80, &theme);

    eprintln!("=== table_in_tail test ===");
    for (i, line) in lines.iter().enumerate() {
        eprintln!(
            "Line {}: {:?}",
            i,
            line.spans.iter().map(|s| &s.content).collect::<Vec<_>>()
        );
    }

    // 验证表格被渲染
    let has_table_border: bool = lines
        .iter()
        .flat_map(|l| l.spans.iter())
        .any(|s| s.content.contains('┌') || s.content.contains('└'));

    assert!(
        has_table_border,
        "tail 中的表格应被正确渲染，实际输出: {:?}",
        lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.to_string()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn nested_list_preserves_ordered_and_indentation() {
    // 回归用例：外层有序列表 + 嵌套无序子列表。
    // 修复前的 bug 表现：
    //   1. 外层 1./2. 被渲染为 • （order 标志被内层覆盖）
    //   2. 子项未缩进，与外层项同级
    //   3. 出现空的 "• " 幻影行
    //   4. 父项尾部内容（如 "字段"）被吞入并与子项拼接
    let md = "1. **路由未注册** - 文件: `router/router.go`\n   - Controllers 结构体中缺少 `SearchHistory *controller.SearchHistoryController` 字段\n   - private 路由组中缺少 4 条搜索历史路由\n\n2. **Wire 依赖注入未更新** - 文件: `cmd/server/wire.go`\n   - 缺少 `service.NewSearchHistoryService`\n";
    let theme = Theme::from_name(&ThemeName::default());
    let lines = markdown_to_lines(md, 80, &theme);
    let rendered: Vec<String> = lines
        .iter()
        .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect())
        .collect();

    // 外层序号 1./2. 必须出现
    assert!(
        rendered.iter().any(|l| l.starts_with("1. ")),
        "外层首项应以 '1. ' 开头，实际: {:?}",
        rendered
    );
    assert!(
        rendered.iter().any(|l| l.starts_with("2. ")),
        "外层第二项应以 '2. ' 开头，实际: {:?}",
        rendered
    );

    // 不应出现未指定序号的 "• " 顶级项（即嵌套子项不能冒泡到顶级）
    assert!(
        !rendered.iter().any(|l| l.starts_with("• ")),
        "顶级不应出现 '• '（嵌套冒泡 bug），实际: {:?}",
        rendered
    );

    // 子项必须带 2 空格缩进 + '•'
    assert!(
        rendered
            .iter()
            .any(|l| l.starts_with("  • ") && l.contains("Controllers 结构体中缺少")),
        "嵌套子项应缩进 2 空格，实际: {:?}",
        rendered
    );

    // 不应出现纯 "• " 空 bullet 行（幻影项）
    assert!(
        !rendered.iter().any(|l| l.trim() == "•"),
        "不应产生空 bullet 幻影行，实际: {:?}",
        rendered
    );

    // 父项尾部 "字段" 必须归属于子项而非混入父项
    let first_outer = rendered
        .iter()
        .find(|l| l.starts_with("1. "))
        .expect("存在外层 1. 项");
    assert!(
        !first_outer.contains("Controllers 结构体中缺少"),
        "父项内容不应吞掉子项文本，实际外层首项: {:?}",
        first_outer
    );
}

#[test]
fn deeply_nested_list_indentation() {
    // 深度 3 的列表应按 0 / 2 / 4 空格递增缩进
    let md = "- a\n  - b\n    - c\n";
    let theme = Theme::from_name(&ThemeName::default());
    let lines = markdown_to_lines(md, 80, &theme);
    let rendered: Vec<String> = lines
        .iter()
        .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect())
        .collect();

    assert!(
        rendered.iter().any(|l| l.starts_with("• a")),
        "第 1 层无缩进，实际: {:?}",
        rendered
    );
    assert!(
        rendered.iter().any(|l| l.starts_with("  • b")),
        "第 2 层缩进 2 空格，实际: {:?}",
        rendered
    );
    assert!(
        rendered.iter().any(|l| l.starts_with("    • c")),
        "第 3 层缩进 4 空格，实际: {:?}",
        rendered
    );
}
