//! CDP 浏览器控制模块（需要 `browser_cdp` feature）
//!
//! 使用 chromiumoxide 进行真实 CDP 浏览器控制。

use crate::command::chat::constants::{BROWSER_SNAPSHOT_MAX_ELEMENTS, BROWSER_TEXT_MAX_CHARS};
use chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat;
use chromiumoxide::{Browser, BrowserConfig, Page};
use futures::StreamExt;
use std::collections::HashMap;
use std::sync::OnceLock;
use tokio::sync::Mutex;

/// 全局 tokio Runtime，保持 handler 事件循环存活
static BROWSER_RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

/// 全局浏览器实例
static BROWSER: OnceLock<Mutex<Option<BrowserState>>> = OnceLock::new();

struct BrowserState {
    browser: Browser,
    pages: HashMap<String, Page>,
    #[allow(dead_code)]
    handler_handle: tokio::task::JoinHandle<()>,
}

pub(super) fn get_runtime() -> &'static tokio::runtime::Runtime {
    BROWSER_RUNTIME.get_or_init(|| {
        tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("创建浏览器 Runtime 失败: {e}"))
    })
}

fn browser_state() -> &'static Mutex<Option<BrowserState>> {
    BROWSER.get_or_init(|| Mutex::new(None))
}

/// 启动浏览器（如果尚未运行）
pub(super) async fn ensure_browser(headless: bool) -> Result<(), String> {
    let mut state = browser_state().lock().await;
    if state.is_some() {
        return Ok(());
    }

    let config = if headless {
        BrowserConfig::builder()
            .viewport(None)
            .build()
            .map_err(|e| format!("构建浏览器配置失败: {}", e))?
    } else {
        BrowserConfig::builder()
            .with_head()
            .viewport(None)
            .build()
            .map_err(|e| format!("构建浏览器配置失败: {}", e))?
    };

    let (browser, mut handler) = Browser::launch(config)
        .await
        .map_err(|e| format!("启动浏览器失败: {}", e))?;

    let handler_handle = tokio::spawn(async move {
        while let Some(_event) = handler.next().await {
            // 处理 CDP 事件（浏览器正常运行所必需）
        }
    });

    *state = Some(BrowserState {
        browser,
        pages: HashMap::new(),
        handler_handle,
    });

    Ok(())
}

/// 获取浏览器状态
pub(super) async fn status() -> Result<String, String> {
    let state = browser_state().lock().await;
    if let Some(ref s) = *state {
        let mut tab_list = Vec::new();
        for (id, page) in &s.pages {
            let url: String = page
                .evaluate("window.location.href")
                .await
                .ok()
                .and_then(|v| v.into_value().ok())
                .unwrap_or_default();
            let title: String = page
                .evaluate("document.title")
                .await
                .ok()
                .and_then(|v| v.into_value().ok())
                .unwrap_or_default();
            tab_list.push(serde_json::json!({
                "id": id,
                "url": url,
                "title": if title.is_empty() { "(无标题)".to_string() } else { title },
            }));
        }
        Ok(serde_json::json!({
            "running": true,
            "tabs_count": s.pages.len(),
            "tabs": tab_list,
        })
        .to_string())
    } else {
        Ok(serde_json::json!({
            "running": false,
            "tabs_count": 0,
            "tabs": [],
            "hint": "使用 action='start' 启动浏览器，或 action='open' 直接打开页面（会自动启动）"
        })
        .to_string())
    }
}

/// 启动浏览器
pub(super) async fn start(headless: bool) -> Result<String, String> {
    ensure_browser(headless).await?;
    Ok("浏览器已启动".to_string())
}

/// 停止浏览器
pub(super) async fn stop() -> Result<String, String> {
    let mut state = browser_state().lock().await;
    if let Some(s) = state.take() {
        for (_id, page) in s.pages {
            let _ = page.close().await;
        }
        Ok("浏览器已停止".to_string())
    } else {
        Ok("浏览器未在运行".to_string())
    }
}

