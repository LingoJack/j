use crate::command::chat::tools::{Tool, ToolResult};
use scraper::{Html, Selector};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock, atomic::AtomicBool};
use std::time::Duration;

// ==================== 常量 ====================

/// 请求超时时间（秒）
const REQUEST_TIMEOUT_SECS: u64 = 15;
/// 最大响应体大小（字节）：1MB
const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
/// 默认最大输出字符数
const DEFAULT_MAX_CHARS: usize = 50000;
/// 默认搜索结果数量
const DEFAULT_SEARCH_COUNT: usize = 5;
/// 最大搜索结果数量
const MAX_SEARCH_COUNT: usize = 10;

// ==================== WebTool ====================

/// 统一的 Web 工具：搜索、抓取、浏览器 Lite 模式
pub struct WebTool;

impl Tool for WebTool {
    fn name(&self) -> &str {
        "web"
    }

    fn description(&self) -> &str {
        "统一的 Web 工具，支持多种操作：\n\
            - search: 使用 Brave Search API 搜索网络\n\
            - fetch: 获取网页内容并转为 Markdown\n\
            - open: 打开 URL 到浏览器 tab（支持交互元素解析）\n\
            - tabs: 列出已打开的 tab\n\
            - snapshot: 获取页面可交互元素列表（按钮、输入框、链接等）\n\
            - navigate: 将指定 tab 导航到新 URL\n\
            - close: 关闭指定 tab\n\
         通过 action 参数选择操作。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["search", "fetch", "open", "tabs", "snapshot", "navigate", "close"],
                    "description": "操作类型：search(搜索) / fetch(抓取网页) / open(打开tab) / tabs(列出tab) / snapshot(获取页面交互元素) / navigate(导航) / close(关闭tab)"
                },
                "url": {
                    "type": "string",
                    "description": "[fetch/open/navigate] 目标 URL"
                },
                "query": {
                    "type": "string",
                    "description": "[search] 搜索关键词"
                },
                "tab_id": {
                    "type": "string",
                    "description": "[snapshot/navigate/close] 目标 tab ID（不指定则使用最近打开的 tab）"
                },
                "count": {
                    "type": "integer",
                    "default": 5,
                    "minimum": 1,
                    "maximum": 10,
                    "description": "[search] 搜索结果数量"
                },
                "country": {
                    "type": "string",
                    "default": "CN",
                    "description": "[search] 搜索国家/地区代码（如 CN、US、JP）"
                },
                "search_lang": {
                    "type": "string",
                    "description": "[search] 搜索语言代码（如 zh-hans、en、ja）"
                },
                "freshness": {
                    "type": "string",
                    "enum": ["pd", "pw", "pm", "py"],
                    "description": "[search] 时间范围：pd(24小时) pw(一周) pm(一月) py(一年)"
                },
                "extract_mode": {
                    "type": "string",
                    "enum": ["markdown", "text"],
                    "default": "markdown",
                    "description": "[fetch] 输出格式"
                },
                "max_chars": {
                    "type": "integer",
                    "default": 50000,
                    "description": "[fetch] 最大返回字符数"
                },
                "authorization": {
                    "type": "string",
                    "description": "[fetch/open] Authorization 请求头"
                },
                "headers": {
                    "type": "object",
                    "description": "[fetch/open] 自定义请求头",
                    "additionalProperties": { "type": "string" }
                }
            },
            "required": ["action"]
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

        let action = match args.get("action").and_then(|a| a.as_str()) {
            Some(a) => a,
            None => {
                return ToolResult {
                    output: "参数缺少 action 字段。可选值: search, fetch, open, tabs, snapshot, navigate, close".to_string(),
                    is_error: true,
                };
            }
        };

        match action {
            "search" => exec_search(&args),
            "fetch" => exec_fetch(&args, cancelled),
            "open" => exec_open(&args),
            "tabs" => exec_tabs(),
            "snapshot" => exec_snapshot(&args),
            "navigate" => exec_navigate(&args),
            "close" => exec_close(&args),
            _ => ToolResult {
                output: format!(
                    "未知的 action: {}。可选值: search, fetch, open, tabs, snapshot, navigate, close",
                    action
                ),
                is_error: true,
            },
        }
    }

    fn requires_confirmation(&self) -> bool {
        false
    }
}

