mod ax;
mod error;
mod keyboard;
mod keymap;
mod mouse;
mod preview;
mod screenshot;
mod som;

use crate::command::chat::tools::{ImageData, Tool, ToolResult, schema_to_tool_params};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// ComputerUseTool 参数
#[derive(Deserialize, JsonSchema)]
#[allow(dead_code)]
struct ComputerUseParams {
    /// Action to perform (for single action mode)
    #[serde(default)]
    action: Option<String>,
    /// Batch mode: array of action objects executed sequentially. Stops on first error.
    #[serde(default)]
    actions: Option<Vec<Value>>,
    /// X coordinate (logical points)
    #[serde(default)]
    x: Option<f64>,
    /// Y coordinate (logical points)
    #[serde(default)]
    y: Option<f64>,
    /// SoM element number from last screenshot
    #[serde(default)]
    element: Option<u64>,
    /// Text to type (for 'type' action)
    #[serde(default)]
    text: Option<String>,
    /// Key name (for 'key' action, e.g. 'enter', 'tab', 'escape')
    #[serde(default)]
    key: Option<String>,
    /// Key names for combo (e.g. ["cmd", "c"])
    #[serde(default)]
    keys: Option<Vec<String>>,
    /// Horizontal scroll amount
    #[serde(default)]
    dx: Option<i32>,
    /// Vertical scroll amount (negative=up)
    #[serde(default)]
    dy: Option<i32>,
    /// Drag start X
    #[serde(default)]
    start_x: Option<f64>,
    /// Drag start Y
    #[serde(default)]
    start_y: Option<f64>,
    /// Drag end X
    #[serde(default)]
    end_x: Option<f64>,
    /// Drag end Y
    #[serde(default)]
    end_y: Option<f64>,
    /// Drag start SoM element
    #[serde(default)]
    start_element: Option<u64>,
    /// Drag end SoM element
    #[serde(default)]
    end_element: Option<u64>,
    /// Drag duration in milliseconds
    #[serde(default)]
    duration_ms: Option<u64>,
    /// Delay between keystrokes in ms (for 'type')
    #[serde(default)]
    delay_ms: Option<u64>,
    /// Target application name
    #[serde(default)]
    app: Option<String>,
    /// Accessibility tree depth limit
    #[serde(default)]
    depth: Option<u32>,
    /// Only show clickable elements in ax_tree
    #[serde(default)]
    clickable: Option<bool>,
    /// Enable SoM annotations on screenshot (default: true)
    #[serde(default)]
    som: Option<bool>,
    /// Search query for find_element
    #[serde(default)]
    query: Option<String>,
    /// Accessibility role filter (e.g. 'AXButton')
    #[serde(default)]
    role: Option<String>,
}

// ========== SoM 状态 ==========

/// SoM 索引条目
#[derive(Debug, Clone, Deserialize)]
struct SomEntry {
    index: usize,
    role: String,
    title: Option<String>,
    center_x: f64,
    center_y: f64,
}

/// SoM 状态（最近一次截屏的元素索引）
struct SomState {
    entries: Vec<SomEntry>,
    timestamp: Instant,
    /// 截屏时的 frontmost app 名称，用于 AX click
    app_name: Option<String>,
}

/// SoM 过期时间（秒）
const SOM_STALE_SECONDS: u64 = 30;

// ========== ComputerUseTool ==========

pub struct ComputerUseTool {
    som_state: Arc<Mutex<Option<SomState>>>,
}

impl ComputerUseTool {
    pub fn new() -> Self {
        Self {
            som_state: Arc::new(Mutex::new(None)),
        }
    }

    /// 获取当前活跃窗口名称
    fn get_active_window() -> String {
        let output = Command::new("osascript")
            .args([
                "-e",
                "tell application \"System Events\" to get name of first application process whose frontmost is true",
            ])
            .output();
        match output {
            Ok(out) => String::from_utf8_lossy(&out.stdout).trim().to_string(),
            Err(_) => "unknown".to_string(),
        }
    }

