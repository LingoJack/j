use crate::command::chat::tools::{Tool, ToolResult};
use serde_json::{Value, json};
use std::sync::{Arc, atomic::AtomicBool};
use std::time::Duration;

// ==================== 常量 ====================

/// 请求超时时间（秒）
const REQUEST_TIMEOUT_SECS: u64 = 15;
/// 默认搜索结果数量
const DEFAULT_SEARCH_COUNT: usize = 5;
/// 最大搜索结果数量
const MAX_SEARCH_COUNT: usize = 10;
/// Exa API 端点
const EXA_API_URL: &str = "https://api.exa.ai/search";
/// highlights 最大字符数
const HIGHLIGHTS_MAX_CHARS: usize = 4000;

// ==================== WebSearchTool ====================

/// Exa Search API 搜索工具
pub struct WebSearchTool;

impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "使用 Exa Search API 搜索网络。需要设置 EXA_API_KEY 环境变量。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "搜索关键词"
                },
                "count": {
                    "type": "integer",
                    "default": 5,
                    "minimum": 1,
                    "maximum": 10,
                    "description": "搜索结果数量"
                },
                "type": {
                    "type": "string",
                    "enum": ["auto", "keyword", "neural"],
                    "default": "auto",
                    "description": "搜索类型：auto(自动) keyword(关键词) neural(语义)"
                }
            },
            "required": ["query"]
        })
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let args: Value = match serde_json::from_str(arguments) {
            Ok(v) => v,
            Err(e) => {
                return ToolResult {
                    output: format!("参数解析失败: {}", e),
                    is_error: true,
                };
            }
        };

        exec_search(&args)
    }

    fn requires_confirmation(&self) -> bool {
        false
    }
}

// ==================== Search 实现 ====================

fn exec_search(args: &Value) -> ToolResult {
    let query = match args.get("query").and_then(|q| q.as_str()) {
        Some(q) => q,
        None => {
            return ToolResult {
                output: "缺少 query 参数".to_string(),
                is_error: true,
            };
        }
    };

    let count = args
        .get("count")
        .and_then(|c| c.as_u64())
        .map(|c| c as usize)
        .unwrap_or(DEFAULT_SEARCH_COUNT)
        .clamp(1, MAX_SEARCH_COUNT);

    let search_type = args
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("auto");

    // 检查 API Key
    let api_key = match std::env::var("EXA_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            return ToolResult {
                output: "未设置 EXA_API_KEY 环境变量。请在 https://exa.ai/ 获取 API Key 并设置环境变量。".to_string(),
                is_error: true,
            };
        }
    };

    // 构建请求体
    let request_body = json!({
        "query": query,
        "type": search_type,
        "numResults": count,
        "contents": {
            "highlights": {
                "maxCharacters": HIGHLIGHTS_MAX_CHARS
            }
        }
    });

    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
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

    let response = match client
        .post(EXA_API_URL)
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .header("x-api-key", &api_key)
        .json(&request_body)
        .send()
    {
        Ok(r) => r,
        Err(e) => {
            return ToolResult {
                output: format!("Exa Search 请求失败: {}", e),
                is_error: true,
            };
        }
    };

    let status = response.status();
    if !status.is_success() {
        let body = response.text().unwrap_or_default();
        return ToolResult {
            output: format!("Exa Search API 错误 {}: {}", status.as_u16(), body),
            is_error: true,
        };
    }

    let data: Value = match response.json() {
        Ok(d) => d,
        Err(e) => {
            return ToolResult {
                output: format!("解析 Exa Search 响应失败: {}", e),
                is_error: true,
            };
        }
    };

    let results = match data.get("results").and_then(|r| r.as_array()) {
        Some(r) => r,
        None => {
            return ToolResult {
                output: "未找到搜索结果".to_string(),
                is_error: false,
            };
        }
    };

    if results.is_empty() {
        return ToolResult {
            output: "未找到搜索结果".to_string(),
            is_error: false,
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

        // 提取 highlights
        if let Some(highlights) = result.get("highlights").and_then(|h| h.as_array()) {
            for highlight in highlights {
                if let Some(text) = highlight.as_str() {
                    let desc = if text.chars().count() > 200 {
                        let end = text.char_indices().nth(200).map(|(i, _)| i).unwrap_or(text.len());
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
    }
}