// ==================== 浏览器 Lite 模式（Tab 管理）====================

/// 一个 Lite 模式的 "tab"：HTTP 抓取 + HTML 解析
struct LiteTab {
    url: String,
    title: String,
    #[allow(dead_code)]
    body: String,
    text_content: String,
    links: Vec<Value>,
    forms: Vec<Value>,
    interactive: Vec<Value>,
}

/// 全局 tab 存储
struct LiteBrowser {
    tabs: HashMap<String, LiteTab>,
    next_id: usize,
}

static LITE_BROWSER: OnceLock<Mutex<LiteBrowser>> = OnceLock::new();

fn browser() -> &'static Mutex<LiteBrowser> {
    LITE_BROWSER.get_or_init(|| {
        Mutex::new(LiteBrowser {
            tabs: HashMap::new(),
            next_id: 0,
        })
    })
}

/// 构建 HTTP 客户端
fn http_client_with(
    authorization: Option<&str>,
    headers: Option<&Vec<(String, String)>>,
) -> Result<reqwest::blocking::Client, String> {
    let builder = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .redirect(reqwest::redirect::Policy::limited(10));

    // 注意：reqwest::blocking::ClientBuilder 不支持 default_headers 的动态 auth/headers
    // 我们在具体请求中添加
    let _ = (&authorization, &headers); // suppress unused

    builder
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))
}