    /// 通过 Accessibility API (AppleScript) 对元素执行 AXPress，不移动鼠标
    fn ax_click_element(entry: &SomEntry, app_name: &str) -> Result<String, String> {
        let role = &entry.role;
        let title = entry.title.as_deref().unwrap_or("");

        if title.is_empty() {
            return Err("元素没有 title，无法通过 AX 定位，将使用坐标点击".to_string());
        }

        let escaped_title = title.replace('\\', "\\\\").replace('"', "\\\"");
        let escaped_app = app_name.replace('\\', "\\\\").replace('"', "\\\"");

        let script = format!(
            r#"
tell application "System Events"
    tell process "{app}"
        set targetElements to (every {ax_role} whose title is "{title}" or description is "{title}" or value is "{title}")
        if (count of targetElements) > 0 then
            click item 1 of targetElements
            return "ok"
        else
            -- 递归搜索子元素
            set allUIElements to every UI element of window 1
            repeat with elem in allUIElements
                try
                    set subElements to (every {ax_role} of elem whose title is "{title}" or description is "{title}")
                    if (count of subElements) > 0 then
                        click item 1 of subElements
                        return "ok"
                    end if
                end try
            end repeat
            return "not_found"
        end if
    end tell
end tell
"#,
            app = escaped_app,
            ax_role = ax_role_to_applescript(role),
            title = escaped_title,
        );

        let output = Command::new("osascript")
            .args(["-e", &script])
            .output()
            .map_err(|e| format!("osascript 执行失败: {}", e))?;

        let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

        if result == "ok" {
            Ok(format!(
                "通过 Accessibility API 点击 {} \"{}\"",
                role, title
            ))
        } else if result == "not_found" {
            Err(format!(
                "AX 未找到匹配元素 {} \"{}\"，将使用坐标点击",
                role, title
            ))
        } else {
            Err(format!("AX 点击失败: {} {}", result, stderr))
        }
    }

    /// 通过坐标点击（直接调用内部模块）
    fn click_at_coordinates(x: f64, y: f64, click_type: &str) -> Result<String, String> {
        show_click_indicator(x, y, Some("Click"));

        let result = match click_type {
            "click" => mouse::click(x, y),
            "doubleclick" => mouse::double_click(x, y),
            "rightclick" => mouse::right_click(x, y),
            _ => mouse::click(x, y),
        };

        result
            .map(|_| format!("坐标点击 ({:.0}, {:.0})", x, y))
            .map_err(|e| e.to_string())
    }

    /// 从 SoM 状态中解析坐标
    fn resolve_coordinates(&self, v: &Value) -> Result<(f64, f64), String> {
        if let Some(element) = v.get("element").and_then(|e| e.as_u64()) {
            let state = self.som_state.lock().unwrap();
            match state.as_ref() {
                None => {
                    return Err("没有 SoM 索引，请先执行 screenshot action".to_string());
                }
                Some(som) => {
                    if som.timestamp.elapsed().as_secs() > SOM_STALE_SECONDS {
                        return Err(format!(
                            "SoM 索引已过期（{}秒前），请重新执行 screenshot action",
                            som.timestamp.elapsed().as_secs()
                        ));
                    }
                    match som.entries.iter().find(|e| e.index == element as usize) {
                        Some(entry) => return Ok((entry.center_x, entry.center_y)),
                        None => {
                            return Err(format!(
                                "元素 #{} 不存在（当前索引有 {} 个元素）",
                                element,
                                som.entries.len()
                            ));
                        }
                    }
                }
            }
        }

        let x = v
            .get("x")
            .and_then(|x| x.as_f64())
            .ok_or("缺少 x 坐标（需要 x,y 或 element 参数）")?;
        let y = v.get("y").and_then(|y| y.as_f64()).ok_or("缺少 y 坐标")?;
        Ok((x, y))
    }

