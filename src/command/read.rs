//! `j read <file>` — 在浏览器中预览文件。
//!
//! 入口：[`handle_read`]。当前实现支持 Markdown / 纯文本，
//! 未来通过 [`renderer::Renderer`] trait 扩展到 PPT / DOCX / XLSX。
//!
//! 子模块：
//! - [`renderer`] — 文档→JSON payload 的转换
//! - [`server`]   — axum HTTP 服务
//! - [`embed`]    — 编译期嵌入的 Reader SPA 资源

mod embed;
pub mod renderer;
mod server;

use crate::config::YamlConfig;
use std::path::Path;
use std::process::Command;

/// 单文件大小上限：5 MiB。超过则拒绝（避免瞬间把巨型文件加载进内存）。
const MAX_FILE_SIZE: u64 = 5 * 1024 * 1024;

/// `j read <file>` 命令入口。
pub fn handle_read(file_path: &str, port: Option<u16>, no_open: bool, _config: &mut YamlConfig) {
    if let Err(msg) = run(file_path, port, no_open) {
        eprintln!("❌ {msg}");
        std::process::exit(1);
    }
}

fn run(file_path: &str, port: Option<u16>, no_open: bool) -> Result<(), String> {
    let expanded = expand_tilde(file_path);
    let path = Path::new(&expanded);

    // 1. 检查文件存在 & 大小
    let metadata =
        std::fs::metadata(path).map_err(|e| format!("无法读取文件 \"{file_path}\"：{e}"))?;
    if !metadata.is_file() {
        return Err(format!("\"{file_path}\" 不是一个普通文件"));
    }
    if metadata.len() > MAX_FILE_SIZE {
        return Err(format!(
            "文件过大（{} 字节，超过 {} 字节上限），暂不支持预览",
            metadata.len(),
            MAX_FILE_SIZE
        ));
    }

    // 2. 读取 + 渲染
    let bytes = std::fs::read(path).map_err(|e| format!("读取文件 \"{file_path}\" 失败：{e}"))?;
    let renderer = renderer::pick_renderer(path);
    let filename = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(file_path)
        .to_string();
    let doc = renderer.render(&bytes, &filename)?;

    // 3. 自动打开浏览器（在 server 启动前抢先打开，复用本进程的端口分配结果不现实，
    //    所以我们改成：先绑定、拿到端口再打开 —— 在 server.rs 内做不便，这里先解析端口。）
    //    简化做法：使用 `port` 给定值；未给定时，先临时绑一次拿到端口、立刻释放、再启动 axum。
    //    （短窗口竞态可接受：仅在本机、用户唯一会话中使用。）
    let actual_port = match port {
        Some(p) => p,
        None => probe_free_port()?,
    };

    let url = format!("http://127.0.0.1:{actual_port}/");

    if !no_open {
        match open_in_browser(&url) {
            Ok(()) => {}
            Err(e) => eprintln!("⚠️  自动打开浏览器失败（{e}），请手动访问 {url}"),
        }
    } else {
        println!("📖 已禁用自动打开浏览器，请手动访问：{url}");
    }

    // 4. 启动 server，阻塞至 Ctrl-C
    server::serve_blocking(doc, Some(actual_port))
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
fn open_in_browser(url: &str) -> Result<(), String> {
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
