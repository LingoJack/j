//! ComputerUse action 实现
//!
//! 包含所有 action_* 方法和相关辅助方法。

use super::ComputerUseTool;
use super::helper;
use super::tool::{SOM_STALE_SECONDS, SomEntry, SomState};
use crate::constants::{
    APP_FOCUS_WAIT_MS, AX_TREE_OUTPUT_MAX_CHARS, DRAG_DEFAULT_DURATION_MS,
};
use crate::tools::{ImageData, PlanDecision, ToolResult};
use serde_json::{Value, json};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

impl ComputerUseTool {
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
            ax_role = helper::ax_role_to_applescript(role),
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

    fn click_at_coordinates(x: f64, y: f64, click_type: &str) -> Result<String, String> {
        helper::show_click_indicator(x, y, Some("Click"));

        let result = match click_type {
            "click" => super::mouse::click(x, y),
            "doubleclick" => super::mouse::double_click(x, y),
            "rightclick" => super::mouse::right_click(x, y),
            _ => super::mouse::click(x, y),
        };

        result
            .map(|_| format!("坐标点击 ({:.0}, {:.0})", x, y))
            .map_err(|e| e.to_string())
    }

    fn resolve_coordinates(&self, v: &Value) -> Result<(f64, f64), String> {
        if let Some(element) = v.get("element").and_then(|e| e.as_u64()) {
            let state = self.som_state.lock().unwrap_or_else(|e| e.into_inner());
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

    fn get_som_entry(&self, element: u64) -> Result<(SomEntry, Option<String>), String> {
        let state = self.som_state.lock().unwrap_or_else(|e| e.into_inner());
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

    fn action_screenshot(&self, v: &Value, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let use_som = v.get("som").and_then(|s| s.as_bool()).unwrap_or(true);
        let app_name = v.get("app").and_then(|a| a.as_str());

        if use_som {
            match super::som::capture_som(app_name, None) {
                Ok((b64, som_entries)) => {
                    let active_window = Self::get_active_window();

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

                    let mut state = self.som_state.lock().unwrap_or_else(|e| e.into_inner());
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
                        plan_decision: PlanDecision::None,
                    }
                }
                Err(e) => ToolResult {
                    output: format!("截屏失败: {}", e),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                },
            }
        } else {
            match super::som::capture_plain_screenshot() {
                Ok(b64) => {
                    let active_window = Self::get_active_window();
                    ToolResult {
                        output: format!("[截屏完成]\n当前活跃窗口: {}", active_window),
                        is_error: false,
                        images: vec![ImageData {
                            base64: b64,
                            media_type: "image/png".to_string(),
                        }],
                        plan_decision: PlanDecision::None,
                    }
                }
                Err(e) => ToolResult {
                    output: format!("截屏失败: {}", e),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
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
        if let Some(element) = v.get("element").and_then(|e| e.as_u64()) {
            match self.get_som_entry(element) {
                Ok((entry, app_name)) => {
                    if click_type == "click"
                        && let Some(ref app) = app_name
                        && let Ok(msg) = Self::ax_click_element(&entry, app)
                    {
                        helper::show_click_indicator(
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
                            plan_decision: PlanDecision::None,
                        };
                    }
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
                                plan_decision: PlanDecision::None,
                            };
                        }
                        Err(e) => {
                            return ToolResult {
                                output: format!("{} 失败: {}", click_type, e),
                                is_error: true,
                                images: vec![],
                                plan_decision: PlanDecision::None,
                            };
                        }
                    }
                }
                Err(e) => {
                    return ToolResult {
                        output: e,
                        is_error: true,
                        images: vec![],
                        plan_decision: PlanDecision::None,
                    };
                }
            }
        }

        let (x, y) = match self.resolve_coordinates(v) {
            Ok(coords) => coords,
            Err(e) => {
                return ToolResult {
                    output: e,
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
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
                    plan_decision: PlanDecision::None,
                }
            }
            Err(e) => ToolResult {
                output: format!("{} 失败: {}", click_type, e),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
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
                    plan_decision: PlanDecision::None,
                };
            }
        };

        let delay_ms = v.get("delay_ms").and_then(|d| d.as_u64()).unwrap_or(10);

        match super::keyboard::type_text(text, delay_ms) {
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
                    plan_decision: PlanDecision::None,
                }
            }
            Err(e) => ToolResult {
                output: format!("输入失败: {}", e),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
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
                    plan_decision: PlanDecision::None,
                };
            }
        };

        match super::keyboard::press_key(key) {
            Ok(_) => {
                let active_window = Self::get_active_window();
                ToolResult {
                    output: format!("[按键完成] key: {}\n当前活跃窗口: {}", key, active_window),
                    is_error: false,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                }
            }
            Err(e) => ToolResult {
                output: format!("按键失败: {}", e),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
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
                    plan_decision: PlanDecision::None,
                };
            }
        };

        if keys.len() < 2 {
            return ToolResult {
                output: "key_combo 至少需要 2 个键".to_string(),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            };
        }

        match super::keyboard::key_combo(&keys) {
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
                    plan_decision: PlanDecision::None,
                }
            }
            Err(e) => ToolResult {
                output: format!("组合键失败: {}", e),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            },
        }
    }

    fn action_scroll(&self, v: &Value, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let dx = v.get("dx").and_then(|d| d.as_i64()).unwrap_or(0) as i32;
        let dy = v.get("dy").and_then(|d| d.as_i64()).unwrap_or(0) as i32;
        let at = self.resolve_coordinates(v).ok();

        if let Some((x, y)) = at {
            helper::show_click_indicator(x, y, Some("Scroll"));
        }

        match super::mouse::scroll(dx, dy, at) {
            Ok(_) => {
                let active_window = Self::get_active_window();
                ToolResult {
                    output: format!(
                        "[滚动完成] dx: {}, dy: {}\n当前活跃窗口: {}",
                        dx, dy, active_window
                    ),
                    is_error: false,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                }
            }
            Err(e) => ToolResult {
                output: format!("滚动失败: {}", e),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            },
        }
    }

    fn action_drag(&self, v: &Value, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let (sx, sy) = if let Some(se) = v.get("start_element") {
            let sv = json!({"element": se});
            match self.resolve_coordinates(&sv) {
                Ok(c) => c,
                Err(e) => {
                    return ToolResult {
                        output: format!("起点解析失败: {}", e),
                        is_error: true,
                        images: vec![],
                        plan_decision: PlanDecision::None,
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
                        plan_decision: PlanDecision::None,
                    };
                }
            }
        };

        let (ex, ey) = if let Some(ee) = v.get("end_element") {
            let ev = json!({"element": ee});
            match self.resolve_coordinates(&ev) {
                Ok(c) => c,
                Err(e) => {
                    return ToolResult {
                        output: format!("终点解析失败: {}", e),
                        is_error: true,
                        images: vec![],
                        plan_decision: PlanDecision::None,
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
                        plan_decision: PlanDecision::None,
                    };
                }
            }
        };

        helper::show_click_indicator(sx, sy, Some("Drag\u{2197}"));

        let duration_ms = v
            .get("duration_ms")
            .and_then(|d| d.as_u64())
            .unwrap_or(DRAG_DEFAULT_DURATION_MS);

        match super::mouse::drag(sx, sy, ex, ey, duration_ms) {
            Ok(_) => {
                let active_window = Self::get_active_window();
                ToolResult {
                    output: format!(
                        "[拖拽完成] ({:.0},{:.0}) → ({:.0},{:.0})\n当前活跃窗口: {}",
                        sx, sy, ex, ey, active_window
                    ),
                    is_error: false,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                }
            }
            Err(e) => ToolResult {
                output: format!("拖拽失败: {}", e),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
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

        match super::ax::query_tree(app, depth, clickable) {
            Ok(tree) => {
                let active_window = Self::get_active_window();
                let mut output = format!("[无障碍树]\n当前活跃窗口: {}\n\n", active_window);

                let json_str = serde_json::to_string_pretty(&tree).unwrap_or_default();
                if json_str.len() > AX_TREE_OUTPUT_MAX_CHARS {
                    output.push_str(&json_str[..AX_TREE_OUTPUT_MAX_CHARS]);
                    output
                        .push_str("\n...(输出过长已截断，请使用 depth 或 clickable 参数缩小范围)");
                } else {
                    output.push_str(&json_str);
                }
                ToolResult {
                    output,
                    is_error: false,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                }
            }
            Err(e) => ToolResult {
                output: format!("获取无障碍树失败: {}", e),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
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
                    plan_decision: PlanDecision::None,
                };
            }
        };

        let app = v.get("app").and_then(|a| a.as_str());
        let role = v.get("role").and_then(|r| r.as_str());

        match super::ax::find_elements(query, app, role) {
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
                    plan_decision: PlanDecision::None,
                }
            }
            Err(e) => ToolResult {
                output: format!("搜索元素失败: {}", e),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
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
                    plan_decision: PlanDecision::None,
                };
            }
        };

        let result = Command::new("open").args(["-a", app]).output();

        match result {
            Ok(output) => {
                if output.status.success() {
                    std::thread::sleep(std::time::Duration::from_millis(APP_FOCUS_WAIT_MS));
                    let active_window = Self::get_active_window();
                    ToolResult {
                        output: format!(
                            "[聚焦应用完成] 请求: {}\n当前活跃窗口: {}",
                            app, active_window
                        ),
                        is_error: false,
                        images: vec![],
                        plan_decision: PlanDecision::None,
                    }
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    ToolResult {
                        output: format!("聚焦应用失败: {}", stderr.trim()),
                        is_error: true,
                        images: vec![],
                        plan_decision: PlanDecision::None,
                    }
                }
            }
            Err(e) => ToolResult {
                output: format!("聚焦应用失败: {}", e),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            },
        }
    }

    fn action_cursor_position(&self, _v: &Value, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        match helper::get_cursor_position() {
            Some((x, y)) => {
                let active_window = Self::get_active_window();
                ToolResult {
                    output: format!(
                        "[光标位置] ({:.0}, {:.0})\n当前活跃窗口: {}",
                        x, y, active_window
                    ),
                    is_error: false,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                }
            }
            None => ToolResult {
                output: "获取光标位置失败".to_string(),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            },
        }
    }

    pub(super) fn execute_single_action(
        &self,
        v: &Value,
        cancelled: &Arc<AtomicBool>,
    ) -> ToolResult {
        let action = match v.get("action").and_then(|a| a.as_str()) {
            Some(a) => a,
            None => {
                return ToolResult {
                    output: "缺少 action 参数".to_string(),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
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
                plan_decision: PlanDecision::None,
            },
        }
    }
}