/// 列出标签页
pub(super) async fn list_tabs() -> Result<String, String> {
    let state = browser_state().lock().await;
    if let Some(ref s) = *state {
        let mut tabs = Vec::new();
        for (id, page) in &s.pages {
            let url: String = page
                .evaluate("window.location.href")
                .await
                .ok()
                .and_then(|v| v.into_value().ok())
                .unwrap_or_default();
            let title: String = page
                .evaluate("document.title")
                .await
                .ok()
                .and_then(|v| v.into_value().ok())
                .unwrap_or_default();
            tabs.push(serde_json::json!({
                "id": id,
                "url": url,
                "title": if title.is_empty() { "(无标题)".to_string() } else { title },
            }));
        }
        Ok(serde_json::json!({ "tabs": tabs, "count": tabs.len() }).to_string())
    } else {
        Err(
            "浏览器未运行。请先使用 action='start' 启动浏览器，或 action='open' 直接打开页面"
                .to_string(),
        )
    }
}

/// 打开新标签页
pub(super) async fn open_tab(url: &str, headless: bool) -> Result<String, String> {
    ensure_browser(headless).await?;

    let mut state = browser_state().lock().await;
    let s = state.as_mut().ok_or("浏览器未初始化")?;

    let page = s.browser.new_page(url).await.map_err(|e| {
        format!(
            "打开页面失败: {}。请检查 URL 是否正确（需包含 https://）",
            e
        )
    })?;

    // 尝试获取页面标题
    let title: String = page
        .evaluate("document.title")
        .await
        .ok()
        .and_then(|v| v.into_value().ok())
        .unwrap_or_default();

    let tab_id = format!("tab_{}", s.pages.len());
    s.pages.insert(tab_id.clone(), page);

    Ok(serde_json::json!({
        "success": true,
        "tab_id": tab_id,
        "url": url,
        "title": if title.is_empty() { "(页面加载中)".to_string() } else { title },
        "hint": "使用 action='snapshot' 查看页面可交互元素，action='content' 获取正文文本"
    })
    .to_string())
}

/// 导航到 URL
pub(super) async fn navigate(tab_id: Option<&str>, url: &str) -> Result<String, String> {
    let mut state = browser_state().lock().await;
    let s = state
        .as_mut()
        .ok_or("浏览器未运行。请先使用 action='open' 打开页面（会自动启动浏览器）")?;

    let page = if let Some(id) = tab_id {
        s.pages.get(id).ok_or_else(|| {
            format!(
                "未找到标签页 '{}'。使用 action='tabs' 查看所有可用标签页",
                id
            )
        })?
    } else {
        s.pages
            .values()
            .next()
            .ok_or("没有已打开的标签页。请先使用 action='open' 打开一个页面")?
    };

    page.goto(url)
        .await
        .map_err(|e| format!("导航失败: {}。请检查 URL 是否正确", e))?;

    // 尝试获取导航后的页面标题
    let title: String = page
        .evaluate("document.title")
        .await
        .ok()
        .and_then(|v| v.into_value().ok())
        .unwrap_or_default();

    Ok(serde_json::json!({
        "success": true,
        "url": url,
        "title": if title.is_empty() { "(页面加载中)".to_string() } else { title }
    })
    .to_string())
}

