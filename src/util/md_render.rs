/// 在终端中渲染 Markdown 文本
/// 优先通过嵌入的 md_render 二进制渲染（效果更佳），
/// 如果不可用则 fallback 到 termimad
#[macro_export]
macro_rules! md {
    ($($arg:tt)*) => {{
        let text = format!($($arg)*);
        $crate::util::md_render::render_md(&text);
    }};
}

/// 在终端中渲染单行 Markdown（不换行，用于内联场景）
#[macro_export]
macro_rules! md_inline {
    ($($arg:tt)*) => {{
        let text = format!($($arg)*);
        termimad::print_inline(&text);
    }};
}

/// 编译时从 plugin/md_render/bin/ 目录嵌入
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const ASK_BINARY: &[u8] = include_bytes!("../../plugin/md_render/bin/md_render-darwin-arm64");

/// 获取嵌入的 render 二进制路径
/// 首次调用时释放嵌入的二进制到 ~/.jdata/bin/md_render，后续复用
fn md_render_path() -> Option<std::path::PathBuf> {
    // 如果不是 macOS arm64，则返回 None
    #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
    {
        return None;
    }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        use std::os::unix::fs::PermissionsExt;

        let data_dir = crate::config::YamlConfig::data_dir();
        let bin_dir = data_dir.join("bin");
        let ask_path = bin_dir.join("md_render");

        if ask_path.exists() {
            // 已释放过，检查大小是否一致（版本更新时自动覆盖）
            if let Ok(meta) = std::fs::metadata(&ask_path) {
                if meta.len() == ASK_BINARY.len() as u64 {
                    return Some(ask_path);
                }
            }
        }

        // 首次释放或版本更新，写入嵌入的二进制
        if std::fs::create_dir_all(&bin_dir).is_err() {
            return None;
        }
        if std::fs::write(&ask_path, ASK_BINARY).is_err() {
            return None;
        }
        // 设置可执行权限 (chmod 755)
        if let Ok(meta) = std::fs::metadata(&ask_path) {
            let mut perms = meta.permissions();
            perms.set_mode(0o755);
            let _ = std::fs::set_permissions(&ask_path, perms);
        }

        Some(ask_path)
    }
}

/// 渲染 Markdown 文本到终端
/// 优先通过嵌入的 ask 二进制渲染（stdin → stdout，效果更佳），
/// 如果不可用则 fallback 到 termimad
pub fn render_md(text: &str) {
    use std::io::Write;
    use std::process::{Command, Stdio};

    // 获取嵌入的 render 二进制路径
    let renderer_path = md_render_path();

    if let Some(path) = renderer_path {
        // 调用 render：直接从 stdin 读取 Markdown，渲染后输出 stdout
        let result = Command::new(&path)
            .stdin(Stdio::piped())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn();

        match result {
            Ok(mut child) => {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(text.as_bytes());
                    drop(stdin);
                }
                let _ = child.wait();
                return;
            }
            Err(_) => {}
        }
    }

    // fallback 到 termimad
    termimad::print_text(text);
}
