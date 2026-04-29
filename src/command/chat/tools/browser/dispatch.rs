//! CDP / Lite 入口分发
//!
//! 根据 feature flag 将浏览器操作分发到 CDP 或 Lite 模块。

use crate::command::chat::tools::{PlanDecision, ToolResult};
use serde_json::Value;

// ── CDP 入口 ────────────────────────────────────────────────────────────

#[cfg(feature = "browser_cdp")]
pub(super) fn exec_browser_cdp(args: &Value, action: &str) -> ToolResult {
    // 读取 headless 配置：参数 > config.yaml > 默认 true
    let headless = args
        .get("headless")
        .and_then(|v| v.as_bool())
        .unwrap_or_else(read_headless_config);

    let rt = super::cdp::get_runtime();
    let result = rt.block_on(super::cdp::exec_browser_async(args, action, headless));

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

// ── Lite fallback ────────────────────────────────────────────────────────

#[cfg(not(feature = "browser_cdp"))]
pub(super) fn exec_browser_stub(args: &Value, action: &str) -> ToolResult {
    let tab_id = args.get("tab_id").and_then(|v| v.as_str());

    let result = match action {
        "status" => super::lite::status(),
        "start" => super::lite::start(),
        "stop" => super::lite::stop(),
        "tabs" => super::lite::list_tabs(),

        "open" => {
            let url = args
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or("open 操作缺少 url 参数".to_string());
            match url {
                Ok(u) => super::lite::open_tab(u),
                Err(e) => Err(e),
            }
        }

        "navigate" => {
            let url = args
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or("navigate 操作缺少 url 参数".to_string());
            match url {
                Ok(u) => super::lite::navigate(tab_id, u),
                Err(e) => Err(e),
            }
        }

        "screenshot" => {
            let output_dir = args.get("output_dir").and_then(|v| v.as_str());
            super::lite::screenshot(output_dir)
        }
        "snapshot" => super::lite::snapshot(tab_id),
        "content" | "get_content" => super::lite::get_content(tab_id),

        "close" => {
            let id = tab_id.ok_or("close 操作缺少 tab_id 参数".to_string());
            match id {
                Ok(i) => super::lite::close_tab(i),
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

// ── 配置读取 ────────────────────────────────────────────────────────────

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