/// 截图并保存到指定目录，返回文件完整路径
pub(super) async fn screenshot(
    tab_id: Option<&str>,
    full_page: bool,
    output_dir: &str,
) -> Result<String, String> {
    let state = browser_state().lock().await;
    let s = state
        .as_ref()
        .ok_or("浏览器未运行。请先使用 action='open' 打开页面")?;

    let page = if let Some(id) = tab_id {
        s.pages.get(id).ok_or_else(|| {
            format!(
                "未找到标签页 '{}'。使用 action='tabs' 查看所有可用标签页",
                id
            )
        })?
    } else {
        s.pages
            .values()
            .next()
            .ok_or("没有已打开的标签页。请先使用 action='open' 打开一个页面")?
    };

    let screenshot_data = if full_page {
        page.screenshot(
            chromiumoxide::page::ScreenshotParams::builder()
                .format(CaptureScreenshotFormat::Png)
                .full_page(true)
                .build(),
        )
        .await
        .map_err(|e| format!("截图失败: {}", e))?
    } else {
        page.screenshot(
            chromiumoxide::page::ScreenshotParams::builder()
                .format(CaptureScreenshotFormat::Png)
                .build(),
        )
        .await
        .map_err(|e| format!("截图失败: {}", e))?
    };

    // 确保输出目录存在
    let dir = std::path::Path::new(output_dir);
    std::fs::create_dir_all(dir).map_err(|e| format!("创建输出目录失败: {}", e))?;

    // 生成带时间戳的文件名
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("获取时间戳失败: {}", e))?
        .as_millis();
    let filename = format!("screenshot_{}.png", timestamp);
    let file_path = dir.join(&filename);

    std::fs::write(&file_path, &screenshot_data).map_err(|e| format!("保存截图失败: {}", e))?;

    let full_path = file_path
        .canonicalize()
        .unwrap_or(file_path.clone())
        .to_string_lossy()
        .to_string();

    Ok(serde_json::json!({
        "success": true,
        "format": "png",
        "path": full_path
    })
    .to_string())
}

/// 获取页面内容（智能提取正文，去除导航/脚注/脚本等噪音）
pub(super) async fn get_content(tab_id: Option<&str>) -> Result<String, String> {
    let state = browser_state().lock().await;
    let s = state
        .as_ref()
        .ok_or("浏览器未运行。请先使用 action='open' 打开页面")?;

    let page = if let Some(id) = tab_id {
        s.pages.get(id).ok_or_else(|| {
            format!(
                "未找到标签页 '{}'。使用 action='tabs' 查看所有可用标签页",
                id
            )
        })?
    } else {
        s.pages
            .values()
            .next()
            .ok_or("没有已打开的标签页。请先使用 action='open' 打开一个页面")?
    };

    let raw_html = page.content().await.map_err(|e| {
        format!(
            "获取页面内容失败: {}。页面可能已关闭或正在加载，建议用 action='tabs' 检查状态",
            e
        )
    })?;

    let text = crate::util::html_extract::extract_text_from_html(&raw_html);

    if text.len() > BROWSER_TEXT_MAX_CHARS {
        let mut end = BROWSER_TEXT_MAX_CHARS;
        while end > 0 && !text.is_char_boundary(end) {
            end -= 1;
        }
        Ok(format!(
            "{}…\n\n[内容已截断，原长度: {} 字符]",
            &text[..end],
            text.len()
        ))
    } else {
        Ok(text)
    }
}

/// 点击元素
pub(super) async fn click(tab_id: Option<&str>, selector: &str) -> Result<String, String> {
    let state = browser_state().lock().await;
    let s = state
        .as_ref()
        .ok_or("浏览器未运行。请先使用 action='open' 打开页面")?;

    let page = if let Some(id) = tab_id {
        s.pages.get(id).ok_or_else(|| {
            format!(
                "未找到标签页 '{}'。使用 action='tabs' 查看所有可用标签页",
                id
            )
        })?
    } else {
        s.pages
            .values()
            .next()
            .ok_or("没有已打开的标签页。请先使用 action='open' 打开一个页面")?
    };

    // 使用 JS click，绕过 CDP 原生 click 的可见性检查
    let escaped = selector.replace('\\', "\\\\").replace('\'', "\\'");
    let script = format!(
        r#"(() => {{
            const el = document.querySelector('{}');
            if (!el) return 'not_found';
            el.scrollIntoView({{block: 'center'}});
            el.click();
            return 'ok';
        }})()"#,
        escaped
    );
    let result: String = page
        .evaluate(script)
        .await
        .map_err(|e| format!("点击失败: {}", e))?
        .into_value()
        .unwrap_or_default();

    if result == "not_found" {
        return Err(format!(
            "未找到元素 '{}'。建议先用 action='snapshot' 查看页面元素列表，使用返回的 selector 字段",
            selector
        ));
    }

    Ok(serde_json::json!({
        "success": true,
        "action": "click",
        "selector": selector
    })
    .to_string())
}

