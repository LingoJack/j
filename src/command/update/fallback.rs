use colored::Colorize;

use super::codesign::fix_codesign_and_quarantine;
use super::github_auth::get_github_auth_token;
#[cfg(target_os = "macos")]
use super::indicator::install_indicator_from_release;
use super::restart::restart_self;

/// 备用更新方案（当 self_update 因 TLS/API 限流等问题失败时）
/// Unix: 使用 curl 下载
/// Windows: 使用 PowerShell Invoke-WebRequest 下载（绕过 rustls 问题）
pub(crate) fn perform_update_fallback(target: &str, interactive: bool) {
    // 获取最新版本号
    let version = get_latest_version_fallback();
    let version_display = version.as_deref().unwrap_or("未知").to_string();
    println!("最新版本: {}", version_display.cyan());

    let version = match version {
        Some(v) => v,
        None => {
            println!("{}", "无法获取最新版本号".red());
            println!("请尝试手动更新:");
            #[cfg(unix)]
            println!(
                "  curl -fsSL https://raw.githubusercontent.com/LingoJack/jcli/main/install.sh | sh"
            );
            #[cfg(windows)]
            println!(
                "  irm https://raw.githubusercontent.com/LingoJack/jcli/main/install.ps1 | iex"
            );
            println!("或访问 GitHub Releases 页面查看最新版本:");
            println!("  https://github.com/LingoJack/jcli/releases");
            return;
        }
    };

    // 确定 j 所在目录
    let exe_path = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            println!("{} {}", "无法获取当前可执行文件路径:".red(), e);
            return;
        }
    };
    let exe_dir = match exe_path.parent() {
        Some(d) => d.to_path_buf(),
        None => {
            println!("{}", "无法获取可执行文件所在目录".red());
            return;
        }
    };

    let tag = if version.starts_with('v') {
        version.clone()
    } else {
        format!("v{}", version)
    };

    let asset_name = format!("j-{}", target);

    #[cfg(unix)]
    let url = format!(
        "https://github.com/LingoJack/jcli/releases/download/{}/{}.tar.gz",
        tag, asset_name
    );

    #[cfg(windows)]
    let url = format!(
        "https://github.com/LingoJack/jcli/releases/download/{}/{}.zip",
        tag, asset_name
    );

    println!("下载地址: {}", url.dimmed());

    // 创建临时目录
    let tmp_dir = std::env::temp_dir().join("j-update-curl");
    let _ = std::fs::create_dir_all(&tmp_dir);

    #[cfg(unix)]
    let tmp_archive = tmp_dir.join(format!("{}.tar.gz", asset_name));

    #[cfg(windows)]
    let tmp_archive = tmp_dir.join(format!("{}.zip", asset_name));

    // 下载
    println!("{}", "正在下载...".yellow());

    #[cfg(unix)]
    let download_result = {
        std::process::Command::new("curl")
            .args(["-fsSL", "--progress-bar", "-o"])
            .arg(&tmp_archive)
            .arg(&url)
            .status()
    };

    #[cfg(windows)]
    let download_result = {
        let tmp_archive_str = tmp_archive.to_string_lossy().to_string();
        // 使用 PowerShell 的 Invoke-WebRequest 下载（使用系统原生 TLS，避免 rustls 问题）
        let ps_script = format!(
            "[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; \
             Invoke-WebRequest -Uri '{}' -OutFile '{}' -UseBasicParsing",
            url, tmp_archive_str
        );
        std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-Command", &ps_script])
            .status()
    };

    match download_result {
        Ok(status) if status.success() => {
            // 验证下载文件存在且非空
            if !tmp_archive.exists() {
                println!("{}", "下载文件不存在".red());
                let _ = std::fs::remove_dir_all(&tmp_dir);
                return;
            }
            let file_size = std::fs::metadata(&tmp_archive)
                .map(|m| m.len())
                .unwrap_or(0);
            if file_size == 0 {
                println!("{}", "下载文件为空".red());
                let _ = std::fs::remove_dir_all(&tmp_dir);
                return;
            }
        }
        Ok(status) => {
            println!(
                "{} 退出码: {}",
                "下载失败".red(),
                status.code().unwrap_or(-1)
            );
            let _ = std::fs::remove_dir_all(&tmp_dir);
            return;
        }
        Err(e) => {
            println!("{} {}", "下载失败:".red(), e);
            let _ = std::fs::remove_dir_all(&tmp_dir);
            return;
        }
    }

    // 解压
    println!("{}", "正在解压...".yellow());

    #[cfg(unix)]
    let extract_result = std::process::Command::new("tar")
        .args(["-xzf"])
        .arg(&tmp_archive)
        .args(["-C"])
        .arg(&tmp_dir)
        .status();

    #[cfg(windows)]
    let extract_result = {
        // Windows 10+ 自带 tar 命令，优先使用 tar（解压由 7z 创建的 zip 可靠性更高）
        // 回退使用 PowerShell Expand-Archive
        let tar_result = std::process::Command::new("tar")
            .args(["-xf"])
            .arg(&tmp_archive)
            .args(["-C"])
            .arg(&tmp_dir)
            .status();

        match tar_result {
            Ok(status) if status.success() => Ok(status),
            _ => {
                // tar 解压 zip 失败，回退到 PowerShell
                std::process::Command::new("powershell.exe")
                    .args([
                        "-NoProfile",
                        "-Command",
                        &format!(
                            "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
                            tmp_archive.display(),
                            tmp_dir.display()
                        ),
                    ])
                    .status()
            }
        }
    };

    match extract_result {
        Ok(status) if status.success() => {}
        Ok(status) => {
            println!(
                "{} 退出码: {}",
                "解压失败".red(),
                status.code().unwrap_or(-1)
            );
            let _ = std::fs::remove_dir_all(&tmp_dir);
            return;
        }
        Err(e) => {
            println!("{} {}", "解压失败:".red(), e);
            let _ = std::fs::remove_dir_all(&tmp_dir);
            return;
        }
    }

    // 替换二进制文件
    #[cfg(unix)]
    let src_bin = tmp_dir.join("j");

    #[cfg(windows)]
    let src_bin = tmp_dir.join("j.exe");

    #[cfg(unix)]
    let dst_bin = exe_dir.join("j");

    #[cfg(windows)]
    let dst_bin = exe_dir.join("j.exe");

    if !src_bin.exists() {
        println!("{}", "解压后未找到 j 二进制文件".red());
        let _ = std::fs::remove_dir_all(&tmp_dir);
        return;
    }

    // Windows 上替换正在运行的 exe 需要先重命名旧文件
    #[cfg(windows)]
    let rename_backup = if dst_bin.exists() {
        let backup = dst_bin.with_extension("exe.bak");
        let _ = std::fs::remove_file(&backup);
        match std::fs::rename(&dst_bin, &backup) {
            Ok(_) => Some(backup),
            Err(e) => {
                println!("{} {}", "无法重命名旧版本文件:".red(), e);
                println!("请尝试关闭所有 j 进程后重新执行更新");
                let _ = std::fs::remove_dir_all(&tmp_dir);
                return;
            }
        }
    } else {
        None
    };

    match std::fs::copy(&src_bin, &dst_bin) {
        Ok(_) => {
            // 设置可执行权限
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&dst_bin, std::fs::Permissions::from_mode(0o755));
            }
            // 修复 macOS 代码签名
            fix_codesign_and_quarantine(&dst_bin);
            println!(
                "{} {}",
                "更新成功！".green(),
                format!("版本: {}", version_display).cyan()
            );

            // Windows: 清理备份文件（延迟删除，因为旧进程可能还在运行）
            #[cfg(windows)]
            if let Some(ref backup) = rename_backup {
                let backup_str = backup.to_string_lossy().to_string();
                let cleanup_script = format!(
                    "Start-Sleep -Seconds 3; Remove-Item '{}' -Force -ErrorAction SilentlyContinue",
                    backup_str
                );
                let _ = std::process::Command::new("powershell.exe")
                    .args([
                        "-NoProfile",
                        "-WindowStyle",
                        "Hidden",
                        "-Command",
                        &cleanup_script,
                    ])
                    .spawn();
            }

            // 尝试同步安装 j-indicator（仅 macOS）
            #[cfg(target_os = "macos")]
            install_indicator_from_release(&version);

            if interactive {
                restart_self();
            }
        }
        Err(e) => {
            println!("{} {}", "安装失败:".red(), e);
            #[cfg(unix)]
            println!("可能需要管理员权限，请尝试:");
            #[cfg(unix)]
            println!("  {}", "sudo j update".cyan());
            #[cfg(windows)]
            println!("请尝试以管理员身份运行 PowerShell 后重新执行更新");

            // Windows: 恢复备份
            #[cfg(windows)]
            if let Some(backup) = rename_backup {
                let _ = std::fs::rename(&backup, &dst_bin);
            }
        }
    }

    let _ = std::fs::remove_dir_all(&tmp_dir);
}

