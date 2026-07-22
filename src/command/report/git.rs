//! 日报 git 同步：push / pull / set-url、仓库管理。

use crate::config::YamlConfig;
use crate::constants::{config_key, section};
use crate::{error, info, usage};
use chrono::Local;
use std::fs;
use std::path::Path;
use std::process::Command;

use super::io::get_report_dir;

// ========== 对外公共接口 ==========

/// 处理 reportctl set-url 命令：设置 git 仓库地址
pub fn handle_set_url(url: Option<&str>, config: &mut YamlConfig) {
    match url {
        Some(u) if !u.is_empty() => {
            let old = config
                .get_property(section::REPORT, config_key::GIT_REPO)
                .cloned();
            config
                .set_property(section::REPORT, config_key::GIT_REPO, u)
                .unwrap_or_else(|e| error!("保存配置失败: {}", e));

            // 如果日报目录已有 .git，同步更新 remote origin
            if let Some(dir) = get_report_dir(config) {
                let git_dir = Path::new(&dir).join(".git");
                if git_dir.exists() {
                    sync_git_remote(config);
                }
            }

            match old {
                Some(old_url) if !old_url.is_empty() => {
                    info!("git 仓库地址已更新: {} -> {}", old_url, u);
                }
                _ => {
                    info!("git 仓库地址已设置: {}", u);
                }
            }
        }
        _ => {
            // 无参数时显示当前配置
            match config.get_property(section::REPORT, config_key::GIT_REPO) {
                Some(url) if !url.is_empty() => {
                    info!("当前 git 仓库地址: {}", url);
                }
                _ => {
                    info!("尚未配置 git 仓库地址");
                    usage!("reportctl set-url <repo_url>");
                }
            }
        }
    }
}

/// 处理 reportctl push 命令：推送周报到远程仓库
pub fn handle_push(commit_msg: Option<&str>, config: &YamlConfig) {
    // 检查 git_repo 配置
    let git_repo = config.get_property(section::REPORT, config_key::GIT_REPO);
    match git_repo {
        Some(url) if !url.is_empty() => {}
        _ => {
            error!("尚未配置 git 仓库地址，请先执行: j reportctl set-url <repo_url>");
            return;
        }
    }

    // 确保 git 仓库已初始化
    if !ensure_git_repo(config) {
        return;
    }

    let default_msg = format!("update report {}", Local::now().format("%Y-%m-%d %H:%M"));
    let msg = commit_msg.unwrap_or(&default_msg);

    info!("正在推送周报到远程仓库...");

    // git add .
    if let Some(status) = run_git(&["add", "."], config) {
        if !status.success() {
            error!("git add 失败");
            return;
        }
    } else {
        return;
    }

    // git commit -m "<msg>"
    if let Some(status) = run_git(&["commit", "-m", msg], config) {
        if !status.success() {
            info!("git commit 返回非零退出码（可能没有新变更）");
        }
    } else {
        return;
    }

    // git push origin main
    if let Some(status) = run_git(&["push", "-u", "origin", "main"], config) {
        if status.success() {
            info!("周报已成功推送到远程仓库");
        } else {
            error!("git push 失败，请检查网络连接和仓库权限");
        }
    }
}

/// 处理 reportctl pull 命令：从远程仓库拉取周报
pub fn handle_pull(config: &YamlConfig) {
    let git_repo = config.get_property(section::REPORT, config_key::GIT_REPO);
    let repo_url = match git_repo {
        Some(url) if !url.is_empty() => url.clone(),
        _ => {
            error!("尚未配置 git 仓库地址，请先执行: j reportctl set-url <repo_url>");
            return;
        }
    };

    let dir = match get_report_dir(config) {
        Some(d) => d,
        None => {
            error!("无法确定日报目录");
            return;
        }
    };

    let git_dir = Path::new(&dir).join(".git");

    if !git_dir.exists() {
        pull_via_clone(&repo_url, &dir, config);
    } else {
        pull_existing_repo(&repo_url, &dir, config);
    }
}

// ========== pull 内部实现 ==========

/// 日报目录无 .git：clone 远程仓库
fn pull_via_clone(repo_url: &str, dir: &str, config: &YamlConfig) {
    info!("日报目录尚未初始化，正在从远程仓库克隆...");

    // 先备份已有文件
    let report_path = config.report_file_path();
    let has_existing = report_path.exists()
        && fs::metadata(&report_path)
            .map(|m| m.len() > 0)
            .unwrap_or(false);

    if has_existing {
        let backup_path = report_path.with_extension("md.bak");
        if let Err(e) = fs::copy(&report_path, &backup_path) {
            error!("备份现有日报文件失败: {}", e);
        } else {
            info!("已备份现有日报到: {:?}", backup_path);
        }
    }

    // clone 到临时目录再移动
    let temp_dir = Path::new(dir).with_file_name(".report_clone_tmp");
    let _ = fs::remove_dir_all(&temp_dir);

    let result = Command::new("git")
        .args(["clone", "-b", "main", repo_url, &temp_dir.to_string_lossy()])
        .status();

    match result {
        Ok(status) if status.success() => {
            let _ = fs::remove_dir_all(dir);
            if let Err(e) = fs::rename(&temp_dir, dir) {
                error!("移动克隆仓库失败: {}，临时目录: {:?}", e, temp_dir);
                return;
            }
            info!("成功从远程仓库克隆周报");
        }
        Ok(_) => {
            error!("git clone 失败，请检查仓库地址和网络连接");
            let _ = fs::remove_dir_all(&temp_dir);
        }
        Err(e) => {
            error!("执行 git clone 失败: {}", e);
            let _ = fs::remove_dir_all(&temp_dir);
        }
    }
}