    /// 从 SoM 状态获取元素信息
    fn get_som_entry(&self, element: u64) -> Result<(SomEntry, Option<String>), String> {
        let state = self.som_state.lock().unwrap();
        match state.as_ref() {
            None => Err("没有 SoM 索引，请先执行 screenshot action".to_string()),
            Some(som) => {
                if som.timestamp.elapsed().as_secs() > SOM_STALE_SECONDS {
                    return Err(format!(
                        "SoM 索引已过期（{}秒前），请重新执行 screenshot action",
                        som.timestamp.elapsed().as_secs()
                    ));
                }
                match som.entries.iter().find(|e| e.index == element as usize) {
                    Some(entry) => Ok((entry.clone(), som.app_name.clone())),
                    None => Err(format!(
                        "元素 #{} 不存在（当前索引有 {} 个元素）",
                        element,
                        som.entries.len()
                    )),
                }
            }
        }
    }

    /// 格式化 SoM 索引为文本表格
    fn format_som_index(entries: &[SomEntry]) -> String {
        let mut lines = Vec::new();
        for e in entries {
            let title = e.title.as_deref().unwrap_or("");
            let title_display = if title.chars().count() > 40 {
                let truncated: String = title.chars().take(37).collect();
                format!("{}...", truncated)
            } else {
                title.to_string()
            };
            lines.push(format!(
                "#{:<3} {} \"{}\" ({:.0}, {:.0})",
                e.index, e.role, title_display, e.center_x, e.center_y
            ));
        }
        lines.join("\n")
    }

    // ========== Action 实现 ==========

    fn action_screenshot(&self, v: &Value, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let use_som = v.get("som").and_then(|s| s.as_bool()).unwrap_or(true);

        let app_name = v.get("app").and_then(|a| a.as_str());

        if use_som {
            // SoM screenshot: capture with annotations
            match som::capture_som(app_name, None) {
                Ok((b64, som_entries)) => {
                    let active_window = Self::get_active_window();

                    // Convert som entries to our SomEntry format for state storage
                    let entries: Vec<SomEntry> = som_entries
                        .iter()
                        .map(|e| SomEntry {
                            index: e.index,
                            role: e.role.clone(),
                            title: e.title.clone(),
                            center_x: e.center_x,
                            center_y: e.center_y,
                        })
                        .collect();

                    let mut output = String::new();
                    output.push_str(&format!("[截屏完成，共 {} 个可交互元素]\n", entries.len()));
                    output.push_str(&format!("当前活跃窗口: {}\n\n", active_window));
                    output.push_str("元素索引:\n");
                    output.push_str(&Self::format_som_index(&entries));
                    output
                        .push_str("\n\n使用 click/doubleclick/rightclick 指定 element=N 来交互。");

                    // 更新 SoM 状态
                    let mut state = self.som_state.lock().unwrap();
                    *state = Some(SomState {
                        entries,
                        timestamp: Instant::now(),
                        app_name: Some(active_window),
                    });

                    ToolResult {
                        output,
                        is_error: false,
                        images: vec![ImageData {
                            base64: b64,
                            media_type: "image/png".to_string(),
                        }],
                    }
                }
                Err(e) => ToolResult {
                    output: format!("截屏失败: {}", e),
                    is_error: true,
                    images: vec![],
                },
            }
        } else {
            // Plain screenshot without SoM
            match som::capture_plain_screenshot() {
                Ok(b64) => {
                    let active_window = Self::get_active_window();
                    ToolResult {
                        output: format!("[截屏完成]\n当前活跃窗口: {}", active_window),
                        is_error: false,
                        images: vec![ImageData {
                            base64: b64,
                            media_type: "image/png".to_string(),
                        }],
                    }
                }
                Err(e) => ToolResult {
                    output: format!("截屏失败: {}", e),
                    is_error: true,
                    images: vec![],
                },
            }
        }
    }

