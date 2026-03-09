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

// ==================== WebSearchTool ====================

/// Brave Search API 搜索工具
pub struct WebSearchTool;

impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "使用 Brave Search API 搜索网络。需要设置 BRAVE_API_KEY 环境变量。"
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
                "country": {
                    "type": "string",
                    "default": "CN",
                    "description": "搜索国家/地区代码（如 CN、US、JP）"
                },
                "search_lang": {
                    "type": "string",
                    "description": "搜索语言代码（如 zh-hans、en、ja）"
                },
                "freshness": {
                    "type": "string",
                    "enum": ["pd", "pw", "pm", "py"],
                    "description": "时间范围：pd(24小时) pw(一周) pm(一月) py(一年)"
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

    let country = args.get("country").and_then(|c| c.as_str()).unwrap_or("CN");
    let search_lang = args.get("search_lang").and_then(|l| l.as_str());
    let freshness = args.get("freshness").and_then(|f| f.as_str());

    // 检查 API Key
    let api_key = match std::env::var("BRAVE_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            return ToolResult {
                output: "未设置 BRAVE_API_KEY 环境变量。请在 https://brave.com/search/api/ 获取免费 API Key 并设置环境变量。".to_string(),
                is_error: true,
            };
        }
    };

    // 构建 API URL
    let mut url = format!(
        "https://api.search.brave.com/res/v1/web/search?q={}&count={}",
        urlencoding::encode(query),
        count,
    );
    if country != "ALL" {
        url.push_str(&format!("&country={}", country));
    }
    if let Some(lang) = search_lang {
        url.push_str(&format!("&search_lang={}", lang));
    }
    if let Some(fresh) = freshness {
        url.push_str(&format!("&freshness={}", fresh));
    }

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
        .get(&url)
        .header("Accept", "application/json")
        .header("Accept-Encoding", "gzip")
        .header("X-Subscription-Token", &api_key)
        .send()
    {
        Ok(r) => r,
        Err(e) => {
            return ToolResult {
                output: format!("Brave Search 请求失败: {}", e),
                is_error: true,
            };
        }
    };

    let status = response.status();
    if !status.is_success() {
        let body = response.text().unwrap_or_default();
        return ToolResult {
            output: format!("Brave Search API 错误 {}: {}", status.as_u16(), body),
            is_error: true,
        };
    }

    let data: Value = match response.json() {
        Ok(d) => d,
        Err(e) => {
            return ToolResult {
                output: format!("解析 Brave Search 响应失败: {}", e),
                is_error: true,
            };
        }
    };

    let web_results = data
        .get("web")
        .and_then(|w| w.get("results"))
        .and_then(|r| r.as_array());

    let Some(results) = web_results else {
        return ToolResult {
            output: "未找到搜索结果".to_string(),
            is_error: false,
        };
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
        let description = result
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("");

        output.push_str(&format!("{}. {}\n", i + 1, title));
        output.push_str(&format!("   {}\n", url));
        if !description.is_empty() {
            let desc = if description.len() > 200 {
                format!("{}...", &description[..200])
            } else {
                description.to_string()
            };
            output.push_str(&format!("   {}\n", desc));
        }
        output.push('\n');
    }

    ToolResult {
        output,
        is_error: false,
    }
}
