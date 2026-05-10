use crate::constants::{
    WEB_REQUEST_TIMEOUT_SECS, WEB_RESPONSE_DEFAULT_MAX_CHARS, WEB_RESPONSE_MAX_BYTES,
};
use crate::tools::{PlanDecision, Tool, ToolResult, parse_tool_args, schema_to_tool_params};
use crate::util::html_extract;
use schemars::JsonSchema;
use scraper::Html;
use serde::Deserialize;
use serde_json::Value;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, atomic::AtomicBool};
use std::time::Duration;

/// WebFetchTool 参数
#[derive(Deserialize, JsonSchema)]
struct WebFetchParams {
    /// Target URL (must start with http:// or https://)
    url: String,
    /// Output format: markdown or text
    #[serde(default = "default_extract_mode")]
    extract_mode: String,
    /// Maximum number of characters to return
    #[serde(default = "default_max_chars")]
    max_chars: usize,
    /// Authorization header value
    #[serde(default)]
    authorization: Option<String>,
    /// Custom request headers
    #[serde(default)]
    headers: Option<HashMap<String, String>>,
}

fn default_extract_mode() -> String {
    "markdown".to_string()
}

fn default_max_chars() -> usize {
    WEB_RESPONSE_DEFAULT_MAX_CHARS
}

// ==================== WebFetchTool ====================

/// HTTP 抓取网页工具
#[derive(Debug)]
pub struct WebFetchTool;

impl WebFetchTool {
    pub const NAME: &'static str = "WebFetch";
}

impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn description(&self) -> Cow<'_, str> {
        r#"
        Fetches content from a specified URL, converts HTML to Markdown or plain text.

        Usage notes:
        - The URL must be a fully-formed valid URL starting with http:// or https://
        - The tool is read-only and does not modify any files
        - Results may be truncated if the content is very large
        - Supports custom headers and authorization for authenticated APIs
        - For GitHub URLs, prefer using the `gh` CLI via Bash instead (e.g., gh pr view, gh issue view, gh api)
        "#.into()
    }

    fn parameters_schema(&self) -> Value {
        schema_to_tool_params::<WebFetchParams>()
    }

    fn execute(&self, arguments: &str, cancelled: &Arc<AtomicBool>) -> ToolResult {
        let params: WebFetchParams = match parse_tool_args(arguments) {
            Ok(p) => p,
            Err(e) => return e,
        };

        exec_fetch(&params, cancelled)
    }

    fn requires_confirmation(&self) -> bool {
        false
    }
}

// ==================== Fetch 实现 ====================

/// 构建带 UA 和超时的 blocking HTTP 客户端
fn build_http_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(WEB_REQUEST_TIMEOUT_SECS))
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))
}

/// 发送 HTTP GET 请求，附带 authorization 和 custom headers
fn send_http_request(
    client: &reqwest::blocking::Client,
    params: &WebFetchParams,
) -> Result<reqwest::blocking::Response, String> {
    let mut request = client.get(&params.url).header("Referer", &params.url);

    if let Some(ref auth) = params.authorization {
        request = request.header("Authorization", auth.as_str());
    }
    if let Some(ref custom_headers) = params.headers {
        for (key, value) in custom_headers {
            request = request.header(key.as_str(), value.as_str());
        }
    }

    request.send().map_err(|e| format!("请求失败: {}", e))
}

/// 从 HTTP 响应提取文本内容（HTML → markdown/text，截断到 max_chars）
fn extract_text_from_response(
    response: reqwest::blocking::Response,
    params: &WebFetchParams,
) -> ToolResult {
    let status = response.status();
    if !status.is_success() {
        return ToolResult {
            output: format!(
                "HTTP 请求返回错误状态码: {} {}",
                status.as_u16(),
                status.canonical_reason().unwrap_or("")
            ),
            is_error: true,
            images: vec![],
            plan_decision: PlanDecision::None,
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
            images: vec![],
            plan_decision: PlanDecision::None,
        };
    }

    let body = match read_response_body(response) {
        Ok(b) => b,
        Err(e) => {
            return ToolResult {
                output: e,
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            };
        }
    };

    let text = if is_html || (!is_text && content_type.is_empty()) {
        let document = Html::parse_document(&body);
        let content_html = html_extract::extract_readable_content(&document);
        match params.extract_mode.as_str() {
            "text" => html_extract::html_to_text(&content_html),
            _ => html2md::parse_html(&content_html),
        }
    } else {
        body
    };

    let truncated = if text.len() > params.max_chars {
        let mut end = params.max_chars;
        while end > 0 && !text.is_char_boundary(end) {
            end -= 1;
        }
        format!(
            "{}...\n\n[内容已截断，原长度: {} 字符]",
            &text[..end],
            text.len()
        )
    } else {
        text
    };

    ToolResult {
        output: format!("[来源: {}]\n\n{}", params.url, truncated),
        is_error: false,
        images: vec![],
        plan_decision: PlanDecision::None,
    }
}

fn exec_fetch(params: &WebFetchParams, cancelled: &Arc<AtomicBool>) -> ToolResult {
    if cancelled.load(std::sync::atomic::Ordering::Relaxed) {
        return ToolResult {
            output: "操作已取消".to_string(),
            is_error: true,
            images: vec![],
            plan_decision: PlanDecision::None,
        };
    }

    let client = match build_http_client() {
        Ok(c) => c,
        Err(e) => {
            return ToolResult {
                output: e,
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            };
        }
    };

    let response = match send_http_request(&client, params) {
        Ok(r) => r,
        Err(e) => {
            return ToolResult {
                output: e,
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            };
        }
    };

    extract_text_from_response(response, params)
}

// ==================== HTML 解析辅助函数 ====================

/// 读取响应体，超过限制则截断
fn read_response_body(response: reqwest::blocking::Response) -> Result<String, String> {
    if let Some(len) = response.content_length()
        && len as usize > WEB_RESPONSE_MAX_BYTES
    {
        return Err(format!(
            "响应体过大（{:.1} MB），超过 {} MB 限制",
            len as f64 / 1024.0 / 1024.0,
            WEB_RESPONSE_MAX_BYTES / 1024 / 1024
        ));
    }

    match response.text() {
        Ok(text) => {
            if text.len() > WEB_RESPONSE_MAX_BYTES {
                let mut end = WEB_RESPONSE_MAX_BYTES;
                while end > 0 && !text.is_char_boundary(end) {
                    end -= 1;
                }
                Ok(text[..end].to_string())
            } else {
                Ok(text)
            }
        }
        Err(e) => Err(format!("读取响应体失败: {}", e)),
    }
}