    fn action_click(
        &self,
        v: &Value,
        _cancelled: &Arc<AtomicBool>,
        click_type: &str,
    ) -> ToolResult {
        // 如果使用 element 编号，优先尝试 Accessibility API 点击
        if let Some(element) = v.get("element").and_then(|e| e.as_u64()) {
            match self.get_som_entry(element) {
                Ok((entry, app_name)) => {
                    // 尝试 AX 点击（仅支持 click）
                    if click_type == "click"
                        && let Some(ref app) = app_name
                    {
                        match Self::ax_click_element(&entry, app) {
                            Ok(msg) => {
                                show_click_indicator(
                                    entry.center_x,
                                    entry.center_y,
                                    Some(&format!("AX #{}", element)),
                                );
                                let active_window = Self::get_active_window();
                                return ToolResult {
                                    output: format!(
                                        "[{} 完成 via AX] 元素 #{}: {} \"{}\"\n{}\n当前活跃窗口: {}",
                                        click_type,
                                        element,
                                        entry.role,
                                        entry.title.as_deref().unwrap_or(""),
                                        msg,
                                        active_window
                                    ),
                                    is_error: false,
                                    images: vec![],
                                };
                            }
                            Err(_) => {
                                // AX 失败，回退到坐标点击
                            }
                        }
                    }
                    // 使用坐标点击
                    let (x, y) = (entry.center_x, entry.center_y);
                    match Self::click_at_coordinates(x, y, click_type) {
                        Ok(_) => {
                            let active_window = Self::get_active_window();
                            return ToolResult {
                                output: format!(
                                    "[{} 完成] 元素 #{} → ({:.0}, {:.0})\n当前活跃窗口: {}",
                                    click_type, element, x, y, active_window
                                ),
                                is_error: false,
                                images: vec![],
                            };
                        }
                        Err(e) => {
                            return ToolResult {
                                output: format!("{} 失败: {}", click_type, e),
                                is_error: true,
                                images: vec![],
                            };
                        }
                    }
                }
                Err(e) => {
                    return ToolResult {
                        output: e,
                        is_error: true,
                        images: vec![],
                    };
                }
            }
        }

        // 使用 x,y 坐标点击
        let (x, y) = match self.resolve_coordinates(v) {
            Ok(coords) => coords,
            Err(e) => {
                return ToolResult {
                    output: e,
                    is_error: true,
                    images: vec![],
                };
            }
        };

        match Self::click_at_coordinates(x, y, click_type) {
            Ok(_) => {
                let active_window = Self::get_active_window();
                ToolResult {
                    output: format!(
                        "[{} 完成] 坐标: ({:.0}, {:.0})\n当前活跃窗口: {}",
                        click_type, x, y, active_window
                    ),
                    is_error: false,
                    images: vec![],
                }
            }
            Err(e) => ToolResult {
                output: format!("{} 失败: {}", click_type, e),
                is_error: true,
                images: vec![],
            },
        }
    }

    fn action_type(&self, v: &Value, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let text = match v.get("text").and_then(|t| t.as_str()) {
            Some(t) => t,
            None => {
                return ToolResult {
                    output: "缺少 text 参数".to_string(),
                    is_error: true,
                    images: vec![],
                };
            }
        };

        let delay_ms = v.get("delay_ms").and_then(|d| d.as_u64()).unwrap_or(10);

        match keyboard::type_text(text, delay_ms) {
            Ok(_) => {
                let active_window = Self::get_active_window();
                ToolResult {
                    output: format!(
                        "[输入完成] 文本长度: {} 字符\n当前活跃窗口: {}",
                        text.len(),
                        active_window
                    ),
                    is_error: false,
                    images: vec![],
                }
            }
            Err(e) => ToolResult {
                output: format!("输入失败: {}", e),
                is_error: true,
                images: vec![],
            },
        }
    }

    fn action_key(&self, v: &Value, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let key = match v.get("key").and_then(|k| k.as_str()) {
            Some(k) => k,
            None => {
                return ToolResult {
                    output: "缺少 key 参数".to_string(),
                    is_error: true,
                    images: vec![],
                };
            }
        };

        match keyboard::press_key(key) {
            Ok(_) => {
                let active_window = Self::get_active_window();
                ToolResult {
                    output: format!("[按键完成] key: {}\n当前活跃窗口: {}", key, active_window),
                    is_error: false,
                    images: vec![],
                }
            }
            Err(e) => ToolResult {
                output: format!("按键失败: {}", e),
                is_error: true,
                images: vec![],
            },
        }
    }

