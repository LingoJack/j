#[cfg(target_os = "macos")]
use colored::Colorize;

#[cfg(target_os = "macos")]
use super::codesign::fix_codesign_and_quarantine;

#[cfg(target_os = "macos")]
/// 从 GitHub Release 下载并安装 j-indicator 到 j 同目录
/// 这是 best-effort 的：失败只打印警告，不影响主更新
pub(crate) fn install_indicator_from_release(version: &str) {
    // 确定 j 所在目录
    let j_dir = match std::env::current_exe() {
        Ok(p) => match p.parent() {
            Some(dir) => dir.to_path_buf(),
            None => return,
        },
        Err(_) => return,
    };

    let tag = if version.starts_with('v') {
        version.to_string()
    } else {
        format!("v{}", version)
    };
    let url = format!(
        "https://github.com/LingoJack/jcli/releases/download/{}/j-darwin-arm64.tar.gz",
        tag
    );

    println!("{}", "正在安装 j-indicator...".yellow());

    // 下载到临时文件
    let tmp_dir = std::env::temp_dir().join("j-update-indicator");
    let _ = std::fs::create_dir_all(&tmp_dir);
    let tmp_tar = tmp_dir.join("j-darwin-arm64.tar.gz");

    // 用 curl 下载（macOS 自带）
    let download = std::process::Command::new("curl")
        .args(["-fsSL", "-o"])
        .arg(&tmp_tar)
        .arg(&url)
        .output();

    match download {
        Ok(output) if output.status.success() => {}
        _ => {
            println!(
                "{}",
                "  j-indicator 下载失败，跳过（不影响 j 主程序）".dimmed()
            );
            let _ = std::fs::remove_dir_all(&tmp_dir);
            return;
        }
    }

    // 从 tarball 中提取 j-indicator
    let extract = std::process::Command::new("tar")
        .args(["-xzf"])
        .arg(&tmp_tar)
        .args(["-C"])
        .arg(&tmp_dir)
        .arg("j-indicator")
        .output();

    match extract {
        Ok(output) if output.status.success() => {
            let src = tmp_dir.join("j-indicator");
            let dst = j_dir.join("j-indicator");
            if src.exists() {
                match std::fs::copy(&src, &dst) {
                    Ok(_) => {
                        // 设置可执行权限
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            let _ = std::fs::set_permissions(
                                &dst,
                                std::fs::Permissions::from_mode(0o755),
                            );
                        }
                        // 修复 macOS 代码签名
                        fix_codesign_and_quarantine(&dst);
                        println!("{}", "  j-indicator 已安装".green());
                    }
                    Err(e) => {
                        println!(
                            "{}",
                            format!("  j-indicator 拷贝失败: {}（不影响 j 主程序）", e).dimmed()
                        );
                    }
                }
            }
        }
        _ => {
            println!(
                "{}",
                "  j-indicator 提取失败，跳过（不影响 j 主程序）".dimmed()
            );
        }
    }

    let _ = std::fs::remove_dir_all(&tmp_dir);
}
