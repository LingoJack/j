use crate::command::chat::tools::{Tool, ToolResult};
use scraper::{Html, Selector};
use serde_json::{Value, json};
use std::sync::{Arc, atomic::AtomicBool};
use std::time::Duration;

// ==================== 常量 ====================

/// 请求超时时间（秒）
const REQUEST_TIMEOUT_SECS: u64 = 15;
/// 最大响应体大小（字节）：1MB
const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
/// 默认最大输出字符数
const DEFAULT_MAX_CHARS: usize = 50000;

// ==================== WebFetchTool ====================

/// HTTP 抓取网页工具
pub struct WebFetchTool;

impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "web_fetch"
    }

    fn description(&self) -> &str {
        "获取网页内容并转为 Markdown 或纯文本。支持自定义请求头和授权信息。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "目标 URL（必须以 http:// 或 https:// 开头）"
                },
                "extract_mode": {
                    "type": "string",
                    "enum": ["markdown", "text"],
                    "default": "markdown",
                    "description": "输出格式：markdown 或 text"
                },
                "max_chars": {
                    "type": "integer",
                    "default": 50000,
                    "description": "最大返回字符数"
                },
                "authorization": {
                    "type": "string",
                    "description": "Authorization 请求头"
                },
                "headers": {
                    "type": "object",
                    "description": "自定义请求头",
                    "additionalProperties": { "type": "string" }
                }
            },
            "required": ["url"]
        })
    }

    fn execute(&self, arguments: &str, cancelled: &Arc<AtomicBool>) -> ToolResult {
        let args: Value = match serde_json::from_str(arguments) {
            Ok(v) => v,
            Err(e) => {
                return ToolResult {
                    output: format!("参数解析失败: {}", e),
                    is_error: true,
                };
            }
        };

        exec_fetch(&args, cancelled)
    }

    fn requires_confirmation(&self) -> bool {
        false
    }
}

// ==================== Fetch 实现 ====================

fn exec_fetch(args: &Value, cancelled: &Arc<AtomicBool>) -> ToolResult {
    let url = match args.get("url").and_then(|u| u.as_str()) {
        Some(u) => u,
        None => {
            return ToolResult {
                output: "缺少 url 参数".to_string(),
                is_error: true,
            };
        }
    };

    let extract_mode = args
        .get("extract_mode")
        .and_then(|m| m.as_str())
        .unwrap_or("markdown");

    let max_chars = args
        .get("max_chars")
        .and_then(|c| c.as_u64())
        .map(|c| c as usize)
        .unwrap_or(DEFAULT_MAX_CHARS);

    let authorization = args
        .get("authorization")
        .and_then(|a| a.as_str())
        .map(|s| s.to_string());

    let headers = args.get("headers").and_then(|h| h.as_object()).map(|obj| {
        obj.iter()
            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
            .collect::<Vec<_>>()
    });

    if cancelled.load(std::sync::atomic::Ordering::Relaxed) {
        return ToolResult {
            output: "操作已取消".to_string(),
            is_error: true,
        };
    }

    // 构建 HTTP 客户端
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
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

    let mut request = client.get(url).header("Referer", url);

    if let Some(ref auth) = authorization {
        request = request.header("Authorization", auth.as_str());
    }
    if let Some(ref custom_headers) = headers {
        for (key, value) in custom_headers {
            request = request.header(key.as_str(), value.as_str());
        }
    }

    let response = match request.send() {
        Ok(r) => r,
        Err(e) => {
            return ToolResult {
                output: format!("请求失败: {}", e),
                is_error: true,
            };
        }
    };

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

    let body = match read_response_body(response) {
        Ok(b) => b,
        Err(e) => {
            return ToolResult {
                output: e,
                is_error: true,
            };
        }
    };

    let text = if is_html || (!is_text && content_type.is_empty()) {
        let document = Html::parse_document(&body);
        let content_html = extract_readable_content(&document);
        match extract_mode {
            "text" => html_to_text(&content_html),
            _ => html2md::parse_html(&content_html),
        }
    } else {
        body
    };

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

// ==================== HTML 解析辅助函数 ====================

/// 读取响应体，超过限制则截断
fn read_response_body(response: reqwest::blocking::Response) -> Result<String, String> {
    if let Some(len) = response.content_length()
        && len as usize > MAX_RESPONSE_BYTES
    {
        return Err(format!(
            "响应体过大（{:.1} MB），超过 {} MB 限制",
            len as f64 / 1024.0 / 1024.0,
            MAX_RESPONSE_BYTES / 1024 / 1024
        ));
    }

    match response.text() {
        Ok(text) => {
            if text.len() > MAX_RESPONSE_BYTES {
                Ok(text[..MAX_RESPONSE_BYTES].to_string())
            } else {
                Ok(text)
            }
        }
        Err(e) => Err(format!("读取响应体失败: {}", e)),
    }
}

/// 智能提取网页正文内容
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

/// 将 HTML 转换为纯文本
fn html_to_text(html: &str) -> String {
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
