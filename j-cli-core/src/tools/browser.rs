//! 浏览器自动化工具（CDP + Lite fallback）
//!
//! 启用 `browser_cdp` feature 时，使用 chromiumoxide 进行真实 CDP 浏览器控制。
//! 未启用时，退化为基于 reqwest 的 Lite 模式（HTTP 抓取 + HTML 解析）。
use std::borrow::Cow;

use crate::tools::{PlanDecision, Tool, ToolResult, schema_to_tool_params};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::sync::{Arc, atomic::AtomicBool};

#[cfg(feature = "browser_cdp")]
mod cdp;
mod dispatch;
#[cfg(not(feature = "browser_cdp"))]
mod lite;

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

/// 浏览器自动化工具，支持网页浏览、交互和内容提取
#[derive(Debug)]
pub struct BrowserTool;

impl BrowserTool {
    pub const NAME: &'static str = "Browser";
}

impl Tool for BrowserTool {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn description(&self) -> Cow<'_, str> {
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
         Note: snapshot injects a data-jref attribute on each element and returns the corresponding selector; always use that selector for click/type instead of constructing your own.".into()
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
            dispatch::exec_browser_cdp(&args, &params.action)
        }

        #[cfg(not(feature = "browser_cdp"))]
        {
            dispatch::exec_browser_stub(&args, &params.action)
        }
    }

    fn requires_confirmation(&self) -> bool {
        false
    }
}