    fn action_key_combo(&self, v: &Value, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let keys = match v.get("keys").and_then(|k| k.as_array()) {
            Some(arr) => arr
                .iter()
                .filter_map(|k| k.as_str())
                .map(|s| s.to_string())
                .collect::<Vec<_>>(),
            None => {
                return ToolResult {
                    output: "缺少 keys 数组参数".to_string(),
                    is_error: true,
                    images: vec![],
                };
            }
        };

        if keys.len() < 2 {
            return ToolResult {
                output: "key_combo 至少需要 2 个键".to_string(),
                is_error: true,
                images: vec![],
            };
        }

        match keyboard::key_combo(&keys) {
            Ok(_) => {
                let active_window = Self::get_active_window();
                ToolResult {
                    output: format!(
                        "[组合键完成] keys: {}\n当前活跃窗口: {}",
                        keys.join("+"),
                        active_window
                    ),
                    is_error: false,
                    images: vec![],
                }
            }
            Err(e) => ToolResult {
                output: format!("组合键失败: {}", e),
                is_error: true,
                images: vec![],
            },
        }
    }

    fn action_scroll(&self, v: &Value, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let dx = v.get("dx").and_then(|d| d.as_i64()).unwrap_or(0) as i32;
        let dy = v.get("dy").and_then(|d| d.as_i64()).unwrap_or(0) as i32;

        // 可选位置
        let at = self.resolve_coordinates(v).ok();

        if let Some((x, y)) = at {
            show_click_indicator(x, y, Some("Scroll"));
        }

        match mouse::scroll(dx, dy, at) {
            Ok(_) => {
                let active_window = Self::get_active_window();
                ToolResult {
                    output: format!(
                        "[滚动完成] dx: {}, dy: {}\n当前活跃窗口: {}",
                        dx, dy, active_window
                    ),
                    is_error: false,
                    images: vec![],
                }
            }
            Err(e) => ToolResult {
                output: format!("滚动失败: {}", e),
                is_error: true,
                images: vec![],
            },
        }
    }

    fn action_drag(&self, v: &Value, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        // 解析起点
        let (sx, sy) = if let Some(se) = v.get("start_element") {
            let sv = json!({"element": se});
            match self.resolve_coordinates(&sv) {
                Ok(c) => c,
                Err(e) => {
                    return ToolResult {
                        output: format!("起点解析失败: {}", e),
                        is_error: true,
                        images: vec![],
                    };
                }
            }
        } else {
            let sx = v.get("start_x").and_then(|x| x.as_f64());
            let sy = v.get("start_y").and_then(|y| y.as_f64());
            match (sx, sy) {
                (Some(x), Some(y)) => (x, y),
                _ => {
                    return ToolResult {
                        output: "缺少 start_x/start_y 或 start_element 参数".to_string(),
                        is_error: true,
                        images: vec![],
                    };
                }
            }
        };

        // 解析终点
        let (ex, ey) = if let Some(ee) = v.get("end_element") {
            let ev = json!({"element": ee});
            match self.resolve_coordinates(&ev) {
                Ok(c) => c,
                Err(e) => {
                    return ToolResult {
                        output: format!("终点解析失败: {}", e),
                        is_error: true,
                        images: vec![],
                    };
                }
            }
        } else {
            let ex = v.get("end_x").and_then(|x| x.as_f64());
            let ey = v.get("end_y").and_then(|y| y.as_f64());
            match (ex, ey) {
                (Some(x), Some(y)) => (x, y),
                _ => {
                    return ToolResult {
                        output: "缺少 end_x/end_y 或 end_element 参数".to_string(),
                        is_error: true,
                        images: vec![],
                    };
                }
            }
        };

        show_click_indicator(sx, sy, Some("Drag\u{2197}"));

        let duration_ms = v.get("duration_ms").and_then(|d| d.as_u64()).unwrap_or(500);

        match mouse::drag(sx, sy, ex, ey, duration_ms) {
            Ok(_) => {
                let active_window = Self::get_active_window();
                ToolResult {
                    output: format!(
                        "[拖拽完成] ({:.0},{:.0}) → ({:.0},{:.0})\n当前活跃窗口: {}",
                        sx, sy, ex, ey, active_window
                    ),
                    is_error: false,
                    images: vec![],
                }
            }
            Err(e) => ToolResult {
                output: format!("拖拽失败: {}", e),
                is_error: true,
                images: vec![],
            },
        }
    }

