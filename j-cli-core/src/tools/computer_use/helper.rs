//! ComputerUse 工具的辅助函数
//!
//! 包含点击指示器、光标位置查询、AX role 转换等独立辅助逻辑。

use serde_json::Value;
use std::process::{Command, Stdio};

/// 在屏幕坐标处显示点击指示器
pub(crate) fn show_click_indicator(x: f64, y: f64, label: Option<&str>) {
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
            .spawn();
    }
}

fn which_j_indicator() -> Option<std::path::PathBuf> {
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
pub(crate) fn get_cursor_position() -> Option<(f64, f64)> {
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

/// 将 AX role 转换为 AppleScript 元素类型名
pub(crate) fn ax_role_to_applescript(role: &str) -> &'static str {
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