/// 日报目录已有 .git：fetch + pull（处理空仓库和正常仓库两种情况）
fn pull_existing_repo(repo_url: &str, dir: &str, config: &YamlConfig) {
    // 先同步 remote URL
    sync_git_remote(config);

    // 检测是否是空仓库（unborn branch）
    let has_commits = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(dir)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !has_commits {
        pull_empty_repo(dir, config);
    } else {
        pull_normal_repo(dir, config);
    }

    let _ = repo_url; // 已在 sync_git_remote 中使用
}

/// 空仓库：fetch + reset --hard
fn pull_empty_repo(_dir: &str, config: &YamlConfig) {
    info!("本地仓库尚无提交，正在从远程仓库拉取...");

    // 备份本地文件
    let report_path = config.report_file_path();
    if report_path.exists()
        && fs::metadata(&report_path)
            .map(|m| m.len() > 0)
            .unwrap_or(false)
    {
        let backup_path = report_path.with_extension("md.bak");
        let _ = fs::copy(&report_path, &backup_path);
        info!("已备份本地日报到: {:?}", backup_path);
    }

    // git fetch origin main
    if let Some(status) = run_git(&["fetch", "origin", "main"], config) {
        if !status.success() {
            error!("git fetch 失败，请检查网络连接和仓库地址");
            return;
        }
    } else {
        return;
    }

    // git reset --hard origin/main
    if let Some(status) = run_git(&["reset", "--hard", "origin/main"], config) {
        if status.success() {
            info!("成功从远程仓库拉取周报");
        } else {
            error!("git reset 失败");
        }
    }
}

/// 正常仓库：stash + pull --rebase + stash pop
fn pull_normal_repo(dir: &str, config: &YamlConfig) {
    info!("正在从远程仓库拉取最新周报...");

    // 先暂存本地未跟踪/修改的文件
    let _ = run_git(&["add", "-A"], config);
    let stash_result = Command::new("git")
        .args(["stash", "push", "-m", "auto-stash-before-pull"])
        .current_dir(dir)
        .output();
    let has_stash = match &stash_result {
        Ok(output) => {
            let msg = String::from_utf8_lossy(&output.stdout);
            !msg.contains("No local changes")
        }
        Err(_) => false,
    };

    // 执行 pull
    let pull_ok = if let Some(status) = run_git(&["pull", "origin", "main", "--rebase"], config) {
        if status.success() {
            info!("周报已更新到最新版本");
            true
        } else {
            error!("git pull 失败，请检查网络连接或手动解决冲突");
            false
        }
    } else {
        false
    };

    // 恢复 stash
    if has_stash
        && let Some(status) = run_git(&["stash", "pop"], config)
        && !status.success()
        && pull_ok
    {
        info!("stash pop 存在冲突，请手动合并本地修改（已保存在 git stash 中）");
    }
}

// ========== git 工具函数 ==========

/// 在日报目录下执行 git 命令
fn run_git(args: &[&str], config: &YamlConfig) -> Option<std::process::ExitStatus> {
    let dir = match get_report_dir(config) {
        Some(d) => d,
        None => {
            error!("无法确定日报目录");
            return None;
        }
    };

    let result = Command::new("git").args(args).current_dir(&dir).status();

    match result {
        Ok(status) => Some(status),
        Err(e) => {
            error!("执行 git 命令失败: {}", e);
            None
        }
    }
}

/// 检查日报目录是否已初始化 git 仓库，如果没有则初始化并配置 remote
fn ensure_git_repo(config: &YamlConfig) -> bool {
    let dir = match get_report_dir(config) {
        Some(d) => d,
        None => {
            error!("无法确定日报目录");
            return false;
        }
    };

    let git_dir = Path::new(&dir).join(".git");
    if git_dir.exists() {
        sync_git_remote(config);
        return true;
    }

    let git_repo = config.get_property(section::REPORT, config_key::GIT_REPO);
    match git_repo {
        Some(url) if !url.is_empty() => {
            let repo_url = url.clone();
            info!("日报目录尚未初始化 git 仓库，正在初始化...");

            // git init -b main
            if let Some(status) = run_git(&["init", "-b", "main"], config) {
                if !status.success() {
                    error!("git init 失败");
                    return false;
                }
            } else {
                return false;
            }

            // git remote add origin <repo_url>
            if let Some(status) = run_git(&["remote", "add", "origin", &repo_url], config) {
                if !status.success() {
                    error!("git remote add 失败");
                    return false;
                }
            } else {
                return false;
            }

            info!("git 仓库初始化完成，remote: {}", repo_url);
            true
        }
        _ => {
            error!("尚未配置 git 仓库地址，请先执行: j reportctl set-url <repo_url>");
            false
        }
    }
}

/// 同步 git remote origin URL 与配置文件中的 git_repo 保持一致
fn sync_git_remote(config: &YamlConfig) {
    let git_repo = match config.get_property(section::REPORT, config_key::GIT_REPO) {
        Some(url) if !url.is_empty() => url.clone(),
        _ => return,
    };

    let dir = match get_report_dir(config) {
        Some(d) => d,
        None => return,
    };

    let current_url = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(&dir)
        .output();

    match current_url {
        Ok(output) if output.status.success() => {
            let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if url != git_repo {
                let _ = run_git(&["remote", "set-url", "origin", &git_repo], config);
                info!("已同步 remote origin: {} -> {}", url, git_repo);
            }
        }
        _ => {
            let _ = run_git(&["remote", "add", "origin", &git_repo], config);
        }
    }
}
