use crate::tools::{PlanDecision, Tool, ToolResult, schema_to_tool_params};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::borrow::Cow;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Deserialize, JsonSchema)]
#[allow(dead_code)]
struct ComputerUseParams {
    action: String,
    #[serde(default)]
    x: Option<f64>,
    #[serde(default)]
    y: Option<f64>,
    #[serde(default)]
    element: Option<u64>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    keys: Option<Vec<String>>,
    #[serde(default)]
    dx: Option<i32>,
    #[serde(default)]
    dy: Option<i32>,
    #[serde(default)]
    start_x: Option<f64>,
    #[serde(default)]
    start_y: Option<f64>,
    #[serde(default)]
    end_x: Option<f64>,
    #[serde(default)]
    end_y: Option<f64>,
    #[serde(default)]
    start_element: Option<u64>,
    #[serde(default)]
    end_element: Option<u64>,
    #[serde(default)]
    duration_ms: Option<u64>,
    #[serde(default)]
    delay_ms: Option<u64>,
    #[serde(default)]
    app: Option<String>,
    #[serde(default)]
    depth: Option<u32>,
    #[serde(default)]
    clickable: Option<bool>,
    #[serde(default)]
    som: Option<bool>,
    #[serde(default)]
    query: Option<String>,
    #[serde(default)]
    role: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct SomEntry {
    pub(super) index: usize,
    pub(super) role: String,
    pub(super) title: Option<String>,
    pub(super) center_x: f64,
    pub(super) center_y: f64,
}

#[derive(Debug)]
pub(super) struct SomState {
    pub(super) entries: Vec<SomEntry>,
    pub(super) timestamp: Instant,
    pub(super) app_name: Option<String>,
}

pub(super) const SOM_STALE_SECONDS: u64 = 30;

/// 计算机使用工具，支持 macOS 桌面截屏、点击、输入、滚动等操作
#[derive(Debug)]
pub struct ComputerUseTool {
    pub(super) som_state: Arc<Mutex<Option<SomState>>>,
}

impl ComputerUseTool {
    pub const NAME: &'static str = "ComputerUse";

    pub fn new() -> Self {
        Self {
            som_state: Arc::new(Mutex::new(None)),
        }
    }
}

impl Tool for ComputerUseTool {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn description(&self) -> Cow<'_, str> {
        "Control the macOS desktop: take screenshots, click/type/scroll, query accessibility tree, focus apps. Use screenshot with SoM (Set-of-Mark) to get numbered interactive elements, then reference by element number. Always take a screenshot first to understand the screen state. Element clicks use Accessibility API when possible (no mouse cursor movement).".into()
    }

    fn parameters_schema(&self) -> Value {
        schema_to_tool_params::<ComputerUseParams>()
    }

    fn execute(&self, arguments: &str, cancelled: &Arc<AtomicBool>) -> ToolResult {
        let v = match serde_json::from_str::<Value>(arguments) {
            Ok(v) => v,
            Err(e) => {
                return ToolResult {
                    output: format!("参数解析失败: {}", e),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                };
            }
        };

        self.execute_single_action(&v, cancelled)
    }

    fn requires_confirmation(&self) -> bool {
        true
    }

    fn confirmation_message(&self, arguments: &str) -> String {
        let v: Value = serde_json::from_str(arguments).unwrap_or_default();

        let action = v
            .get("action")
            .and_then(|a| a.as_str())
            .unwrap_or("unknown");

        match action {
            "screenshot" => "ComputerUse: 截屏".to_string(),
            "click" | "doubleclick" | "rightclick" => {
                let coords = if let Some(el) = v.get("element").and_then(|e| e.as_u64()) {
                    format!("元素 #{}", el)
                } else {
                    let x = v.get("x").and_then(|x| x.as_f64()).unwrap_or(0.0);
                    let y = v.get("y").and_then(|y| y.as_f64()).unwrap_or(0.0);
                    format!("({:.0}, {:.0})", x, y)
                };
                format!("ComputerUse: {} {}", action, coords)
            }
            "type" => {
                let text = v.get("text").and_then(|t| t.as_str()).unwrap_or("");
                let preview = if text.chars().count() > 30 {
                    let truncated: String = text.chars().take(27).collect();
                    format!("{}...", truncated)
                } else {
                    text.to_string()
                };
                format!("ComputerUse: 输入 \"{}\"", preview)
            }
            "key" => {
                let key = v.get("key").and_then(|k| k.as_str()).unwrap_or("?");
                format!("ComputerUse: 按键 {}", key)
            }
            "key_combo" => {
                let keys = v
                    .get("keys")
                    .and_then(|k| k.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|k| k.as_str())
                            .collect::<Vec<_>>()
                            .join("+")
                    })
                    .unwrap_or_default();
                format!("ComputerUse: 组合键 {}", keys)
            }
            "scroll" => {
                let dx = v.get("dx").and_then(|d| d.as_i64()).unwrap_or(0);
                let dy = v.get("dy").and_then(|d| d.as_i64()).unwrap_or(0);
                format!("ComputerUse: 滚动 dx={} dy={}", dx, dy)
            }
            "drag" => "ComputerUse: 拖拽".to_string(),
            "ax_tree" => "ComputerUse: 获取无障碍树".to_string(),
            "find_element" => {
                let query = v.get("query").and_then(|q| q.as_str()).unwrap_or("?");
                format!("ComputerUse: 搜索元素 \"{}\"", query)
            }
            "focus_app" => {
                let app = v.get("app").and_then(|a| a.as_str()).unwrap_or("?");
                format!("ComputerUse: 聚焦 {}", app)
            }
            "cursor_position" => "ComputerUse: 获取光标位置".to_string(),
            _ => format!("ComputerUse: {}", action),
        }
    }
}