    fn action_ax_tree(&self, v: &Value, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let app = v.get("app").and_then(|a| a.as_str());
        let depth = v.get("depth").and_then(|d| d.as_u64()).map(|d| d as u32);
        let clickable = v
            .get("clickable")
            .and_then(|c| c.as_bool())
            .unwrap_or(false);

        match ax::query_tree(app, depth, clickable) {
            Ok(tree) => {
                let active_window = Self::get_active_window();
                let mut output = format!("[无障碍树]\n当前活跃窗口: {}\n\n", active_window);

                let json_str = serde_json::to_string_pretty(&tree).unwrap_or_default();
                if json_str.len() > 20000 {
                    output.push_str(&json_str[..20000]);
                    output
                        .push_str("\n...(输出过长已截断，请使用 depth 或 clickable 参数缩小范围)");
                } else {
                    output.push_str(&json_str);
                }
                ToolResult {
                    output,
                    is_error: false,
                    images: vec![],
                }
            }
            Err(e) => ToolResult {
                output: format!("获取无障碍树失败: {}", e),
                is_error: true,
                images: vec![],
            },
        }
    }

    fn action_find_element(&self, v: &Value, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let query = match v.get("query").and_then(|q| q.as_str()) {
            Some(q) => q,
            None => {
                return ToolResult {
                    output: "缺少 query 参数".to_string(),
                    is_error: true,
                    images: vec![],
                };
            }
        };

        let app = v.get("app").and_then(|a| a.as_str());
        let role = v.get("role").and_then(|r| r.as_str());

        match ax::find_elements(query, app, role) {
            Ok(results) => {
                let active_window = Self::get_active_window();
                let json_str = serde_json::to_string_pretty(&results).unwrap_or_default();
                ToolResult {
                    output: format!(
                        "[元素搜索: \"{}\"]\n当前活跃窗口: {}\n\n{}",
                        query, active_window, json_str
                    ),
                    is_error: false,
                    images: vec![],
                }
            }
            Err(e) => ToolResult {
                output: format!("搜索元素失败: {}", e),
                is_error: true,
                images: vec![],
            },
        }
    }

    fn action_focus_app(&self, v: &Value, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let app = match v.get("app").and_then(|a| a.as_str()) {
            Some(a) => a,
            None => {
                return ToolResult {
                    output: "缺少 app 参数".to_string(),
                    is_error: true,
                    images: vec![],
                };
            }
        };

        let result = Command::new("open").args(["-a", app]).output();

        match result {
            Ok(output) => {
                if output.status.success() {
                    std::thread::sleep(std::time::Duration::from_millis(300));
                    let active_window = Self::get_active_window();
                    ToolResult {
                        output: format!(
                            "[聚焦应用完成] 请求: {}\n当前活跃窗口: {}",
                            app, active_window
                        ),
                        is_error: false,
                        images: vec![],
                    }
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    ToolResult {
                        output: format!("聚焦应用失败: {}", stderr.trim()),
                        is_error: true,
                        images: vec![],
                    }
                }
            }
            Err(e) => ToolResult {
                output: format!("聚焦应用失败: {}", e),
                is_error: true,
                images: vec![],
            },
        }
    }

