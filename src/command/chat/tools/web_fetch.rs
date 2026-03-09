use crate::command::chat::tools::{Tool, ToolResult};
use scraper::{Html, Selector};
use serde_json::{Value, json};
use std::sync::{Arc, atomic::AtomicBool};

/// 获取网页内容的工具
pub struct WebFetchTool;

/// 请求超时时间（秒）
const REQUEST_TIMEOUT_SECS: u64 = 15;
/// 最大响应体大小（字节）：1MB
const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
/// 默认最大输出字符数
const DEFAULT_MAX_CHARS: usize = 50000;
/// html2text 文本折行宽度（字符数）
const TEXT_WIDTH: usize = 120;

impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "web_fetch"
    }

    fn description(&self) -> &str {
        "获取指定 URL 的网页内容，智能提取正文并转换为 Markdown 或纯文本返回。适用于阅读文章、文档、查看网页信息等场景。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "要获取的完整 URL（如 https://example.com）"
                },
                "extract_mode": {
                    "type": "string",
                    "enum": ["markdown", "text"],
                    "default": "markdown",
                    "description": "输出格式：markdown 保留结构更适合 LLM 理解，text 为纯文本"
                },
                "max_chars": {
                    "type": "integer",
                    "default": 50000,
                    "description": "返回内容的最大字符数，超出将截断"
                }
            },
            "required": ["url"]
        })
    }

    fn execute(&self, arguments: &str, cancelled: &Arc<AtomicBool>) -> ToolResult {
        let v = match serde_json::from_str::<Value>(arguments) {
            Ok(v) => v,
            Err(e) => {
                return ToolResult {
                    output: format!("参数解析失败: {}", e),
                    is_error: true,
                };
            }
        };

        let url = match v.get("url").and_then(|u| u.as_str()) {
            Some(u) => u,
            None => {
                return ToolResult {
                    output: "参数缺少 url 字段".to_string(),
                    is_error: true,
                };
            }
        };

        let extract_mode = v
            .get("extract_mode")
            .and_then(|m| m.as_str())
            .unwrap_or("markdown");

        let max_chars = v
            .get("max_chars")
            .and_then(|c| c.as_u64())
            .map(|c| c as usize)
            .unwrap_or(DEFAULT_MAX_CHARS);

        fetch_url(url, extract_mode, max_chars, cancelled)
    }

    fn requires_confirmation(&self) -> bool {
        false
    }
}

fn fetch_url(
    url: &str,
    extract_mode: &str,
    max_chars: usize,
    cancelled: &Arc<AtomicBool>,
) -> ToolResult {
    // 检查取消
    if cancelled.load(std::sync::atomic::Ordering::Relaxed) {
        return ToolResult {
            output: "操作已取消".to_string(),
            is_error: true,
        };
    }

    // 1. 构建 HTTP 客户端
    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return ToolResult {
                output: format!("创建 HTTP 客户端失败: {}", e),
                is_error: true,
            };
        }
    };

    // 2. 发送 GET 请求（添加 Referer 头模拟浏览器行为）
    let response = match client.get(url).header("Referer", url).send() {
        Ok(r) => r,
        Err(e) => {
            return ToolResult {
                output: format!("请求失败: {}", e),
                is_error: true,
            };
        }
    };

    // 3. 检查 HTTP 状态码
    let status = response.status();
    if !status.is_success() {
        return ToolResult {
            output: format!(
                "HTTP 请求返回错误状态码: {} {}",
                status.as_u16(),
                status.canonical_reason().unwrap_or("")
            ),
            is_error: true,
        };
    }

    // 4. 检查 Content-Type，判断是否为可处理的文本类型
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();

    let is_html = content_type.contains("text/html");
    let is_text = content_type.contains("text/plain");

    if !is_html && !is_text && !content_type.is_empty() {
        return ToolResult {
            output: format!(
                "该 URL 返回的内容类型为 {}，不是 HTML 或纯文本，无法提取文字内容。",
                content_type
            ),
            is_error: true,
        };
    }

    // 5. 读取响应体（限制大小）
    let body = match read_response_body(response) {
        Ok(b) => b,
        Err(e) => {
            return ToolResult {
                output: e,
                is_error: true,
            };
        }
    };

    // 6. 转换为目标格式
    let text = if is_html || (!is_text && content_type.is_empty()) {
        // HTML 或未知类型，智能提取内容
        let document = Html::parse_document(&body);
        let content_html = extract_readable_content(&document);

        match extract_mode {
            "text" => html_to_text(&content_html),
            _ => html2md::parse_html(&content_html),
        }
    } else {
        // 纯文本直接返回
        body
    };

    // 7. 截断到 max_chars
    let truncated = if text.len() > max_chars {
        format!(
            "{}...\n\n[内容已截断，原长度: {} 字符]",
            &text[..max_chars],
            text.len()
        )
    } else {
        text
    };

    ToolResult {
        output: format!("[来源: {}]\n\n{}", url, truncated),
        is_error: false,
    }
}

/// 读取响应体，超过 MAX_RESPONSE_BYTES 则截断
fn read_response_body(response: reqwest::blocking::Response) -> Result<String, String> {
    // 先检查 Content-Length（如果有）
    if let Some(len) = response.content_length() {
        if len as usize > MAX_RESPONSE_BYTES {
            return Err(format!(
                "响应体过大（{:.1} MB），超过 {} MB 限制",
                len as f64 / 1024.0 / 1024.0,
                MAX_RESPONSE_BYTES / 1024 / 1024
            ));
        }
    }

    match response.text() {
        Ok(text) => {
            if text.len() > MAX_RESPONSE_BYTES {
                // 响应体没有 Content-Length 但实际超限，仍然返回（只是提示）
                Ok(text[..MAX_RESPONSE_BYTES].to_string())
            } else {
                Ok(text)
            }
        }
        Err(e) => Err(format!("读取响应体失败: {}", e)),
    }
}

/// 智能提取网页正文内容
/// 按语义优先级查找 article、main 等核心内容区域
fn extract_readable_content(document: &Html) -> String {
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
        if let Ok(selector) = Selector::parse(selector_str) {
            if let Some(element) = document.select(&selector).next() {
                return element.html();
            }
        }
    }

    // 降级到 body
    if let Ok(body_selector) = Selector::parse("body") {
        if let Some(body) = document.select(&body_selector).next() {
            return body.html();
        }
    }

    document.html()
}

/// 将 HTML 转换为纯文本（去除标签，保留文本内容）
/// 过滤 script、style、nav 等噪音元素
fn html_to_text(html: &str) -> String {
    let document = Html::parse_fragment(html);
    let mut text = String::new();

    fn extract_text(node: scraper::ElementRef, text: &mut String) {
        for child in node.children() {
            if let Some(element) = scraper::ElementRef::wrap(child) {
                let tag = element.value().name();
                // 跳过噪音元素
                if matches!(
                    tag,
                    "script" | "style" | "nav" | "header" | "footer" | "aside" | "noscript"
                ) {
                    continue;
                }
                // 块级元素添加换行
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

    if let Ok(root_selector) = Selector::parse(":root") {
        if let Some(root) = document.select(&root_selector).next() {
            extract_text(root, &mut text);
        }
    }

    // 清理多余空白
    text.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}