/// 输入文本（支持中文等非 ASCII 字符）
///
/// 通过 JavaScript 直接设置元素值并触发 input/change 事件，
/// 避免 CDP KeyDown/KeyUp 不支持非拉丁字符的问题。
pub(super) async fn type_text(
    tab_id: Option<&str>,
    selector: &str,
    text: &str,
) -> Result<String, String> {
    let state = browser_state().lock().await;
    let s = state
        .as_ref()
        .ok_or("浏览器未运行。请先使用 action='open' 打开页面")?;

    let page = if let Some(id) = tab_id {
        s.pages.get(id).ok_or_else(|| {
            format!(
                "未找到标签页 '{}'。使用 action='tabs' 查看所有可用标签页",
                id
            )
        })?
    } else {
        s.pages
            .values()
            .next()
            .ok_or("没有已打开的标签页。请先使用 action='open' 打开一个页面")?
    };

    // 通过 JS 点击聚焦，绕过可见性检查
    let escaped_selector = selector.replace('\\', "\\\\").replace('\'', "\\'");
    let focus_script = format!(
        r#"(() => {{
            const el = document.querySelector('{}');
            if (!el) return 'not_found';
            el.scrollIntoView({{block: 'center'}});
            el.click();
            el.focus();
            return 'ok';
        }})()"#,
        escaped_selector
    );
    let focus_result: String = page
        .evaluate(focus_script)
        .await
        .map_err(|e| format!("聚焦失败: {}", e))?
        .into_value()
        .unwrap_or_default();

    if focus_result == "not_found" {
        return Err(format!(
            "未找到元素 '{}'。建议先用 action='snapshot' 查看页面元素列表，使用返回的 selector 字段",
            selector
        ));
    }

    // 通过 JS 设置值并触发事件，兼容所有语言字符
    let escaped_text = text.replace('\\', "\\\\").replace('\'', "\\'");
    let script = format!(
        r#"(() => {{
            const el = document.querySelector('{}');
            if (!el) return 'element_not_found';
            const nativeSetter = Object.getOwnPropertyDescriptor(
                window.HTMLTextAreaElement.prototype, 'value'
            )?.set || Object.getOwnPropertyDescriptor(
                window.HTMLInputElement.prototype, 'value'
            )?.set;
            if (nativeSetter) {{
                nativeSetter.call(el, '{}');
            }} else {{
                el.value = '{}';
            }}
            el.dispatchEvent(new Event('input', {{ bubbles: true }}));
            el.dispatchEvent(new Event('change', {{ bubbles: true }}));
            return 'ok';
        }})()"#,
        escaped_selector, escaped_text, escaped_text
    );

    let result: serde_json::Value = page
        .evaluate(script.as_str())
        .await
        .map_err(|e| format!("输入失败: {}", e))?
        .into_value()
        .map_err(|e| format!("转换结果失败: {}", e))?;

    if result.as_str() == Some("element_not_found") {
        return Err(format!(
            "JS 未找到元素 '{}'。建议先用 action='snapshot' 获取最新的元素 selector",
            selector
        ));
    }

    Ok(serde_json::json!({
        "success": true,
        "action": "type",
        "selector": selector,
        "text_length": text.len()
    })
    .to_string())
}