    fn action_cursor_position(&self, _v: &Value, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        match get_cursor_position() {
            Some((x, y)) => {
                let active_window = Self::get_active_window();
                ToolResult {
                    output: format!(
                        "[光标位置] ({:.0}, {:.0})\n当前活跃窗口: {}",
                        x, y, active_window
                    ),
                    is_error: false,
                    images: vec![],
                }
            }
            None => ToolResult {
                output: "获取光标位置失败".to_string(),
                is_error: true,
                images: vec![],
            },
        }
    }

    /// 执行单个 action
    fn execute_single_action(&self, v: &Value, cancelled: &Arc<AtomicBool>) -> ToolResult {
        let action = match v.get("action").and_then(|a| a.as_str()) {
            Some(a) => a,
            None => {
                return ToolResult {
                    output: "缺少 action 参数".to_string(),
                    is_error: true,
                    images: vec![],
                };
            }
        };

        match action {
            "screenshot" => self.action_screenshot(v, cancelled),
            "click" => self.action_click(v, cancelled, "click"),
            "doubleclick" => self.action_click(v, cancelled, "doubleclick"),
            "rightclick" => self.action_click(v, cancelled, "rightclick"),
            "type" => self.action_type(v, cancelled),
            "key" => self.action_key(v, cancelled),
            "key_combo" => self.action_key_combo(v, cancelled),
            "scroll" => self.action_scroll(v, cancelled),
            "drag" => self.action_drag(v, cancelled),
            "ax_tree" => self.action_ax_tree(v, cancelled),
            "find_element" => self.action_find_element(v, cancelled),
            "focus_app" => self.action_focus_app(v, cancelled),
            "cursor_position" => self.action_cursor_position(v, cancelled),
            _ => ToolResult {
                output: format!("未知 action: {}", action),
                is_error: true,
                images: vec![],
            },
        }
    }
}

// ========== 辅助函数 ==========

