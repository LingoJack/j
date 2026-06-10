//! `j read <path>` — 启动独立的 jstudio Tauri 应用。
//!
//! jstudio 已从 `j` 二进制中拆出，作为 `apps/jstudio` Git submodule 中的
//! Tauri 桌面应用维护。这里仅保留轻量 launcher：校验路径后定位 jstudio
//! 可执行文件或 `.app` bundle，并把目标路径作为启动参数传给 jstudio。

use crate::config::YamlConfig;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const JSTUDIO_BIN_ENV: &str = "JSTUDIO_BIN";
#[cfg(target_os = "macos")]
const MACOS_APP_PATHS: &[&str] = &[
    "apps/jstudio/src-tauri/target/release/bundle/macos/jstudio.app",
    "apps/jstudio/src-tauri/target/debug/bundle/macos/jstudio.app",
    "/Applications/jstudio.app",
];
#[cfg(all(unix, not(target_os = "macos")))]
const UNIX_BIN_PATHS: &[&str] = &[
    "apps/jstudio/src-tauri/target/release/jstudio",
    "apps/jstudio/src-tauri/target/debug/jstudio",
];
#[cfg(windows)]
const WINDOWS_BIN_PATHS: &[&str] = &[
    "apps\\jstudio\\src-tauri\\target\\release\\jstudio.exe",
    "apps\\jstudio\\src-tauri\\target\\debug\\jstudio.exe",
];

/// `j read <path>` 命令入口。
pub fn handle_read(file_path: &str, _config: &mut YamlConfig) {
    if let Err(msg) = run(file_path) {
        crate::error!("❌ {msg}");
    }
}

fn run(file_path: &str) -> Result<(), String> {
    let path = canonicalize_input(file_path)?;
    ensure_supported_target(&path, file_path)?;
    launch_jstudio(&path)
}

fn canonicalize_input(file_path: &str) -> Result<PathBuf, String> {
    let expanded = expand_tilde(file_path);
    let path = Path::new(&expanded);
    std::fs::canonicalize(path).map_err(|e| format!("无法解析路径 \"{file_path}\"：{e}"))
}

fn ensure_supported_target(path: &Path, original: &str) -> Result<(), String> {
    let metadata =
        std::fs::metadata(path).map_err(|e| format!("无法读取路径 \"{original}\"：{e}"))?;
    if metadata.is_file() || metadata.is_dir() {
        Ok(())
    } else {
        Err(format!("\"{original}\" 不是普通文件或目录"))
    }
}

fn launch_jstudio(target: &Path) -> Result<(), String> {
    if let Some(jstudio) = jstudio_from_env() {
        return spawn_jstudio_bin(&jstudio, target);
    }

    #[cfg(target_os = "macos")]
    if let Some(app) = first_existing_path(MACOS_APP_PATHS) {
        return open_macos_app(&app, target);
    }

    #[cfg(windows)]
    if let Some(bin) = first_existing_path(WINDOWS_BIN_PATHS) {
        return spawn_jstudio_bin(&bin, target);
    }

    #[cfg(not(windows))]
    if let Some(bin) = first_existing_path(UNIX_BIN_PATHS) {
        return spawn_jstudio_bin(&bin, target);
    }

    Err(jstudio_missing_message())
}

fn jstudio_from_env() -> Option<PathBuf> {
    std::env::var_os(JSTUDIO_BIN_ENV)
        .map(PathBuf::from)
        .filter(|path| path.exists())
}

fn first_existing_path(candidates: &[&str]) -> Option<PathBuf> {
    let root = repo_root();
    candidates.iter().find_map(|candidate| {
        let raw = PathBuf::from(candidate);
        let path = if raw.is_absolute() {
            raw
        } else {
            root.join(raw)
        };
        path.exists().then_some(path)
    })
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[cfg(target_os = "macos")]
fn open_macos_app(app: &Path, target: &Path) -> Result<(), String> {
    let status = Command::new("open")
        .arg("-na")
        .arg(app)
        .arg("--args")
        .arg(target)
        .status()
        .map_err(|e| format!("无法启动 jstudio.app：{e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("jstudio.app 启动失败：{status}"))
    }
}

fn spawn_jstudio_bin(bin: &Path, target: &Path) -> Result<(), String> {
    Command::new(bin)
        .arg(target)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("无法启动 jstudio：{e}"))?;
    Ok(())
}

fn jstudio_missing_message() -> String {
    format!(
        "未找到 jstudio。请先执行 `make install-jstudio`，或通过 {JSTUDIO_BIN_ENV} 指定 jstudio 可执行文件路径。"
    )
}

/// 展开 `~` 为用户 home 目录。
fn expand_tilde(path: &str) -> String {
    if (path == "~" || path.starts_with("~/"))
        && let Some(home) = dirs::home_dir()
    {
        if path == "~" {
            home.display().to_string()
        } else {
            format!("{}{}", home.display(), &path[1..])
        }
    } else {
        path.to_string()
    }
}
