//! 浏览器自动化工具（CDP + Lite fallback）
//!
//! 启用 `browser_cdp` feature 时，使用 chromiumoxide 进行真实 CDP 浏览器控制。
//! 未启用时，退化为基于 reqwest 的 Lite 模式（HTTP 抓取 + HTML 解析）。

use crate::command::chat::tools::{    PlanDecision,Tool, ToolResult, schema_to_tool_params};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::sync::{Arc, atomic::AtomicBool};

// ==================== CDP 模块 ====================

#[cfg(feature = "browser_cdp")]
mod cdp {
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

    pub fn get_runtime() -> &'static tokio::runtime::Runtime {
        BROWSER_RUNTIME
            .get_or_init(|| tokio::runtime::Runtime::new().expect("创建浏览器 Runtime 失败"))
    }

    fn browser_state() -> &'static Mutex<Option<BrowserState>> {
        BROWSER.get_or_init(|| Mutex::new(None))
    }

    /// 启动浏览器（如果尚未运行）
    pub async fn ensure_browser(headless: bool) -> Result<(), String> {
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
    pub async fn status() -> Result<String, String> {
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
    pub async fn start(headless: bool) -> Result<String, String> {
        ensure_browser(headless).await?;
        Ok("浏览器已启动".to_string())
    }

    /// 停止浏览器
    pub async fn stop() -> Result<String, String> {
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
    pub async fn list_tabs() -> Result<String, String> {
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
    pub async fn open_tab(url: &str, headless: bool) -> Result<String, String> {
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
    pub async fn navigate(tab_id: Option<&str>, url: &str) -> Result<String, String> {
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
    pub async fn screenshot(
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
    pub async fn get_content(tab_id: Option<&str>) -> Result<String, String> {
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

        if text.len() > 50_000 {
            let mut end = 50_000;
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
    pub async fn click(tab_id: Option<&str>, selector: &str) -> Result<String, String> {
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
    pub async fn type_text(
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
    pub async fn press_key(tab_id: Option<&str>, key: &str) -> Result<String, String> {
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
    pub async fn evaluate(tab_id: Option<&str>, script: &str) -> Result<String, String> {
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
    pub async fn close_tab(tab_id: &str) -> Result<String, String> {
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
    pub async fn snapshot(tab_id: Option<&str>) -> Result<String, String> {
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
                r#"
            Array.from(document.querySelectorAll('a, button, input, select, textarea, [role="button"], [role="link"]'))
                .slice(0, 50)
                .map((el, i) => {
                    const ref = 'e' + i;
                    el.setAttribute('data-jref', ref);
                    return {
                        ref,
                        selector: '[data-jref="' + ref + '"]',
                        tag: el.tagName.toLowerCase(),
                        role: el.getAttribute('role') || el.tagName.toLowerCase(),
                        text: el.textContent?.trim().slice(0, 50) || el.getAttribute('aria-label') || el.getAttribute('placeholder') || '',
                        type: el.type || null,
                        href: el.href || null
                    };
                })
            "#,
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
    pub async fn exec_browser_async(
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
}

// ==================== Lite 模块 ====================

#[cfg(not(feature = "browser_cdp"))]
mod lite {
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
            .timeout(Duration::from_secs(15))
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .redirect(reqwest::redirect::Policy::limited(10))
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

    pub fn status() -> Result<String, String> {
        let br = browser().lock().map_err(|_| "锁被占用")?;
        Ok(json!({
            "running": true,
            "mode": "lite",
            "tabs": br.tabs.len(),
            "note": "Lite 模式（reqwest）。使用 --features browser_cdp 编译以启用完整 CDP 支持。",
        })
        .to_string())
    }

    pub fn start() -> Result<String, String> {
        Ok("浏览器 Lite 模式就绪（基于 reqwest）。无需外部浏览器。".to_string())
    }

    pub fn stop() -> Result<String, String> {
        let mut br = browser().lock().map_err(|_| "锁被占用")?;
        br.tabs.clear();
        br.next_id = 0;
        Ok("Lite 浏览器状态已清空".to_string())
    }

    pub fn list_tabs() -> Result<String, String> {
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

    pub fn open_tab(url: &str) -> Result<String, String> {
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

    pub fn navigate(tab_id: Option<&str>, url: &str) -> Result<String, String> {
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

    pub fn snapshot(tab_id: Option<&str>) -> Result<String, String> {
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
            "text_preview": if tab.text_content.len() > 500 {
                let mut end = 500;
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

    pub fn get_content(tab_id: Option<&str>) -> Result<String, String> {
        let br = browser().lock().map_err(|_| "锁被占用")?;
        let tab = match tab_id {
            Some(id) => br
                .tabs
                .get(id)
                .ok_or_else(|| format!("未找到标签页: {}", id))?,
            None => br.tabs.values().next().ok_or("没有已打开的标签页")?,
        };
        let text = &tab.text_content;
        if text.len() > 50_000 {
            let mut end = 50_000;
            while end > 0 && !text.is_char_boundary(end) {
                end -= 1;
            }
            Ok(format!("{}…\n\n[截断于 50KB]", &text[..end]))
        } else {
            Ok(text.clone())
        }
    }

    pub fn screenshot(_output_dir: Option<&str>) -> Result<String, String> {
        Ok(json!({
            "note": "截图需要 'browser_cdp' feature（CDP）。使用 'snapshot' 获取页面元素列表。",
        })
        .to_string())
    }

    pub fn close_tab(tab_id: &str) -> Result<String, String> {
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
            if links.len() >= 50 {
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
            if forms.len() >= 20 {
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
                if elements.len() >= 50 {
                    break;
                }
                search_from = tag_end + 1;
            }
            if elements.len() >= 50 {
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

    fn attr_value(tag: &str, attr: &str) -> Option<String> {
        let lower = tag.to_lowercase();
        let needle = format!("{}=\"", attr);
        let pos = lower.find(&needle)?;
        let start = pos + needle.len();
        let end = lower[start..].find('"')? + start;
        Some(tag[start..end].to_string())
    }
}

// ==================== BrowserTool ====================

/// BrowserTool 参数
#[derive(Deserialize, JsonSchema)]
#[allow(dead_code)]
struct BrowserParams {
    /// Action type. status=check status; start=launch browser; stop=stop browser; tabs=list tabs; open=open new tab(requires url); navigate=navigate existing tab(requires url); screenshot=capture screenshot(requires output_dir); snapshot=get interactive element snapshot; content=extract page text; close=close tab(requires tab_id); click=click element(requires selector); type=type text(requires selector+text); press=key press(requires key); evaluate=execute JS(requires script)
    action: String,
    /// [open/navigate] Target URL, must include full protocol (e.g. https://example.com)
    #[serde(default)]
    url: Option<String>,
    /// Target tab ID (defaults to the first tab if not specified)
    #[serde(default)]
    tab_id: Option<String>,
    /// [click/type] CSS selector to locate a page element. Strongly recommended to use the selector field returned by snapshot (e.g. '[data-jref="e3"]') for precise matching
    #[serde(default)]
    selector: Option<String>,
    /// [type] Text to input into the target element, supports Unicode characters
    #[serde(default)]
    text: Option<String>,
    /// [press] Key name to press, e.g. 'Enter', 'Tab', 'Escape', 'ArrowDown', 'Backspace', or a single character like 'a'
    #[serde(default)]
    key: Option<String>,
    /// [evaluate] JavaScript code to execute in the page context
    #[serde(default)]
    script: Option<String>,
    /// [screenshot] Absolute path to the screenshot output directory
    #[serde(default)]
    output_dir: Option<String>,
    /// [screenshot] Whether to capture the full page (including parts requiring scrolling). false captures only the current viewport
    #[serde(default)]
    full_page: Option<bool>,
    /// Whether to run the browser in headless mode. true=no browser window, false=show window. Overrides the config.yaml setting
    #[serde(default)]
    headless: Option<bool>,
}

pub struct BrowserTool;

impl Tool for BrowserTool {
    fn name(&self) -> &str {
        "Browser"
    }

    fn description(&self) -> &str {
        "Browser automation tool for web browsing, interaction, and content extraction. Available actions:\n\
         - status: Check browser running status and number of open tabs\n\
         - start: Launch a browser instance (use headless param to control window visibility)\n\
         - stop: Stop the browser and close all tabs\n\
         - tabs: List all open tabs with their IDs and URLs\n\
         - open: Open a new tab and navigate to the specified URL (requires url), returns tab_id\n\
         - navigate: Navigate an existing tab to a new URL (requires url, optional tab_id)\n\
         - screenshot: Capture a page screenshot as PNG (requires output_dir, optional full_page)\n\
         - snapshot: Get a page snapshot with title, URL, and interactive element list (buttons, inputs, links, etc.) for understanding page structure\n\
         - content: Extract page body text (intelligently removes navbars, scripts, and noise)\n\
         - close: Close a specific tab (requires tab_id)\n\
         - click: Click a page element (requires selector, CSS selector)\n\
         - type: Type text into an input field (requires selector and text, supports Unicode)\n\
         - press: Simulate a key press (requires key, e.g. Enter, Tab, Escape)\n\
         - evaluate: Execute JavaScript in the page context (requires script)\n\
         Typical flow: open a page → use snapshot to discover elements → use the selector field from snapshot (e.g. [data-jref=\"e3\"]) with click/type/press to interact → use content to get results.\
         Note: snapshot injects a data-jref attribute on each element and returns the corresponding selector; always use that selector for click/type instead of constructing your own."
    }

    fn parameters_schema(&self) -> Value {
        schema_to_tool_params::<BrowserParams>()
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let params: BrowserParams = match serde_json::from_str(arguments) {
            Ok(p) => p,
            Err(e) => {
                return ToolResult {
                    output: format!("参数解析失败: {}", e),
                    is_error: true,
                    images: vec![],
                plan_decision: PlanDecision::None,
                };
            }
        };

        // 也解析为 Value 以便传给 cdp/lite dispatch（它们使用 .get()）
        let args: Value = serde_json::from_str(arguments).unwrap_or_default();

        #[cfg(feature = "browser_cdp")]
        {
            exec_browser_cdp(&args, &params.action)
        }

        #[cfg(not(feature = "browser_cdp"))]
        {
            exec_browser_stub(&args, &params.action)
        }
    }

    fn requires_confirmation(&self) -> bool {
        false
    }
}

// ==================== CDP 入口 ====================

#[cfg(feature = "browser_cdp")]
fn exec_browser_cdp(args: &Value, action: &str) -> ToolResult {
    // 读取 headless 配置：参数 > config.yaml > 默认 true
    let headless = args
        .get("headless")
        .and_then(|v| v.as_bool())
        .unwrap_or_else(read_headless_config);

    let rt = cdp::get_runtime();
    let result = rt.block_on(cdp::exec_browser_async(args, action, headless));

    match result {
        Ok(output) => ToolResult {
            output,
            is_error: false,
            images: vec![],
                plan_decision: PlanDecision::None,
        },
        Err(err) => ToolResult {
            output: err,
            is_error: true,
            images: vec![],
                plan_decision: PlanDecision::None,
        },
    }
}

// ==================== Lite fallback ====================

#[cfg(not(feature = "browser_cdp"))]
fn exec_browser_stub(args: &Value, action: &str) -> ToolResult {
    let tab_id = args.get("tab_id").and_then(|v| v.as_str());

    let result = match action {
        "status" => lite::status(),
        "start" => lite::start(),
        "stop" => lite::stop(),
        "tabs" => lite::list_tabs(),

        "open" => {
            let url = args
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or("open 操作缺少 url 参数".to_string());
            match url {
                Ok(u) => lite::open_tab(u),
                Err(e) => Err(e),
            }
        }

        "navigate" => {
            let url = args
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or("navigate 操作缺少 url 参数".to_string());
            match url {
                Ok(u) => lite::navigate(tab_id, u),
                Err(e) => Err(e),
            }
        }

        "screenshot" => {
            let output_dir = args.get("output_dir").and_then(|v| v.as_str());
            lite::screenshot(output_dir)
        }
        "snapshot" => lite::snapshot(tab_id),
        "content" | "get_content" => lite::get_content(tab_id),

        "close" => {
            let id = tab_id.ok_or("close 操作缺少 tab_id 参数".to_string());
            match id {
                Ok(i) => lite::close_tab(i),
                Err(e) => Err(e),
            }
        }

        "click" | "type" | "press" => Ok(serde_json::json!({
            "note": format!("操作 '{}' 需要 'browser_cdp' feature（CDP）。使用 'snapshot' 查看页面交互元素。", action),
        })
        .to_string()),

        "evaluate" => Ok(serde_json::json!({
            "note": "JavaScript 执行需要 'browser_cdp' feature（CDP）。使用 'content' 获取页面文本。",
        })
        .to_string()),

        _ => Err(format!(
            "未知操作: {}。可选: status, start, stop, tabs, open, navigate, screenshot, snapshot, content, close, click, type, press, evaluate",
            action
        )),
    };

    match result {
        Ok(output) => ToolResult {
            output,
            is_error: false,
            images: vec![],
                plan_decision: PlanDecision::None,
        },
        Err(err) => ToolResult {
            output: err,
            is_error: true,
            images: vec![],
                plan_decision: PlanDecision::None,
        },
    }
}

// ==================== 配置读取 ====================

#[cfg(feature = "browser_cdp")]
fn read_headless_config() -> bool {
    // TODO(human): 当前每次调用都从磁盘读取 config.yaml，
    // 可考虑用 OnceLock 缓存 headless 值以避免重复 I/O，
    // 但缓存意味着修改 config 后需重启才能生效。
    // 请决定保留当前实现或改为缓存方案。
    use crate::config::yaml_config::YamlConfig;
    use crate::constants::{config_key, section};

    let config = YamlConfig::load();
    config
        .get_property(section::SETTING, config_key::BROWSER_HEADLESS)
        .map(|v| v != "false")
        .unwrap_or(true) // 默认 headless
}
