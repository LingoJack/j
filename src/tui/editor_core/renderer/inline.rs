//! 行内 Markdown 元素渲染子模块
//!
//! 使用共享层渲染原语处理 **bold**、*italic*、~~strike~~、`code`、链接等。
//! 基于 pulldown-cmark 解析，正确处理边界情况和嵌套。

use ratatui::{style::Style, text::Span};

use crate::markdown::ir::Inline;

use super::MarkdownRenderer;

impl MarkdownRenderer {
    /// 渲染行内元素
    ///
    /// 使用 pulldown-cmark 解析文本，提取 inline 元素，
    /// 然后调用共享层渲染原语生成 Span 列表。
    pub(super) fn render_inline(&self, text: &str) -> Vec<Span<'static>> {
        let inlines = parse_inline_text(text);

        let base_style = Style::default()
            .fg(self.theme.text_normal)
            .bg(self.theme.bg_primary);

        crate::markdown::render::inline::render_inlines(&inlines, base_style, &self.theme)
    }
}

// ---------------------------------------------------------------------------
// Inline parsing helpers
// ---------------------------------------------------------------------------

/// Inline 容器类型（用于嵌套栈）
#[derive(Debug, Clone)]
enum InlineContainer {
    Strong,
    Emphasis,
    Strikethrough,
    Link { url: String },
}

/// 解析文本中的 inline 元素（不解析 block 结构）
///
/// 使用 pulldown-cmark 的 inline-only 解析，提取 **bold**、*italic*、
/// ~~strike~~、`code`、[link]() 等元素。
fn parse_inline_text(text: &str) -> Vec<Inline> {
    use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

    let options = Options::ENABLE_STRIKETHROUGH;
    let parser = Parser::new_ext(text, options);

    let mut top_inlines: Vec<Inline> = Vec::new();
    let mut inline_stack: Vec<InlineContainer> = Vec::new();
    let mut children_stack: Vec<Vec<Inline>> = Vec::new();

    for event in parser {
        match event {
            Event::Start(Tag::Strong) => {
                inline_stack.push(InlineContainer::Strong);
                children_stack.push(Vec::new());
            }
            Event::End(TagEnd::Strong) => {
                inline_stack.pop();
                let children = children_stack.pop().unwrap_or_default();
                emit_inline(
                    &mut top_inlines,
                    &mut children_stack,
                    Inline::Strong(children),
                );
            }
            Event::Start(Tag::Emphasis) => {
                inline_stack.push(InlineContainer::Emphasis);
                children_stack.push(Vec::new());
            }
            Event::End(TagEnd::Emphasis) => {
                inline_stack.pop();
                let children = children_stack.pop().unwrap_or_default();
                emit_inline(
                    &mut top_inlines,
                    &mut children_stack,
                    Inline::Emphasis(children),
                );
            }
            Event::Start(Tag::Strikethrough) => {
                inline_stack.push(InlineContainer::Strikethrough);
                children_stack.push(Vec::new());
            }
            Event::End(TagEnd::Strikethrough) => {
                inline_stack.pop();
                let children = children_stack.pop().unwrap_or_default();
                emit_inline(
                    &mut top_inlines,
                    &mut children_stack,
                    Inline::Strikethrough(children),
                );
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                inline_stack.push(InlineContainer::Link {
                    url: dest_url.to_string(),
                });
                children_stack.push(Vec::new());
            }
            Event::End(TagEnd::Link) => {
                let container = inline_stack.pop();
                let children = children_stack.pop().unwrap_or_default();
                if let Some(InlineContainer::Link { url }) = container {
                    emit_inline(
                        &mut top_inlines,
                        &mut children_stack,
                        Inline::Link {
                            text: children,
                            url,
                        },
                    );
                }
            }
            Event::Code(text) => {
                emit_inline(
                    &mut top_inlines,
                    &mut children_stack,
                    Inline::Code(text.to_string()),
                );
            }
            Event::Text(text) => {
                emit_inline(
                    &mut top_inlines,
                    &mut children_stack,
                    Inline::Text(text.to_string()),
                );
            }
            Event::SoftBreak => {
                emit_inline(&mut top_inlines, &mut children_stack, Inline::SoftBreak);
            }
            Event::HardBreak => {
                emit_inline(&mut top_inlines, &mut children_stack, Inline::HardBreak);
            }
            _ => {}
        }
    }

    top_inlines
}

