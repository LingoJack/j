//! `j read <file>` — 在浏览器中打开 Typora 风的实时编辑器。
//!
//! 入口：[`handle_read`]。命令启动后会拉起一个本地 axum 服务，
//! 浏览器打开三栏 UI（左：文件树 / 中：编辑+预览 / 右：大纲）。
//!
//! 子模块：
//! - [`renderer`] — 单文件 → JSON payload 转换（按需调用，不再启动期一次性渲染）
//! - [`server`]   — axum HTTP 服务（多路由：`/api/initial`、`/api/file`、`/api/list`、`/api/parse`、`/api/save`）
//! - [`embed`]    — 编译期嵌入的 Reader SPA 资源

mod embed;
pub mod renderer;
mod server;

use crate::config::YamlConfig;
use std::path::{Path, PathBuf};
use std::process::Command;

/// 单文件大小上限：5 MiB。`/api/file` 与 `/api/save` 共用。
pub(crate) const MAX_FILE_SIZE: u64 = 5 * 1024 * 1024;

/// 资源（图片等）大小上限：20 MiB。`/api/asset` 使用。
pub(crate) const MAX_ASSET_SIZE: u64 = 20 * 1024 * 1024;

/// 单次目录列出的最大条目数。超过则前端提示「目录过大」。
pub(crate) const MAX_DIR_ENTRIES: usize = 2000;

/// `j read <file>` 命令入口。
pub fn handle_read(
    file_path: &str,
    port: Option<u16>,
    no_open: bool,
    tab: bool,
    _config: &mut YamlConfig,
) {
    if let Err(msg) = run(file_path, port, no_open, tab) {
        eprintln!("❌ {msg}");
        std::process::exit(1);
    }
}

fn run(file_path: &str, port: Option<u16>, no_open: bool, tab: bool) -> Result<(), String> {
    let expanded = expand_tilde(file_path);
    let path = Path::new(&expanded);

    // 1. 校验：必须存在；按是文件还是目录分流
    let metadata =
        std::fs::metadata(path).map_err(|e| format!("无法读取路径 \"{file_path}\"：{e}"))?;
    let canonical =
        std::fs::canonicalize(path).map_err(|e| format!("无法解析路径 \"{file_path}\"：{e}"))?;

    let (initial_path, root_dir): (Option<PathBuf>, PathBuf) = if metadata.is_file() {
        // 文件入口：作为 initial tab；root_dir = 父目录
        if metadata.len() > MAX_FILE_SIZE {
            return Err(format!(
                "文件过大（{} 字节，超过 {} 字节上限），暂不支持预览",
                metadata.len(),
                MAX_FILE_SIZE
            ));
        }
        let root = canonical
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| canonical.clone());
        (Some(canonical), root)
    } else if metadata.is_dir() {
        // 目录入口：仅打开文件树，不预选任何文件
        (None, canonical)
    } else {
        return Err(format!("\"{file_path}\" 不是普通文件或目录"));
    };

    // 2. 抢先分配端口（若未指定）—— 用于在 server 启动前就打开浏览器
    let actual_port = match port {
        Some(p) => p,
        None => probe_free_port()?,
    };
    let url = format!("http://127.0.0.1:{actual_port}/");

    if !no_open {
        match open_in_browser(&url, tab) {
            Ok(()) => {}
            Err(e) => eprintln!("⚠️  自动打开浏览器失败（{e}），请手动访问 {url}"),
        }
    } else {
        println!("📖 已禁用自动打开浏览器，请手动访问：{url}");
    }

    // 3. 启动 server，阻塞至 Ctrl-C / 浏览器关闭
    server::serve_blocking(initial_path, root_dir, Some(actual_port))
}

/// 探测一个可用端口：绑定 `127.0.0.1:0`，立刻释放，返回端口号。
fn probe_free_port() -> Result<u16, String> {
    let listener =
        std::net::TcpListener::bind("127.0.0.1:0").map_err(|e| format!("无法分配本地端口：{e}"))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("获取端口失败：{e}"))?
        .port();
    drop(listener);
    Ok(port)
}

/// 跨平台打开 URL。
///
/// 默认（`tab = false`）尝试用 **Chrome app 模式**（`--app=URL`）打开 ——
/// 这种窗口没有标签栏，整个窗口只承载一个网页，所以 `⌘W` / `⌘T` 等
/// 快捷键会被网页接收（`preventDefault` 才有意义）。普通标签页里 `⌘W`
/// 会被 Chrome 自身吞掉关掉标签，根本传不到 JS。
///
/// 失败（找不到 Chrome / Edge）回退到系统默认浏览器（普通标签页）。
/// 用户也可以显式 `j read --tab <file>` 走标签页模式。
fn open_in_browser(url: &str, tab: bool) -> Result<(), String> {
    if !tab {
        if let Ok(()) = try_open_app_mode(url) {
            return Ok(());
        }
    }
    open_default(url)
}

/// 尝试用 Chrome / Edge / Chromium 的 `--app=URL` 模式打开。
/// 失败返回 Err，调用方负责回退。
fn try_open_app_mode(url: &str) -> Result<(), String> {
    let arg_app = format!("--app={url}");

    #[cfg(target_os = "macos")]
    {
        // macOS：用 `open -na <App> --args --app=URL`
        // -n: 新进程；-a: 指定 app；--args 之后的参数透传给 app。
        for app_name in [
            "Google Chrome",
            "Microsoft Edge",
            "Chromium",
            "Brave Browser",
        ] {
            let status = Command::new("open")
                .args(["-na", app_name, "--args", &arg_app])
                .status();
            match status {
                Ok(s) if s.success() => return Ok(()),
                _ => continue,
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        // Windows：直接 `chrome --app=URL`，依赖 PATH 或常见安装路径
        for exe in [
            "chrome.exe",
            "msedge.exe",
            r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
        ] {
            let status = Command::new(exe).arg(&arg_app).status();
            if let Ok(s) = status
                && s.success()
            {
                return Ok(());
            }
        }
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        // Linux：尝试常见的 chrome / chromium 二进制名
        for exe in [
            "google-chrome",
            "google-chrome-stable",
            "chromium",
            "chromium-browser",
            "microsoft-edge",
            "brave-browser",
        ] {
            let status = Command::new(exe).arg(&arg_app).status();
            if let Ok(s) = status
                && s.success()
            {
                return Ok(());
            }
        }
    }

    Err("未找到可用的 Chromium 系浏览器".to_string())
}

/// 普通标签页：调系统默认浏览器。
fn open_default(url: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let result = Command::new("open").arg(url).status();

    #[cfg(target_os = "windows")]
    let result = Command::new("cmd").args(["/C", "start", "", url]).status();

    #[cfg(all(unix, not(target_os = "macos")))]
    let result = Command::new("xdg-open").arg(url).status();

    let status = result.map_err(|e| format!("无法启动浏览器：{e}"))?;
    if !status.success() {
        return Err(format!("浏览器进程返回非零状态：{status}"));
    }
    Ok(())
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