/// 通过外部工具获取最新版本号（模仿 install.sh 的多种回退策略）
/// Unix: 使用 curl
/// Windows: 使用 PowerShell Invoke-WebRequest
fn get_latest_version_fallback() -> Option<String> {
    println!("{}", "正在获取最新版本号...".yellow());

    let auth_token = get_github_auth_token();

    #[cfg(unix)]
    {
        get_latest_version_unix(&auth_token)
    }

    #[cfg(windows)]
    {
        get_latest_version_windows(&auth_token)
    }
}

#[cfg(unix)]
fn get_latest_version_unix(auth_token: &Option<String>) -> Option<String> {
    // 方法1: 使用 GitHub API（带认证）
    let mut api_cmd = std::process::Command::new("curl");
    api_cmd.args(["-fsSL", "-H", "User-Agent: j-cli-updater"]);
    if let Some(token) = auth_token {
        api_cmd.args(["-H", &format!("Authorization: token {}", token)]);
    }
    api_cmd.arg("https://api.github.com/repos/LingoJack/jcli/releases/latest");

    if let Ok(output) = api_cmd.output()
        && output.status.success()
    {
        let body = String::from_utf8_lossy(&output.stdout);
        let re = regex::Regex::new(r#"v[0-9]+\.[0-9]+\.[0-9]+"#).ok();
        // 从 JSON 中提取 tag_name
        for line in body.lines() {
            if line.contains("\"tag_name\"")
                && let Some(re) = &re
                && let Some(m) = re.find(line)
            {
                return Some(m.as_str().to_string());
            }
        }
    }

    // 方法2: 从 releases 页面解析重定向
    if let Ok(output) = std::process::Command::new("curl")
        .args(["-fsSL", "-o", "/dev/null", "-w", "%{url_effective}"])
        .arg("https://github.com/LingoJack/jcli/releases/latest")
        .output()
        && output.status.success()
    {
        let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        // 重定向 URL 格式: https://github.com/LingoJack/jcli/releases/tag/v12.8.10
        let re = regex::Regex::new(r#"v[0-9]+\.[0-9]+\.[0-9]+"#).ok();
        if let Some(re) = re
            && let Some(m) = re.find(&url)
        {
            return Some(m.as_str().to_string());
        }
    }

    None
}

#[cfg(windows)]
fn get_latest_version_windows(auth_token: &Option<String>) -> Option<String> {
    // Windows: 使用 PowerShell 获取最新版本号
    let mut auth_header = String::from("User-Agent: j-cli-updater");
    if let Some(token) = auth_token {
        auth_header.push_str(&format!(", Authorization: token {}", token));
    }

    // 方法1: 通过 GitHub API 获取
    let ps_script = format!(
        "[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; \
         try {{ \
             $r = Invoke-WebRequest -Uri 'https://api.github.com/repos/LingoJack/jcli/releases/latest' \
                 -Headers @{{'User-Agent'='j-cli-updater'}} -UseBasicParsing; \
             $j = $r.Content | ConvertFrom-Json; \
             Write-Output $j.tag_name \
         }} catch {{ Write-Output '' }}"
    );

    if let Ok(output) = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-Command", &ps_script])
        .output()
        && output.status.success()
    {
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let re = regex::Regex::new(r#"v[0-9]+\.[0-9]+\.[0-9]+"#).ok();
        if let Some(re) = re
            && let Some(m) = re.find(&version)
        {
            return Some(m.as_str().to_string());
        }
    }

    // 方法2: 从 releases 页面解析重定向 Location
    let ps_script2 = format!(
        "[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; \
         try {{ \
             $r = Invoke-WebRequest -Uri 'https://github.com/LingoJack/jcli/releases/latest' \
                 -MaximumRedirection 0 -ErrorAction SilentlyContinue; \
             Write-Output '' \
         }} catch {{ \
             $loc = $_.Exception.Response.Headers['Location']; \
             if ($loc) {{ Write-Output $loc }} else {{ Write-Output '' }} \
         }}"
    );

    if let Ok(output) = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-Command", &ps_script2])
        .output()
        && output.status.success()
    {
        let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let re = regex::Regex::new(r#"v[0-9]+\.[0-9]+\.[0-9]+"#).ok();
        if let Some(re) = re
            && let Some(m) = re.find(&url)
        {
            return Some(m.as_str().to_string());
        }
    }

    None
}