/// 将 inline 元素推入当前目标容器
///
/// 如果 children_stack 非空（即在嵌套容器内），push 到栈顶的子容器。
/// 否则 push 到顶层的 top_inlines。
fn emit_inline(top_inlines: &mut Vec<Inline>, children_stack: &mut [Vec<Inline>], inline: Inline) {
    if let Some(parent_children) = children_stack.last_mut() {
        parent_children.push(inline);
    } else {
        top_inlines.push(inline);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_inline_bold_basic() {
        let text = "**关键词**：大语言模型";
        let inlines = parse_inline_text(text);

        // 应该解析出：Strong(["关键词"]) + Text("：大语言模型")
        assert_eq!(inlines.len(), 2);

        match &inlines[0] {
            Inline::Strong(children) => {
                assert_eq!(children.len(), 1);
                match &children[0] {
                    Inline::Text(s) => assert_eq!(s, "关键词"),
                    _ => panic!("Expected Text inside Strong"),
                }
            }
            _ => panic!("Expected Strong, got {:?}", inlines[0]),
        }

        match &inlines[1] {
            Inline::Text(s) => assert!(s.contains("：大语言模型")),
            _ => panic!("Expected Text, got {:?}", inlines[1]),
        }
    }

    #[test]
    fn parse_inline_bold_with_chinese() {
        let text = "第一，**上下文窗口受限**——一个真实的中型业务系统";
        let inlines = parse_inline_text(text);

        let strong_idx = inlines.iter().position(|i| matches!(i, Inline::Strong(_)));
        assert!(strong_idx.is_some(), "Should find Strong in: {:?}", inlines);
    }

    #[test]
    fn parse_inline_nested() {
        let text = "**bold *italic* bold**";
        let inlines = parse_inline_text(text);

        assert_eq!(inlines.len(), 1);
        match &inlines[0] {
            Inline::Strong(children) => {
                assert_eq!(children.len(), 3);
                match &children[1] {
                    Inline::Emphasis(inner) => {
                        assert_eq!(inner.len(), 1);
                        match &inner[0] {
                            Inline::Text(s) => assert_eq!(s, "italic"),
                            _ => panic!("Expected Text inside Emphasis"),
                        }
                    }
                    _ => panic!("Expected Emphasis, got {:?}", children[1]),
                }
            }
            _ => panic!("Expected Strong, got {:?}", inlines[0]),
        }
    }

    #[test]
    fn parse_inline_code() {
        let text = "使用 `code` 标记";
        let inlines = parse_inline_text(text);

        assert_eq!(inlines.len(), 3);
        match &inlines[1] {
            Inline::Code(s) => assert_eq!(s, "code"),
            _ => panic!("Expected Code, got {:?}", inlines[1]),
        }
    }

    #[test]
    fn parse_inline_bold_in_list_item() {
        // 模拟有序列表项内容：1. **跨模块依赖**：前端组件...
        let text = "**跨模块依赖**：前端组件、后端接口、数据库表、配置文件之间存在显式或隐式的数据流依赖";
        let inlines = parse_inline_text(text);

        // 应该解析出：Strong(["跨模块依赖"]) + Text("：前端组件...")
        assert!(
            inlines.len() >= 2,
            "Expected at least 2 inlines, got {:?}",
            inlines
        );

        match &inlines[0] {
            Inline::Strong(children) => {
                assert_eq!(children.len(), 1);
                match &children[0] {
                    Inline::Text(s) => assert_eq!(s, "跨模块依赖"),
                    _ => panic!("Expected Text inside Strong"),
                }
            }
            _ => panic!("Expected Strong, got {:?}", inlines[0]),
        }
    }

    #[test]
    fn parse_inline_bold_with_colon() {
        // 测试 **xxx**：模式（中文冒号）
        let text = "**模板代码占比高**：在常见企业业务系统中";
        let inlines = parse_inline_text(text);

        let strong_idx = inlines
            .iter()
            .position(|i| matches!(i, Inline::Strong(_)));
        assert!(
            strong_idx.is_some(),
            "Should find Strong in: {:?}",
            inlines
        );
    }
}
