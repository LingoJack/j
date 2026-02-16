use crate::constants;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// 版本检查缓存结构
#[derive(Debug, Serialize, Deserialize)]
struct VersionCache {
    /// 最后检查时间（Unix 时间戳，秒）
    last_check: u64,
    /// 最新版本号
    latest_version: String,
    /// 当前版本号（用于判断是否需要重新检查）
    current_version: String,
}

/// GitHub Release API 响应结构
#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
}

/// 获取版本缓存文件路径
fn cache_file_path() -> PathBuf {
    crate::config::YamlConfig::data_dir().join(constants::VERSION_CHECK_CACHE_FILE)
}

/// 获取当前 Unix 时间戳（秒）
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// 比较语义化版本号，返回 true 表示 latest > current
fn is_newer_version(current: &str, latest: &str) -> bool {
    let parse_version = |v: &str| -> Vec<u32> {
        // 移除 'v' 前缀（如 v1.0.0 -> 1.0.0）
        let v = v.trim_start_matches('v');
        v.split('.')
            .filter_map(|s| s.parse().ok())
            .collect()
    };

    let current_parts = parse_version(current);
    let latest_parts = parse_version(latest);

    // 逐段比较
    for i in 0..std::cmp::max(current_parts.len(), latest_parts.len()) {
        let c = current_parts.get(i).unwrap_or(&0);
        let l = latest_parts.get(i).unwrap_or(&0);
        if l > c {
            return true;
        }
        if l < c {
            return false;
        }
    }
    false
}

/// 从 GitHub API 获取最新版本号
fn fetch_latest_version() -> Option<String> {
    let url = constants::GITHUB_RELEASES_API;

    // 使用 ureq 或 std::process 调用 curl
    // 为避免引入额外依赖，使用 curl 命令
    let output = std::process::Command::new("curl")
        .arg("-s")
        .arg("-S")
        .arg("-L")
        .arg("--connect-timeout")
        .arg("5")
        .arg("--max-time")
        .arg("10")
        .arg("-H")
        .arg("Accept: application/vnd.github.v3+json")
        .arg("-H")
        .arg("User-Agent: j-cli")
        .arg(url)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let response = String::from_utf8_lossy(&output.stdout);
    let release: GitHubRelease = serde_json::from_str(&response).ok()?;
    
    // 返回 tag_name，去掉 v 前缀
    Some(release.tag_name.trim_start_matches('v').to_string())
}

/// 读取缓存
fn read_cache() -> Option<VersionCache> {
    let path = cache_file_path();
    if !path.exists() {
        return None;
    }

    let content = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

/// 写入缓存
fn write_cache(cache: &VersionCache) {
    let path = cache_file_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(content) = serde_json::to_string_pretty(cache) {
        let _ = fs::write(&path, content);
    }
}

/// 检查是否有新版本，返回新版本号（如果有）
pub fn check_for_update() -> Option<String> {
    let current_version = constants::VERSION;
    let now = current_timestamp();

    // 读取缓存
    let cache = read_cache();

    // 判断是否需要重新检查
    let need_check = match &cache {
        Some(c) => {
            // 如果当前版本号变了，或者超过检查间隔，需要重新检查
            c.current_version != current_version
                || now - c.last_check >= constants::VERSION_CHECK_INTERVAL_SECS
        }
        None => true,
    };

    if !need_check {
        // 使用缓存的版本信息
        if let Some(c) = cache {
            if is_newer_version(current_version, &c.latest_version) {
                return Some(c.latest_version);
            }
        }
        return None;
    }

    // 从 GitHub 获取最新版本
    let latest_version = match fetch_latest_version() {
        Some(v) => v,
        None => {
            // 网络请求失败，但如果有缓存就使用缓存
            if let Some(c) = cache {
                if is_newer_version(current_version, &c.latest_version) {
                    return Some(c.latest_version);
                }
            }
            return None;
        }
    };

    // 写入缓存
    let new_cache = VersionCache {
        last_check: now,
        latest_version: latest_version.clone(),
        current_version: current_version.to_string(),
    };
    write_cache(&new_cache);

    // 比较版本
    if is_newer_version(current_version, &latest_version) {
        Some(latest_version)
    } else {
        None
    }
}

/// 打印新版本提示
pub fn print_update_hint(latest_version: &str) {
    eprintln!();
    eprintln!("┌─────────────────────────────────────────────────────────┐");
    eprintln!("│  🎉 有新版本可用！                                        │");
    eprintln!("│                                                         │");
    eprintln!("│  当前版本: {:<43}│", constants::VERSION);
    eprintln!("│  最新版本: {:<43}│", latest_version);
    eprintln!("│                                                         │");
    eprintln!("│  更新方式:                                               │");
    eprintln!("│    cargo install j-cli                                  │");
    eprintln!("│    或访问: https://github.com/{}/releases │", constants::GITHUB_REPO);
    eprintln!("└─────────────────────────────────────────────────────────┘");
    eprintln!();
}