/// 按键
pub(super) async fn press_key(tab_id: Option<&str>, key: &str) -> Result<String, String> {
    let state = browser_state().lock().await;
    let s = state
        .as_ref()
        .ok_or("浏览器未运行。请先使用 action='open' 打开页面")?;

    let page = if let Some(id) = tab_id {
        s.pages.get(id).ok_or_else(|| {
            format!(
                "未找到标签页 '{}'。使用 action='tabs' 查看所有可用标签页",
                id
            )
        })?
    } else {
        s.pages
            .values()
            .next()
            .ok_or("没有已打开的标签页。请先使用 action='open' 打开一个页面")?
    };

    use chromiumoxide::cdp::browser_protocol::input::{
        DispatchKeyEventParams, DispatchKeyEventType,
    };

    // 功能键（Enter、Tab、Escape 等）不应设置 text 参数，
    // 只有可打印的单字符才设置 text，否则 CDP 会报 "Invalid 'text' parameter"。
    let text_value = if key.len() == 1 {
        Some(key.to_string())
    } else {
        None
    };

    let mut key_down_builder = DispatchKeyEventParams::builder()
        .key(key.to_string())
        .r#type(DispatchKeyEventType::KeyDown);
    if let Some(ref t) = text_value {
        key_down_builder = key_down_builder.text(t.clone());
    }
    let key_down = key_down_builder
        .build()
        .map_err(|e| format!("构建按键参数失败: {}", e))?;
    page.execute(key_down)
        .await
        .map_err(|e| format!("按键失败: {}", e))?;

    let mut key_up_builder = DispatchKeyEventParams::builder()
        .key(key.to_string())
        .r#type(DispatchKeyEventType::KeyUp);
    if let Some(ref t) = text_value {
        key_up_builder = key_up_builder.text(t.clone());
    }
    let key_up = key_up_builder
        .build()
        .map_err(|e| format!("构建按键参数失败: {}", e))?;
    page.execute(key_up)
        .await
        .map_err(|e| format!("按键失败: {}", e))?;

    Ok(serde_json::json!({
        "success": true,
        "action": "press",
        "key": key
    })
    .to_string())
}

/// 执行 JavaScript
pub(super) async fn evaluate(tab_id: Option<&str>, script: &str) -> Result<String, String> {
    let state = browser_state().lock().await;
    let s = state
        .as_ref()
        .ok_or("浏览器未运行。请先使用 action='open' 打开页面")?;

    let page = if let Some(id) = tab_id {
        s.pages.get(id).ok_or_else(|| {
            format!(
                "未找到标签页 '{}'。使用 action='tabs' 查看所有可用标签页",
                id
            )
        })?
    } else {
        s.pages
            .values()
            .next()
            .ok_or("没有已打开的标签页。请先使用 action='open' 打开一个页面")?
    };

    let result: serde_json::Value = page
        .evaluate(script)
        .await
        .map_err(|e| format!("执行 JS 失败: {}", e))?
        .into_value()
        .map_err(|e| format!("转换结果失败: {}", e))?;

    Ok(result.to_string())
}

/// 关闭标签页
pub(super) async fn close_tab(tab_id: &str) -> Result<String, String> {
    let mut state = browser_state().lock().await;
    let s = state.as_mut().ok_or("浏览器未运行")?;

    if let Some(page) = s.pages.remove(tab_id) {
        page.close()
            .await
            .map_err(|e| format!("关闭标签页失败: {}", e))?;
        Ok(serde_json::json!({
            "success": true,
            "closed": tab_id,
            "remaining_tabs": s.pages.len()
        })
        .to_string())
    } else {
        let available: Vec<&String> = s.pages.keys().collect();
        Err(format!(
            "未找到标签页 '{}'。当前可用标签页: {:?}",
            tab_id, available
        ))
    }
}