/// 触发 j-indicator 显示点击光圈（fire-and-forget）
fn show_click_indicator(x: f64, y: f64, label: Option<&str>) {
    let bin = which_j_indicator();
    if let Some(path) = bin {
        let mut args = vec![x.to_string(), y.to_string()];
        if let Some(lbl) = label {
            args.push(lbl.to_string());
        }
        let _ = Command::new(path)
            .args(&args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn(); // fire-and-forget
    }
}

/// 查找 j-indicator 二进制路径
fn which_j_indicator() -> Option<std::path::PathBuf> {
    // 先查 j 同目录
    if let Ok(output) = Command::new("which").arg("j").output()
        && output.status.success()
    {
        let j_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if let Some(dir) = std::path::Path::new(&j_path).parent() {
            let indicator = dir.join("j-indicator");
            if indicator.exists() {
                return Some(indicator);
            }
        }
    }
    // 再查 PATH
    if let Ok(output) = Command::new("which").arg("j-indicator").output()
        && output.status.success()
    {
        let p = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !p.is_empty() {
            return Some(std::path::PathBuf::from(p));
        }
    }
    None
}

/// 获取当前鼠标光标位置
fn get_cursor_position() -> Option<(f64, f64)> {
    let output = Command::new("osascript")
        .args([
            "-l", "JavaScript",
            "-e", "ObjC.import('CoreGraphics'); var e = $.CGEventCreate(null); var p = $.CGEventGetLocation(e); JSON.stringify({x: p.x, y: p.y})",
        ])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let v: Value = serde_json::from_str(&text).ok()?;
    let x = v.get("x")?.as_f64()?;
    let y = v.get("y")?.as_f64()?;
    Some((x, y))
}

/// 将 AX role 转换为 AppleScript UI element 类名
fn ax_role_to_applescript(role: &str) -> &str {
    match role {
        "AXButton" => "button",
        "AXTextField" => "text field",
        "AXTextArea" => "text area",
        "AXCheckBox" => "checkbox",
        "AXRadioButton" => "radio button",
        "AXPopUpButton" => "pop up button",
        "AXMenuButton" => "menu button",
        "AXSlider" => "slider",
        "AXLink" => "link",
        "AXImage" => "image",
        "AXStaticText" => "static text",
        "AXGroup" => "group",
        "AXTabGroup" => "tab group",
        "AXScrollArea" => "scroll area",
        "AXToolbar" => "toolbar",
        "AXMenuItem" => "menu item",
        "AXMenu" => "menu",
        "AXTable" => "table",
        "AXRow" => "row",
        "AXCell" => "cell",
        "AXComboBox" => "combo box",
        _ => "UI element",
    }
}

// ========== Tool trait 实现 ==========

impl Tool for ComputerUseTool {
    fn name(&self) -> &str {
        "ComputerUse"
    }

    fn description(&self) -> &str {
        "Control the macOS desktop: take screenshots, click/type/scroll, query accessibility tree, focus apps. Use screenshot with SoM (Set-of-Mark) to get numbered interactive elements, then reference by element number. Always take a screenshot first to understand the screen state. Element clicks use Accessibility API when possible (no mouse cursor movement)."
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
                };
            }
        };

        // 批量模式：actions 数组
        if let Some(actions) = v.get("actions").and_then(|a| a.as_array()) {
            let mut outputs = Vec::new();
            let mut all_images = Vec::new();
            for (i, action_v) in actions.iter().enumerate() {
                if cancelled.load(Ordering::Relaxed) {
                    outputs.push(format!("[操作 #{} 已取消]", i + 1));
                    break;
                }
                let result = self.execute_single_action(action_v, cancelled);
                let action_name = action_v
                    .get("action")
                    .and_then(|a| a.as_str())
                    .unwrap_or("?");
                outputs.push(format!(
                    "--- 操作 #{} ({}) ---\n{}",
                    i + 1,
                    action_name,
                    result.output
                ));
                all_images.extend(result.images);
                if result.is_error {
                    outputs.push(format!("[操作 #{} 出错，批量执行中止]", i + 1));
                    return ToolResult {
                        output: outputs.join("\n\n"),
                        is_error: true,
                        images: all_images,
                    };
                }
            }
            return ToolResult {
                output: outputs.join("\n\n"),
                is_error: false,
                images: all_images,
            };
        }

        // 单 action 模式
        self.execute_single_action(&v, cancelled)
    }

    fn requires_confirmation(&self) -> bool {
        true
    }

    fn confirmation_message(&self, arguments: &str) -> String {
        let v: Value = serde_json::from_str(arguments).unwrap_or_default();

        // 批量模式
        if let Some(actions) = v.get("actions").and_then(|a| a.as_array()) {
            let summaries: Vec<String> = actions
                .iter()
                .enumerate()
                .map(|(i, av)| {
                    let action = av.get("action").and_then(|a| a.as_str()).unwrap_or("?");
                    match action {
                        "click" | "doubleclick" | "rightclick" => {
                            if let Some(el) = av.get("element").and_then(|e| e.as_u64()) {
                                format!("{}(#{})", action, el)
                            } else {
                                format!(
                                    "{}({:.0},{:.0})",
                                    action,
                                    av.get("x").and_then(|x| x.as_f64()).unwrap_or(0.0),
                                    av.get("y").and_then(|y| y.as_f64()).unwrap_or(0.0),
                                )
                            }
                        }
                        "type" => {
                            let text = av.get("text").and_then(|t| t.as_str()).unwrap_or("");
                            let preview = if text.len() > 15 {
                                format!("{}...", &text[..12])
                            } else {
                                text.to_string()
                            };
                            format!("type(\"{}\")", preview)
                        }
                        "key" => {
                            let key = av.get("key").and_then(|k| k.as_str()).unwrap_or("?");
                            format!("key({})", key)
                        }
                        "key_combo" => {
                            let keys = av
                                .get("keys")
                                .and_then(|k| k.as_array())
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|k| k.as_str())
                                        .collect::<Vec<_>>()
                                        .join("+")
                                })
                                .unwrap_or_default();
                            format!("key_combo({})", keys)
                        }
                        _ => format!("{}(#{})", action, i + 1),
                    }
                })
                .collect();
            return format!(
                "ComputerUse: [{}个操作] {}",
                actions.len(),
                summaries.join(" → ")
            );
        }

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
