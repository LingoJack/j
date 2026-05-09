use crate::command::chat::constants::{
    WEB_REQUEST_TIMEOUT_SECS, WEB_SEARCH_DEFAULT_COUNT, WEB_SEARCH_HIGHLIGHTS_MAX_CHARS,
    WEB_SEARCH_MAX_COUNT,
};
use crate::command::chat::tools::{
    PlanDecision, Tool, ToolResult, parse_tool_args, schema_to_tool_params,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use std::borrow::Cow;
use std::sync::{Arc, atomic::AtomicBool};
use std::time::Duration;

/// Exa API 端点
const EXA_API_URL: &str = "https://api.exa.ai/search";

/// WebSearchTool 参数
#[derive(Deserialize, JsonSchema)]
struct WebSearchParams {
    /// Search keywords
    query: String,
    /// Number of search results (1-10, default 5)
    #[serde(default = "default_count")]
    count: usize,
    /// Search type: auto, keyword, or neural (semantic)
    #[serde(default = "default_search_type", rename = "type")]
    search_type: String,
}

fn default_count() -> usize {
    WEB_SEARCH_DEFAULT_COUNT
}

fn default_search_type() -> String {
    "auto".to_string()
}

// ==================== WebSearchTool ====================

/// Exa Search API 搜索工具
#[derive(Debug)]
pub struct WebSearchTool;

impl WebSearchTool {
    pub const NAME: &'static str = "WebSearch";
}

impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn description(&self) -> Cow<'_, str> {
        r#"
        Search the web for up-to-date information. Requires the EXA_API_KEY environment variable.

        Usage notes:
        - Use this tool for accessing information beyond your knowledge cutoff
        - After answering the user's question with search results, you SHOULD include a "Sources:" section listing relevant URLs
        - Returns search results with titles, URLs, and highlighted snippets
        "#.into()
    }

    fn parameters_schema(&self) -> Value {
        schema_to_tool_params::<WebSearchParams>()
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let params: WebSearchParams = match parse_tool_args(arguments) {
            Ok(p) => p,
            Err(e) => return e,
        };

        exec_search(&params)
    }

    fn requires_confirmation(&self) -> bool {
        false
    }
}

// ==================== Search 实现 ====================

fn exec_search(params: &WebSearchParams) -> ToolResult {
    let count = params.count.clamp(1, WEB_SEARCH_MAX_COUNT);

    let api_key = match std::env::var("EXA_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            return ToolResult {
                output: "未设置 EXA_API_KEY 环境变量。请在 https://exa.ai/ 获取 API Key 并设置环境变量。".to_string(),
                is_error: true,
                    images: vec![],
                plan_decision: PlanDecision::None,
            };
        }
    };

    match send_search_request(&api_key, params, count) {
        Ok(results) => format_search_results(&params.query, count, &results),
        Err(err_msg) => ToolResult {
            output: err_msg,
            is_error: true,
            images: vec![],
            plan_decision: PlanDecision::None,
        },
    }
}

/// 发送搜索请求，返回 results 数组或错误消息
fn send_search_request(
    api_key: &str,
    params: &WebSearchParams,
    count: usize,
) -> Result<Vec<Value>, String> {
    let request_body = json!({
        "query": params.query,
        "type": params.search_type,
        "numResults": count,
        "contents": {
            "highlights": {
                "maxCharacters": WEB_SEARCH_HIGHLIGHTS_MAX_CHARS
            }
        }
    });

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(WEB_REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

    let response = client
        .post(EXA_API_URL)
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .header("x-api-key", api_key)
        .json(&request_body)
        .send()
        .map_err(|e| format!("Exa Search 请求失败: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().unwrap_or_default();
        return Err(format!("Exa Search API 错误 {}: {}", status.as_u16(), body));
    }

    let data: Value = response
        .json()
        .map_err(|e| format!("解析 Exa Search 响应失败: {}", e))?;

    data.get("results")
        .and_then(|r| r.as_array())
        .cloned()
        .ok_or_else(|| "未找到搜索结果".to_string())
}

/// 将搜索结果格式化为文本输出
fn format_search_results(query: &str, count: usize, results: &[Value]) -> ToolResult {
    if results.is_empty() {
        return ToolResult {
            output: "未找到搜索结果".to_string(),
            is_error: false,
            images: vec![],
            plan_decision: PlanDecision::None,
        };
    }

    let mut output = format!("搜索: {}\n\n", query);
    for (i, result) in results.iter().take(count).enumerate() {
        let title = result
            .get("title")
            .and_then(|t| t.as_str())
            .unwrap_or("(无标题)");
        let url = result.get("url").and_then(|u| u.as_str()).unwrap_or("");

        output.push_str(&format!("{}. {}\n", i + 1, title));
        output.push_str(&format!("   {}\n", url));

        if let Some(highlights) = result.get("highlights").and_then(|h| h.as_array()) {
            for highlight in highlights {
                if let Some(text) = highlight.as_str() {
                    let desc = if text.chars().count() > 200 {
                        let end = text
                            .char_indices()
                            .nth(200)
                            .map(|(i, _)| i)
                            .unwrap_or(text.len());
                        format!("{}...", &text[..end])
                    } else {
                        text.to_string()
                    };
                    output.push_str(&format!("   {}\n", desc));
                }
            }
        }
        output.push('\n');
    }

    ToolResult {
        output,
        is_error: false,
        images: vec![],
        plan_decision: PlanDecision::None,
    }
}
