use crate::constants;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// 版本检查缓存结构
#[derive(Debug, Deserialize)]
struct VersionCache {
    /// 最后检查时间（Unix 时间戳，秒）
    last_check: u64,
    /// 最新版本号
    latest_version: String,
    /// 当前版本号（用于判断是否需要重新检查）
    current_version: String,
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
        let v = v.trim_start_matches('v');
        v.split('.')
            .filter_map(|s| s.parse().ok())
            .collect()
    };

    let current_parts = parse_version(current);
    let latest_parts = parse_version(latest);

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

/// 读取缓存
fn read_cache() -> Option<VersionCache> {
    let path = cache_file_path();
    if !path.exists() {
        return None;
    }

    let content = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

/// 判断缓存是否需要刷新
fn cache_needs_refresh() -> bool {
    let current_version = constants::VERSION;
    let now = current_timestamp();

    match read_cache() {
        Some(c) => {
            c.current_version != current_version
                || now.saturating_sub(c.last_check) >= constants::VERSION_CHECK_INTERVAL_SECS
        }
        None => true,
    }
}

/// 【阶段1：即时检查】从缓存中读取是否有新版本，不涉及网络，立即返回
/// 返回 Some(latest_version) 表示有更新可用
pub fn check_cached() -> Option<String> {
    let current_version = constants::VERSION;
    let cache = read_cache()?;

    if is_newer_version(current_version, &cache.latest_version) {
        Some(cache.latest_version)
    } else {
        None
    }
}

/// 【阶段2：后台刷新】生成临时脚本并 fork 独立子进程静默刷新缓存
/// 子进程完全独立于主进程，主进程退出后子进程仍能完成网络请求
/// 下次运行 check_cached() 时就能读到新的版本信息
pub fn refresh_cache_in_background() {
    if !cache_needs_refresh() {
        return;
    }

    let cache_path = cache_file_path();
    let current_version = constants::VERSION;
    let url = constants::GITHUB_RELEASES_API;

    // 确保缓存文件的父目录存在
    if let Some(parent) = cache_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    // 生成临时脚本文件
    let script_path = cache_path.with_extension("sh");
    let script_content = build_check_script(url, current_version, &cache_path.to_string_lossy());

    if fs::write(&script_path, &script_content).is_err() {
        return;
    }

    // 用 nohup 在后台 fork 独立子进程执行脚本，主进程退出不影响
    let _ = std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(format!(
            "nohup /bin/sh \"{}\" >/dev/null 2>&1 &",
            script_path.to_string_lossy()
        ))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

/// 构建版本检查 shell 脚本内容
fn build_check_script(url: &str, current_version: &str, cache_path: &str) -> String {
    format!(
        r##"
            #!/bin/sh
            # 版本检查脚本（由 j-cli 自动生成，执行后自动删除）
            RESPONSE=$(curl -s -S -L --connect-timeout 3 --max-time 8 \
              -H "Accept: application/vnd.github.v3+json" \
              -H "User-Agent: j-cli" \
              "{url}")
            
            if [ $? -ne 0 ]; then
              rm -f "$0"
              exit 0
            fi
            
            TAG=$(echo "$RESPONSE" | grep -o '"tag_name"[[:space:]]*:[[:space:]]*"[^"]*"' | head -1 | sed 's/.*"\([^"]*\)"$/\1/' | sed 's/^v//')
            
            if [ -n "$TAG" ]; then
              NOW=$(date +%s)
              cat > "{cache_path}" << CACHE_EOF
            {{
              "last_check": $NOW,
              "latest_version": "$TAG",
              "current_version": "{current_version}"
            }}
            CACHE_EOF
            fi
            
            # 清理临时脚本自身
            rm -f "$0"
        "##,
        url = url,
        current_version = current_version,
        cache_path = cache_path,
    )
}

/// 【一步完成】检查缓存 + 后台刷新 + 如果有更新则打印提示
#[allow(dead_code)]
pub fn check_and_hint() {
    if let Some(latest_version) = check_cached() {
        print_update_hint(&latest_version);
    }
    refresh_cache_in_background();
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