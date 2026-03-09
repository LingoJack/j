use crate::command::chat::tools::{Tool, ToolResult};
use serde_json::{Value, json};
use std::sync::{Arc, atomic::AtomicBool};
use std::time::Duration;

/// 网络搜索工具（使用 Brave Search API）
pub struct WebSearchTool;

/// 请求超时时间（秒）
const REQUEST_TIMEOUT_SECS: u64 = 15;
/// 默认搜索结果数量
const DEFAULT_COUNT: usize = 5;
/// 最大结果数量
const MAX_COUNT: usize = 10;

impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "使用 Brave Search API 进行网络搜索，返回相关网页结果。适用于查找信息、搜索资料、获取最新资讯等场景。需要设置 BRAVE_API_KEY 环境变量。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "搜索关键词或问题"
                },
                "count": {
                    "type": "integer",
                    "default": 5,
                    "minimum": 1,
                    "maximum": 10,
                    "description": "返回结果数量（1-10）"
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
                    "description": "时间范围筛选：pd(24小时)、pw(一周)、pm(一月)、py(一年)"
                }
            },
            "required": ["query"]
        })
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let v = match serde_json::from_str::<Value>(arguments) {
            Ok(v) => v,
            Err(e) => {
                return ToolResult {
                    output: format!("参数解析失败: {}", e),
                    is_error: true,
                };
            }
        };

        let query = match v.get("query").and_then(|q| q.as_str()) {
            Some(q) => q,
            None => {
                return ToolResult {
                    output: "参数缺少 query 字段".to_string(),
                    is_error: true,
                };
            }
        };

        let count = v
            .get("count")
            .and_then(|c| c.as_u64())
            .map(|c| c as usize)
            .unwrap_or(DEFAULT_COUNT)
            .min(MAX_COUNT)
            .max(1);

        let country = v.get("country").and_then(|c| c.as_str()).unwrap_or("CN");

        let search_lang = v.get("search_lang").and_then(|l| l.as_str());
        let freshness = v.get("freshness").and_then(|f| f.as_str());

        search_web(query, count, country, search_lang, freshness)
    }

    fn requires_confirmation(&self) -> bool {
        false
    }
}

fn search_web(
    query: &str,
    count: usize,
    country: &str,
    search_lang: Option<&str>,
    freshness: Option<&str>,
) -> ToolResult {
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

    // 构建 HTTP 客户端
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

    // 发送请求
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

    // 检查状态码
    let status = response.status();
    if !status.is_success() {
        let body = response.text().unwrap_or_default();
        return ToolResult {
            output: format!("Brave Search API 错误 {}: {}", status.as_u16(), body),
            is_error: true,
        };
    }

    // 解析响应
    let data: Value = match response.json() {
        Ok(d) => d,
        Err(e) => {
            return ToolResult {
                output: format!("解析 Brave Search 响应失败: {}", e),
                is_error: true,
            };
        }
    };

    // 提取搜索结果
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

    // 格式化输出
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
            // 限制描述长度
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
