use colored::Colorize;

/// 修复 macOS 上替换后的代码签名和隔离属性
/// 未签名的二进制文件在 Apple Silicon 上会被内核 SIGKILL
#[cfg(target_os = "macos")]
pub(crate) fn fix_codesign_and_quarantine(bin_path: &std::path::Path) {
    // 移除隔离属性（com.apple.quarantine）
    let _ = std::process::Command::new("xattr")
        .args(["-cr"])
        .arg(bin_path)
        .status();

    // 使用 ad-hoc 签名重签
    match std::process::Command::new("codesign")
        .args(["--force", "-s", "-"])
        .arg(bin_path)
        .status()
    {
        Ok(s) if s.success() => {
            // codesign 重新签名成功
        }
        _ => {
            println!(
                "{}",
                "  警告: codesign 签名失败，新版本可能无法启动"
                    .to_string()
                    .yellow()
            );
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn fix_codesign_and_quarantine(_bin_path: &std::path::Path) {}
