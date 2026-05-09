//! Lite 模式浏览器模块（不需要 `browser_cdp` feature）
//!
//! 基于 reqwest 的 HTTP 抓取 + HTML 解析降级方案。

use crate::constants::{
    BROWSER_LITE_HTTP_TIMEOUT_SECS, BROWSER_LITE_MAX_FORMS, BROWSER_LITE_MAX_LINKS,
    BROWSER_LITE_MAX_REDIRECTS, BROWSER_LITE_TEXT_PREVIEW_MAX_CHARS, BROWSER_SNAPSHOT_MAX_ELEMENTS,
    BROWSER_TEXT_MAX_CHARS,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

/// Lite 模式的 "tab"：HTTP 抓取 + HTML 解析
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

fn http_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(BROWSER_LITE_HTTP_TIMEOUT_SECS))
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .redirect(reqwest::redirect::Policy::limited(BROWSER_LITE_MAX_REDIRECTS))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))
}

fn fetch_tab(url: &str) -> Result<LiteTab, String> {
    let client = http_client()?;
    let resp = client
        .get(url)
        .header("Referer", url)
        .send()
        .map_err(|e| format!("请求失败: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let body = resp.text().map_err(|e| format!("读取响应失败: {}", e))?;

    let title = extract_tag(&body, "title").unwrap_or_default();
    let links = extract_links(&body);
    let forms = extract_forms(&body);
    let interactive = extract_interactive(&body);
    let text_content = crate::util::html_extract::extract_text_from_html(&body);

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

// ── Public API ──────────────────────────────────────────────────────────

pub(super) fn status() -> Result<String, String> {
    let br = browser().lock().map_err(|_| "锁被占用")?;
    Ok(json!({
        "running": true,
        "mode": "lite",
        "tabs": br.tabs.len(),
        "note": "Lite 模式（reqwest）。使用 --features browser_cdp 编译以启用完整 CDP 支持。",
    })
    .to_string())
}

/// 启动浏览器 Lite 模式
pub(super) fn start() -> Result<String, String> {
    Ok("浏览器 Lite 模式就绪（基于 reqwest）。无需外部浏览器。".to_string())
}

pub(super) fn stop() -> Result<String, String> {
    let mut br = browser().lock().map_err(|_| "锁被占用")?;
    br.tabs.clear();
    br.next_id = 0;
    Ok("Lite 浏览器状态已清空".to_string())
}

/// 列出所有浏览器标签页
pub(super) fn list_tabs() -> Result<String, String> {
    let br = browser().lock().map_err(|_| "锁被占用")?;
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
    Ok(json!({ "tabs": tabs, "count": tabs.len() }).to_string())
}

/// 打开或导航到指定 URL
pub(super) fn open_tab(url: &str) -> Result<String, String> {
    let tab = fetch_tab(url)?;
    let mut br = browser().lock().map_err(|_| "锁被占用")?;
    let id = format!("tab_{}", br.next_id);
    br.next_id += 1;
    let title = tab.title.clone();
    let interactive_count = tab.interactive.len();
    let links_count = tab.links.len();
    let forms_count = tab.forms.len();
    br.tabs.insert(id.clone(), tab);
    Ok(json!({
        "success": true,
        "tab_id": id,
        "url": url,
        "title": title,
        "interactive_elements": interactive_count,
        "links": links_count,
        "forms": forms_count,
    })
    .to_string())
}

pub(super) fn navigate(tab_id: Option<&str>, url: &str) -> Result<String, String> {
    let tab = fetch_tab(url)?;
    let mut br = browser().lock().map_err(|_| "锁被占用")?;
    let id = match tab_id {
        Some(id) => {
            if !br.tabs.contains_key(id) {
                return Err(format!("未找到标签页: {}", id));
            }
            id.to_string()
        }
        None => br.tabs.keys().next().cloned().ok_or("没有已打开的标签页")?,
    };
    let title = tab.title.clone();
    br.tabs.insert(id.clone(), tab);
    Ok(json!({
        "success": true,
        "tab_id": id,
        "url": url,
        "title": title
    })
    .to_string())
}

pub(super) fn snapshot(tab_id: Option<&str>) -> Result<String, String> {
    let br = browser().lock().map_err(|_| "锁被占用")?;
    let tab = match tab_id {
        Some(id) => br
            .tabs
            .get(id)
            .ok_or_else(|| format!("未找到标签页: {}", id))?,
        None => br.tabs.values().next().ok_or("没有已打开的标签页")?,
    };
    Ok(json!({
        "title": tab.title,
        "url": tab.url,
        "elements": tab.interactive,
        "links_count": tab.links.len(),
        "forms_count": tab.forms.len(),
        "text_preview": if tab.text_content.len() > BROWSER_LITE_TEXT_PREVIEW_MAX_CHARS {
            let mut end = BROWSER_LITE_TEXT_PREVIEW_MAX_CHARS;
            while end > 0 && !tab.text_content.is_char_boundary(end) {
                end -= 1;
            }
            format!("{}...", &tab.text_content[..end])
        } else {
            tab.text_content.clone()
        }
    })
    .to_string())
}

pub(super) fn get_content(tab_id: Option<&str>) -> Result<String, String> {
    let br = browser().lock().map_err(|_| "锁被占用")?;
    let tab = match tab_id {
        Some(id) => br
            .tabs
            .get(id)
            .ok_or_else(|| format!("未找到标签页: {}", id))?,
        None => br.tabs.values().next().ok_or("没有已打开的标签页")?,
    };
    let text = &tab.text_content;
    if text.len() > BROWSER_TEXT_MAX_CHARS {
        let mut end = BROWSER_TEXT_MAX_CHARS;
        while end > 0 && !text.is_char_boundary(end) {
            end -= 1;
        }
        Ok(format!("{}…\n\n[截断于 50KB]", &text[..end]))
    } else {
        Ok(text.clone())
    }
}

pub(super) fn screenshot(_output_dir: Option<&str>) -> Result<String, String> {
    Ok(json!({
        "note": "截图需要 'browser_cdp' feature（CDP）。使用 'snapshot' 获取页面元素列表。",
    })
    .to_string())
}

pub(super) fn close_tab(tab_id: &str) -> Result<String, String> {
    let mut br = browser().lock().map_err(|_| "锁被占用")?;
    if br.tabs.remove(tab_id).is_some() {
        Ok(json!({ "success": true, "closed": tab_id }).to_string())
    } else {
        Err(format!("未找到标签页: {}", tab_id))
    }
}

// ── HTML 解析辅助函数 ──────────────────────────────────────────────────

fn extract_tag(html: &str, tag: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let open = format!("<{}", tag);
    let close = format!("</{}>", tag);
    let start = lower.find(&open)?;
    let after = html[start..].find('>')? + start + 1;
    let end = lower[after..].find(&close)? + after;
    Some(html[after..end].trim().to_string())
}

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
                "text": if text.chars().count() > 80 {
                    let end = text.char_indices().nth(80).map(|(i, _)| i).unwrap_or(text.len());
                    format!("{}…", &text[..end])
                } else { text },
            }));
        }
        if links.len() >= BROWSER_LITE_MAX_LINKS {
            break;
        }
        search_from = close + 4;
    }
    links
}

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
        if forms.len() >= BROWSER_LITE_MAX_FORMS {
            break;
        }
        search_from = tag_end + 1;
    }
    forms
}

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
            let ref_id = format!("e{}", elements.len());
            let mut elem = json!({
                "ref": &ref_id,
                "selector": format!("[data-jref=\"{}\"]", &ref_id),
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
            if elements.len() >= BROWSER_SNAPSHOT_MAX_ELEMENTS {
                break;
            }
            search_from = tag_end + 1;
        }
        if elements.len() >= BROWSER_SNAPSHOT_MAX_ELEMENTS {
            break;
        }
    }

    // 额外提取 role="button" 和 role="link"
    for role in &["button", "link"] {
        let pattern = format!("role=\"{}\"", role);
        let mut search_from = 0;
        while let Some(pos) = lower[search_from..].find(&pattern) {
            let abs = search_from + pos;
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

            let tag_name_end = html[tag_start + 1..]
                .find(|c: char| c.is_whitespace() || c == '>')
                .unwrap_or(0)
                + tag_start
                + 1;
            let actual_tag = &html[tag_start + 1..tag_name_end].to_lowercase();

            if matches!(
                actual_tag.as_str(),
                "button" | "input" | "select" | "textarea"
            ) {
                search_from = tag_end + 1;
                continue;
            }

            let ref_id = format!("e{}", elements.len());
            let mut elem = json!({
                "ref": &ref_id,
                "selector": format!("[data-jref=\"{}\"]", &ref_id),
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
            if elements.len() >= BROWSER_SNAPSHOT_MAX_ELEMENTS {
                break;
            }
            search_from = tag_end + 1;
        }
        if elements.len() >= BROWSER_SNAPSHOT_MAX_ELEMENTS {
            break;
        }
    }

    elements
}

fn attr_value(tag: &str, attr: &str) -> Option<String> {
    let lower = tag.to_lowercase();
    let needle = format!("{}=\"", attr);
    let pos = lower.find(&needle)?;
    let start = pos + needle.len();
    let end = lower[start..].find('"')? + start;
    Some(tag[start..end].to_string())
}
