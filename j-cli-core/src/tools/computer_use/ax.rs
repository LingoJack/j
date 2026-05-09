use std::path::PathBuf;
use std::process::Command;

use serde::{Deserialize, Serialize};

use super::error::AicError;

// --- Data types matching j-ax JSON output ---

/// UI 元素的矩形区域，包含位置和尺寸信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frame {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// 无障碍树节点，表示 UI 元素的层级结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxNode {
    pub role: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub value: Option<String>,
    pub frame: Option<Frame>,
    pub enabled: Option<bool>,
    pub children: Option<Vec<AxNode>>,
}

/// 元素查找结果，包含角色、标题、位置和中心坐标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindResult {
    pub role: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub value: Option<String>,
    pub frame: Option<Frame>,
    pub center_x: f64,
    pub center_y: f64,
}

// --- Binary discovery ---

fn find_ax_binary() -> Result<PathBuf, AicError> {
    // Look next to the current executable first
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let path = dir.join("j-ax");
        if path.exists() {
            return Ok(path);
        }
    }
    // Fallback: check PATH via `which`
    if let Ok(output) = Command::new("which").arg("j-ax").output()
        && output.status.success()
    {
        let p = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !p.is_empty() {
            return Ok(PathBuf::from(p));
        }
    }
    Err(AicError::AxHelperNotFound)
}

fn run_ax_helper(args: &[&str]) -> Result<String, AicError> {
    let bin = find_ax_binary()?;
    let output = Command::new(&bin)
        .args(args)
        .output()
        .map_err(|e| AicError::AxQueryFailed(format!("failed to launch j-ax: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(AicError::AxQueryFailed(stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(stdout)
}

// --- Public query functions ---

/// 查询指定应用的辅助功能树，可选限制深度和仅可点击元素。
pub fn query_tree(
    app: Option<&str>,
    depth: Option<u32>,
    clickable: bool,
) -> Result<AxNode, AicError> {
    let mut args: Vec<&str> = vec!["tree"];
    if let Some(a) = app {
        args.push("--app");
        args.push(a);
    }
    let depth_str = depth.map(|d| d.to_string());
    if let Some(ref ds) = depth_str {
        args.push("--depth");
        args.push(ds);
    }
    if clickable {
        args.push("--clickable");
    }

    let json = run_ax_helper(&args)?;
    serde_json::from_str(&json).map_err(|e| AicError::AxParseFailed(e.to_string()))
}

/// 根据查询字符串在辅助功能树中查找匹配元素。
pub fn find_elements(
    query: &str,
    app: Option<&str>,
    role: Option<&str>,
) -> Result<Vec<FindResult>, AicError> {
    let mut args: Vec<&str> = vec!["find", query];
    if let Some(a) = app {
        args.push("--app");
        args.push(a);
    }
    if let Some(r) = role {
        args.push("--role");
        args.push(r);
    }

    let json = run_ax_helper(&args)?;
    serde_json::from_str(&json).map_err(|e| AicError::AxParseFailed(e.to_string()))
}

/// 收集指定应用中所有可交互元素（按钮、链接等）。
pub fn collect_interactive_elements(app: Option<&str>) -> Result<Vec<FindResult>, AicError> {
    let mut args: Vec<&str> = vec!["interactive"];
    if let Some(a) = app {
        args.push("--app");
        args.push(a);
    }

    let json = run_ax_helper(&args)?;
    serde_json::from_str(&json).map_err(|e| AicError::AxParseFailed(e.to_string()))
}
