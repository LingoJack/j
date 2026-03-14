//! HTML 内容智能提取工具模块
//!
//! 提供从 HTML 中提取可读正文内容的公共函数，
//! 被 `web_fetch` 和 `browser` 模块共同使用。

use scraper::{Html, Selector};

/// 智能提取网页正文区域的 HTML
///
/// 按优先级尝试 article / main / 常见内容 class，
/// 匹配不到则回退到 `<body>`，最后回退到整个文档。
pub fn extract_readable_content(document: &Html) -> String {
    let content_selectors = [
        "article",
        "main",
        "[role=\"main\"]",
        ".post-content",
        ".article-content",
        ".entry-content",
        ".content",
        "#content",
        ".post",
        ".article",
    ];

    for selector_str in content_selectors {
        if let Ok(selector) = Selector::parse(selector_str)
            && let Some(element) = document.select(&selector).next()
        {
            return element.html();
        }
    }

    if let Ok(body_selector) = Selector::parse("body")
        && let Some(body) = document.select(&body_selector).next()
    {
        return body.html();
    }

    document.html()
}

/// 将 HTML 转换为干净的纯文本
///
/// - 跳过 script / style / nav / header / footer / aside / noscript
/// - 在块级元素处插入换行以保持可读性
/// - 去除空行和多余空白
pub fn html_to_text(html: &str) -> String {
    let document = Html::parse_fragment(html);
    let mut text = String::new();

    fn extract_text(node: scraper::ElementRef, text: &mut String) {
        for child in node.children() {
            if let Some(element) = scraper::ElementRef::wrap(child) {
                let tag = element.value().name();
                if matches!(
                    tag,
                    "script" | "style" | "nav" | "header" | "footer" | "aside" | "noscript"
                ) {
                    continue;
                }
                if matches!(
                    tag,
                    "p" | "div" | "br" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "li" | "tr"
                ) {
                    text.push('\n');
                }
                extract_text(element, text);
            } else if let Some(t) = child.value().as_text() {
                let trimmed = t.trim();
                if !trimmed.is_empty() {
                    if !text.is_empty() && !text.ends_with('\n') && !text.ends_with(' ') {
                        text.push(' ');
                    }
                    text.push_str(trimmed);
                }
            }
        }
    }

    if let Ok(root_selector) = Selector::parse(":root")
        && let Some(root) = document.select(&root_selector).next()
    {
        extract_text(root, &mut text);
    }

    text.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// 从原始 HTML 中提取可读正文并转为纯文本
///
/// 组合 `extract_readable_content` + `html_to_text` 的便捷方法。
pub fn extract_text_from_html(raw_html: &str) -> String {
    let document = Html::parse_document(raw_html);
    let content_html = extract_readable_content(&document);
    html_to_text(&content_html)
}