/// 页面快照（可交互元素列表）
pub(super) async fn snapshot(tab_id: Option<&str>) -> Result<String, String> {
    let state = browser_state().lock().await;
    let s = state
        .as_ref()
        .ok_or("浏览器未运行。请先使用 action='open' 打开页面")?;

    let page = if let Some(id) = tab_id {
        s.pages.get(id).ok_or_else(|| {
            format!(
                "未找到标签页 '{}'。使用 action='tabs' 查看所有可用标签页",
                id
            )
        })?
    } else {
        s.pages
            .values()
            .next()
            .ok_or("没有已打开的标签页。请先使用 action='open' 打开一个页面")?
    };

    let title: String = page
        .evaluate("document.title")
        .await
        .map_err(|e| format!("页面可能已关闭或无响应（获取标题失败: {}）。建议使用 action='tabs' 检查标签页状态，或重新 open 页面", e))?
        .into_value()
        .unwrap_or_default();

    let url: String = page
        .evaluate("window.location.href")
        .await
        .map_err(|e| format!("页面可能已关闭或无响应（获取 URL 失败: {}）。建议使用 action='tabs' 检查标签页状态，或重新 open 页面", e))?
        .into_value()
        .unwrap_or_default();

    let elements: serde_json::Value = page
        .evaluate(
            format!(
                r#"
            Array.from(document.querySelectorAll('a, button, input, select, textarea, [role="button"], [role="link"]'))
                .slice(0, {})
                .map((el, i) => {{
                    const ref = 'e' + i;
                    el.setAttribute('data-jref', ref);
                    return {{
                        ref,
                        selector: '[data-jref="' + ref + '"]',
                        tag: el.tagName.toLowerCase(),
                        role: el.getAttribute('role') || el.tagName.toLowerCase(),
                        text: el.textContent?.trim().slice(0, 50) || el.getAttribute('aria-label') || el.getAttribute('placeholder') || '',
                        type: el.type || null,
                        href: el.href || null
                    }};
                }})
            "#,
                BROWSER_SNAPSHOT_MAX_ELEMENTS
            )
                .as_str(),
        )
        .await
        .map_err(|e| format!("获取页面元素失败: {}。页面可能正在加载，建议稍后重试", e))?
        .into_value()
        .unwrap_or(serde_json::json!([]));

    Ok(serde_json::json!({
        "title": title,
        "url": url,
        "elements": elements
    })
    .to_string())
}

/// 异步 action 分发
pub(super) async fn exec_browser_async(
    args: &serde_json::Value,
    action: &str,
    headless: bool,
) -> Result<String, String> {
    let tab_id = args.get("tab_id").and_then(|v| v.as_str());

    match action {
        "status" => status().await,
        "start" => start(headless).await,
        "stop" => stop().await,
        "tabs" => list_tabs().await,

        "open" => {
            let url = args
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or("open 操作缺少 url 参数")?;
            open_tab(url, headless).await
        }

        "navigate" => {
            let url = args
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or("navigate 操作缺少 url 参数")?;
            navigate(tab_id, url).await
        }

        "screenshot" => {
            let full_page = args
                .get("full_page")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let output_dir = args
                .get("output_dir")
                .and_then(|v| v.as_str())
                .ok_or("screenshot 操作缺少 output_dir 参数")?;
            screenshot(tab_id, full_page, output_dir).await
        }

        "snapshot" => snapshot(tab_id).await,

        "content" | "get_content" => get_content(tab_id).await,

        "close" => {
            let id = tab_id.ok_or("close 操作缺少 tab_id 参数")?;
            close_tab(id).await
        }

        "click" => {
            let selector = args
                .get("selector")
                .and_then(|v| v.as_str())
                .ok_or("click 操作缺少 selector 参数")?;
            click(tab_id, selector).await
        }

        "type" => {
            let selector = args
                .get("selector")
                .and_then(|v| v.as_str())
                .ok_or("type 操作缺少 selector 参数")?;
            let text = args
                .get("text")
                .and_then(|v| v.as_str())
                .ok_or("type 操作缺少 text 参数")?;
            type_text(tab_id, selector, text).await
        }

        "press" => {
            let key = args
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or("press 操作缺少 key 参数")?;
            press_key(tab_id, key).await
        }

        "evaluate" => {
            let script = args
                .get("script")
                .and_then(|v| v.as_str())
                .ok_or("evaluate 操作缺少 script 参数")?;
            evaluate(tab_id, script).await
        }

        _ => Err(format!(
            "未知操作: {}。可选: status, start, stop, tabs, open, navigate, screenshot, snapshot, content, close, click, type, press, evaluate",
            action
        )),
    }
}