/// 抓取 URL 并构建 LiteTab
fn fetch_tab(
    url: &str,
    authorization: Option<&str>,
    custom_headers: Option<&Vec<(String, String)>>,
) -> Result<LiteTab, String> {
    let client = http_client_with(authorization, custom_headers)?;

    let mut request = client.get(url).header("Referer", url);

    if let Some(auth) = authorization {
        request = request.header("Authorization", auth);
    }
    if let Some(hdrs) = custom_headers {
        for (key, value) in hdrs {
            request = request.header(key.as_str(), value.as_str());
        }
    }

    let resp = request.send().map_err(|e| format!("请求失败: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let body = resp.text().map_err(|e| format!("读取响应失败: {}", e))?;

    let title = extract_tag(&body, "title").unwrap_or_default();
    let links = extract_links(&body);
    let forms = extract_forms(&body);
    let interactive = extract_interactive(&body);
    let text_content = strip_html(&body);

    Ok(LiteTab {
        url: url.to_string(),
        title,
        body,
        text_content,
        links,
        forms,
        interactive,
    })
}

// ==================== Action 实现 ====================

/// action=search：Brave Search API 搜索
fn exec_search(args: &Value) -> ToolResult {
    let query = match args.get("query").and_then(|q| q.as_str()) {
        Some(q) => q,
        None => {
            return ToolResult {
                output: "search 操作缺少 query 参数".to_string(),
                is_error: true,
            };
        }
    };

    let count = args
        .get("count")
        .and_then(|c| c.as_u64())
        .map(|c| c as usize)
        .unwrap_or(DEFAULT_SEARCH_COUNT)
        .min(MAX_SEARCH_COUNT)
        .max(1);

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

/// action=fetch：抓取网页并转为 Markdown/文本
fn exec_fetch(args: &Value, cancelled: &Arc<AtomicBool>) -> ToolResult {
    let url = match args.get("url").and_then(|u| u.as_str()) {
        Some(u) => u,
        None => {
            return ToolResult {
                output: "fetch 操作缺少 url 参数".to_string(),
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

/// action=open：打开 URL 到 Lite tab
fn exec_open(args: &Value) -> ToolResult {
    let url = match args.get("url").and_then(|u| u.as_str()) {
        Some(u) => u,
        None => {
            return ToolResult {
                output: "open 操作缺少 url 参数".to_string(),
                is_error: true,
            };
        }
    };

    let authorization = args.get("authorization").and_then(|a| a.as_str());
    let custom_headers = args.get("headers").and_then(|h| h.as_object()).map(|obj| {
        obj.iter()
            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
            .collect::<Vec<_>>()
    });

    let tab = match fetch_tab(url, authorization, custom_headers.as_ref()) {
        Ok(t) => t,
        Err(e) => {
            return ToolResult {
                output: format!("打开页面失败: {}", e),
                is_error: true,
            };
        }
    };

    let mut br = match browser().lock() {
        Ok(b) => b,
        Err(_) => {
            return ToolResult {
                output: "内部错误: 锁被占用".to_string(),
                is_error: true,
            };
        }
    };

    let id = format!("tab_{}", br.next_id);
    br.next_id += 1;
    let title = tab.title.clone();
    let interactive_count = tab.interactive.len();
    let links_count = tab.links.len();
    let forms_count = tab.forms.len();
    br.tabs.insert(id.clone(), tab);

    ToolResult {
        output: json!({
            "success": true,
            "tab_id": id,
            "url": url,
            "title": title,
            "interactive_elements": interactive_count,
            "links": links_count,
            "forms": forms_count,
            "hint": "使用 action=snapshot 查看页面交互元素详情"
        })
        .to_string(),
        is_error: false,
    }
}

/// action=tabs：列出所有已打开的 tab
fn exec_tabs() -> ToolResult {
    let br = match browser().lock() {
        Ok(b) => b,
        Err(_) => {
            return ToolResult {
                output: "内部错误: 锁被占用".to_string(),
                is_error: true,
            };
        }
    };

    let tabs: Vec<Value> = br
        .tabs
        .iter()
        .map(|(id, t)| {
            json!({
                "id": id,
                "url": t.url,
                "title": t.title,
            })
        })
        .collect();

    ToolResult {
        output: json!({ "tabs": tabs, "count": tabs.len() }).to_string(),
        is_error: false,
    }
}

/// action=snapshot：获取页面可交互元素列表
fn exec_snapshot(args: &Value) -> ToolResult {
    let tab_id = args.get("tab_id").and_then(|t| t.as_str());

    let br = match browser().lock() {
        Ok(b) => b,
        Err(_) => {
            return ToolResult {
                output: "内部错误: 锁被占用".to_string(),
                is_error: true,
            };
        }
    };

    let tab = match tab_id {
        Some(id) => match br.tabs.get(id) {
            Some(t) => t,
            None => {
                return ToolResult {
                    output: format!("未找到 tab: {}", id),
                    is_error: true,
                };
            }
        },
        None => match br.tabs.values().next() {
            Some(t) => t,
            None => {
                return ToolResult {
                    output: "没有已打开的 tab。请先使用 action=open 打开一个页面。".to_string(),
                    is_error: true,
                };
            }
        },
    };

    ToolResult {
        output: json!({
            "title": tab.title,
            "url": tab.url,
            "elements": tab.interactive,
            "links_count": tab.links.len(),
            "forms_count": tab.forms.len(),
            "text_preview": if tab.text_content.len() > 500 {
                format!("{}...", &tab.text_content[..500])
            } else {
                tab.text_content.clone()
            }
        })
        .to_string(),
        is_error: false,
    }
}

/// action=navigate：将 tab 导航到新 URL
fn exec_navigate(args: &Value) -> ToolResult {
    let url = match args.get("url").and_then(|u| u.as_str()) {
        Some(u) => u,
        None => {
            return ToolResult {
                output: "navigate 操作缺少 url 参数".to_string(),
                is_error: true,
            };
        }
    };

    let tab_id = args.get("tab_id").and_then(|t| t.as_str());

    let authorization = args.get("authorization").and_then(|a| a.as_str());
    let custom_headers = args.get("headers").and_then(|h| h.as_object()).map(|obj| {
        obj.iter()
            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
            .collect::<Vec<_>>()
    });

    let tab = match fetch_tab(url, authorization, custom_headers.as_ref()) {
        Ok(t) => t,
        Err(e) => {
            return ToolResult {
                output: format!("导航失败: {}", e),
                is_error: true,
            };
        }
    };

    let mut br = match browser().lock() {
        Ok(b) => b,
        Err(_) => {
            return ToolResult {
                output: "内部错误: 锁被占用".to_string(),
                is_error: true,
            };
        }
    };

    let id = match tab_id {
        Some(id) => {
            if !br.tabs.contains_key(id) {
                return ToolResult {
                    output: format!("未找到 tab: {}", id),
                    is_error: true,
                };
            }
            id.to_string()
        }
        None => match br.tabs.keys().next().cloned() {
            Some(id) => id,
            None => {
                return ToolResult {
                    output: "没有已打开的 tab。请先使用 action=open 打开一个页面。".to_string(),
                    is_error: true,
                };
            }
        },
    };

    let title = tab.title.clone();
    br.tabs.insert(id.clone(), tab);

    ToolResult {
        output: json!({
            "success": true,
            "tab_id": id,
            "url": url,
            "title": title
        })
        .to_string(),
        is_error: false,
    }
}

/// action=close：关闭指定 tab
fn exec_close(args: &Value) -> ToolResult {
    let tab_id = match args.get("tab_id").and_then(|t| t.as_str()) {
        Some(id) => id,
        None => {
            return ToolResult {
                output: "close 操作缺少 tab_id 参数".to_string(),
                is_error: true,
            };
        }
    };

    let mut br = match browser().lock() {
        Ok(b) => b,
        Err(_) => {
            return ToolResult {
                output: "内部错误: 锁被占用".to_string(),
                is_error: true,
            };
        }
    };

    if br.tabs.remove(tab_id).is_some() {
        ToolResult {
            output: json!({ "success": true, "closed": tab_id }).to_string(),
            is_error: false,
        }
    } else {
        ToolResult {
            output: format!("未找到 tab: {}", tab_id),
            is_error: true,
        }
    }
}

// ==================== HTML 解析辅助函数 ====================

/// 读取响应体，超过限制则截断
fn read_response_body(response: reqwest::blocking::Response) -> Result<String, String> {
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
        if let Ok(selector) = Selector::parse(selector_str) {
            if let Some(element) = document.select(&selector).next() {
                return element.html();
            }
        }
    }

    if let Ok(body_selector) = Selector::parse("body") {
        if let Some(body) = document.select(&body_selector).next() {
            return body.html();
        }
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

    if let Ok(root_selector) = Selector::parse(":root") {
        if let Some(root) = document.select(&root_selector).next() {
            extract_text(root, &mut text);
        }
    }

    text.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// 从 HTML 标签中提取内容
fn extract_tag(html: &str, tag: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let open = format!("<{}", tag);
    let close = format!("</{}>", tag);
    let start = lower.find(&open)?;
    let after = html[start..].find('>')? + start + 1;
    let end = lower[after..].find(&close)? + after;
    Some(html[after..end].trim().to_string())
}

/// 从 HTML 中去除标签
fn strip_html(html: &str) -> String {
    let mut out = String::with_capacity(html.len() / 2);
    let mut in_tag = false;
    let mut last_space = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                if !last_space {
                    out.push(' ');
                    last_space = true;
                }
            }
            _ if !in_tag => {
                if ch.is_whitespace() {
                    if !last_space {
                        out.push(' ');
                        last_space = true;
                    }
                } else {
                    out.push(ch);
                    last_space = false;
                }
            }
            _ => {}
        }
    }
    out.trim().to_string()
}

/// 提取 <a href="...">text</a> 链接
fn extract_links(html: &str) -> Vec<Value> {
    let mut links = Vec::new();
    let lower = html.to_lowercase();
    let mut search_from = 0;
    while let Some(pos) = lower[search_from..].find("<a ") {
        let abs = search_from + pos;
        let tag_end = match lower[abs..].find('>') {
            Some(e) => abs + e,
            None => break,
        };
        let close = match lower[tag_end..].find("</a>") {
            Some(c) => tag_end + c,
            None => {
                search_from = tag_end + 1;
                continue;
            }
        };
        let tag_str = &html[abs..tag_end + 1];
        let href = attr_value(tag_str, "href").unwrap_or_default();
        let text = strip_html(&html[tag_end + 1..close]);
        if !href.is_empty() {
            links.push(json!({
                "tag": "a",
                "href": href,
                "text": if text.len() > 80 { format!("{}…", &text[..80]) } else { text },
            }));
        }
        if links.len() >= 50 {
            break;
        }
        search_from = close + 4;
    }
    links
}

/// 提取 <form> 标签
fn extract_forms(html: &str) -> Vec<Value> {
    let mut forms = Vec::new();
    let lower = html.to_lowercase();
    let mut search_from = 0;
    while let Some(pos) = lower[search_from..].find("<form") {
        let abs = search_from + pos;
        let tag_end = match lower[abs..].find('>') {
            Some(e) => abs + e,
            None => break,
        };
        let tag_str = &html[abs..tag_end + 1];
        let action = attr_value(tag_str, "action").unwrap_or_default();
        let method = attr_value(tag_str, "method").unwrap_or_else(|| "GET".into());
        forms.push(json!({
            "tag": "form",
            "action": action,
            "method": method.to_uppercase(),
        }));
        if forms.len() >= 20 {
            break;
        }
        search_from = tag_end + 1;
    }
    forms
}

/// 提取可交互元素（button, input, select, textarea 以及 role="button"/role="link"）
fn extract_interactive(html: &str) -> Vec<Value> {
    let mut elements = Vec::new();
    let tags = ["button", "input", "select", "textarea"];
    let lower = html.to_lowercase();

    for tag_name in &tags {
        let open = format!("<{}", tag_name);
        let mut search_from = 0;
        while let Some(pos) = lower[search_from..].find(&open) {
            let abs = search_from + pos;
            let tag_end = match lower[abs..].find('>') {
                Some(e) => abs + e,
                None => break,
            };
            let tag_str = &html[abs..tag_end + 1];
            let mut elem = json!({
                "ref": format!("e{}", elements.len()),
                "tag": tag_name,
            });
            if let Some(t) = attr_value(tag_str, "type") {
                elem["type"] = json!(t);
            }
            if let Some(n) = attr_value(tag_str, "name") {
                elem["name"] = json!(n);
            }
            if let Some(p) = attr_value(tag_str, "placeholder") {
                elem["placeholder"] = json!(p);
            }
            if let Some(v) = attr_value(tag_str, "value") {
                elem["value"] = json!(v);
            }
            if let Some(l) = attr_value(tag_str, "aria-label") {
                elem["aria-label"] = json!(l);
            }
            // 提取 button 的文本内容
            if *tag_name == "button" {
                let close_tag = format!("</{}>", tag_name);
                if let Some(close_pos) = lower[tag_end..].find(&close_tag) {
                    let text = strip_html(&html[tag_end + 1..tag_end + close_pos]);
                    if !text.is_empty() && text.len() <= 50 {
                        elem["text"] = json!(text);
                    }
                }
            }
            elements.push(elem);
            if elements.len() >= 50 {
                break;
            }
            search_from = tag_end + 1;
        }
        if elements.len() >= 50 {
            break;
        }
    }

    // 额外提取 role="button" 和 role="link" 的元素
    for role in &["button", "link"] {
        let pattern = format!("role=\"{}\"", role);
        let mut search_from = 0;
        while let Some(pos) = lower[search_from..].find(&pattern) {
            let abs = search_from + pos;
            // 向前找到标签开始
            let tag_start = match lower[..abs].rfind('<') {
                Some(s) => s,
                None => {
                    search_from = abs + pattern.len();
                    continue;
                }
            };
            let tag_end = match lower[tag_start..].find('>') {
                Some(e) => tag_start + e,
                None => {
                    search_from = abs + pattern.len();
                    continue;
                }
            };
            let tag_str = &html[tag_start..tag_end + 1];

            // 获取标签名
            let tag_name_end = html[tag_start + 1..]
                .find(|c: char| c.is_whitespace() || c == '>')
                .unwrap_or(0)
                + tag_start
                + 1;
            let actual_tag = &html[tag_start + 1..tag_name_end].to_lowercase();

            // 跳过已在上面处理过的标签
            if matches!(
                actual_tag.as_str(),
                "button" | "input" | "select" | "textarea"
            ) {
                search_from = tag_end + 1;
                continue;
            }

            let mut elem = json!({
                "ref": format!("e{}", elements.len()),
                "tag": actual_tag,
                "role": role,
            });
            if let Some(l) = attr_value(tag_str, "aria-label") {
                elem["aria-label"] = json!(l);
            }
            if let Some(h) = attr_value(tag_str, "href") {
                elem["href"] = json!(h);
            }
            elements.push(elem);
            if elements.len() >= 50 {
                break;
            }
            search_from = tag_end + 1;
        }
        if elements.len() >= 50 {
            break;
        }
    }

    elements
}

/// 提取 HTML 属性值
fn attr_value(tag: &str, attr: &str) -> Option<String> {
    let lower = tag.to_lowercase();
    let needle = format!("{}=\"", attr);
    let pos = lower.find(&needle)?;
    let start = pos + needle.len();
    let end = lower[start..].find('"')? + start;
    Some(tag[start..end].to_string())
}
