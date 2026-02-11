use crate::config::YamlConfig;
use crate::constants::{section, config_key, search_flag, rmeta_action, REPORT_DATE_FORMAT, REPORT_SIMPLE_DATE_FORMAT, DEFAULT_CHECK_LINES};
use crate::util::fuzzy;
use crate::{error, info, usage};
use chrono::{Local, NaiveDate};
use colored::Colorize;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::process::Command;

const DATE_FORMAT: &str = REPORT_DATE_FORMAT;
const SIMPLE_DATE_FORMAT: &str = REPORT_SIMPLE_DATE_FORMAT;

// ========== report 命令 ==========

/// 处理 report 命令: j report <content...> 或 j reportctl new [date] / j reportctl sync [date]
pub fn handle_report(sub: &str, content: &[String], config: &mut YamlConfig) {
    if content.is_empty() {
        usage!("j report <content> | j reportctl new [date] | j reportctl sync [date]");
        return;
    }

    let first = content[0].as_str();

    // 元数据操作
    if sub == "reportctl" {
        match first {
            f if f == rmeta_action::NEW => {
                let date_str = content.get(1).map(|s| s.as_str());
                handle_week_update(date_str, config);
            }
            f if f == rmeta_action::SYNC => {
                let date_str = content.get(1).map(|s| s.as_str());
                handle_sync(date_str, config);
            }
            f if f == rmeta_action::PUSH => {
                let msg = content.get(1).map(|s| s.as_str());
                handle_push(msg, config);
            }
            f if f == rmeta_action::PULL => {
                handle_pull(config);
            }
            f if f == rmeta_action::SET_URL => {
                let url = content.get(1).map(|s| s.as_str());
                handle_set_url(url, config);
            }
            _ => {
                error!("❌ 未知的元数据操作: {}，可选: {}, {}, {}, {}, {}", first, rmeta_action::NEW, rmeta_action::SYNC, rmeta_action::PUSH, rmeta_action::PULL, rmeta_action::SET_URL);
            }
        }
        return;
    }

    // 常规日报写入
    let text = content.join(" ");
    let text = text.trim().trim_matches('"').to_string();

    if text.is_empty() {
        error!("⚠️ 内容为空，无法写入");
        return;
    }

    handle_daily_report(&text, config);
}

/// 获取日报文件路径（统一入口，自动创建目录和文件）
fn get_report_path(config: &YamlConfig) -> Option<String> {
    let report_path = config.report_file_path();

    // 确保父目录存在
    if let Some(parent) = report_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    // 如果文件不存在则自动创建空文件
    if !report_path.exists() {
        if let Err(e) = fs::write(&report_path, "") {
            error!("❌ 创建日报文件失败: {}", e);
            return None;
        }
        info!("📄 已自动创建日报文件: {:?}", report_path);
    }

    Some(report_path.to_string_lossy().to_string())
}

/// 获取日报工作目录下的 settings.json 路径
fn get_settings_json_path(report_path: &str) -> std::path::PathBuf {
    Path::new(report_path).parent().unwrap().join("settings.json")
}

/// 写入日报
fn handle_daily_report(content: &str, config: &mut YamlConfig) {
    let report_path = match get_report_path(config) {
        Some(p) => p,
        None => return,
    };

    info!("📂 日报文件路径：{}", report_path);

    let report_file = Path::new(&report_path);
    let config_path = get_settings_json_path(&report_path);

    load_config_from_json_and_sync(&config_path, config);

    let now = Local::now().date_naive();

    let week_num = config
        .get_property(section::REPORT, config_key::WEEK_NUM)
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(1);

    let last_day_str = config
        .get_property(section::REPORT, config_key::LAST_DAY)
        .cloned()
        .unwrap_or_default();

    let last_day = parse_date(&last_day_str);

    match last_day {
        Some(last_day) => {
            if now > last_day {
                // 进入新的一周
                let next_last_day = now + chrono::Duration::days(6);
                let new_week_title = format!(
                    "# Week{}[{}-{}]\n",
                    week_num,
                    now.format(DATE_FORMAT),
                    next_last_day.format(DATE_FORMAT)
                );
                update_config_files(week_num + 1, &next_last_day, &config_path, config);
                append_to_file(report_file, &new_week_title);
            }
        }
        None => {
            error!("❌ 无法解析 last_day 日期: {}", last_day_str);
            return;
        }
    }

    let today_str = now.format(SIMPLE_DATE_FORMAT);
    let log_entry = format!("- 【{}】 {}\n", today_str, content);
    append_to_file(report_file, &log_entry);
    info!("✅ 成功将内容写入：{}", report_path);
}

/// 处理 reportctl new 命令：开启新的一周
fn handle_week_update(date_str: Option<&str>, config: &mut YamlConfig) {
    let report_path = match get_report_path(config) {
        Some(p) => p,
        None => return,
    };

    let config_path = get_settings_json_path(&report_path);

    let week_num = config
        .get_property(section::REPORT, config_key::WEEK_NUM)
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(1);

    let last_day_str = date_str
        .map(|s| s.to_string())
        .or_else(|| config.get_property(section::REPORT, config_key::LAST_DAY).cloned())
        .unwrap_or_default();

    match parse_date(&last_day_str) {
        Some(last_day) => {
            let next_last_day = last_day + chrono::Duration::days(7);
            update_config_files(week_num + 1, &next_last_day, &config_path, config);
        }
        None => {
            error!("❌ 更新周数失败，请检查日期字符串是否有误: {}", last_day_str);
        }
    }
}

/// 处理 reportctl sync 命令：同步周数和日期
fn handle_sync(date_str: Option<&str>, config: &mut YamlConfig) {
    let report_path = match get_report_path(config) {
        Some(p) => p,
        None => return,
    };

    let config_path = get_settings_json_path(&report_path);

    load_config_from_json_and_sync(&config_path, config);

    let week_num = config
        .get_property(section::REPORT, config_key::WEEK_NUM)
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(1);

    let last_day_str = date_str
        .map(|s| s.to_string())
        .or_else(|| config.get_property(section::REPORT, config_key::LAST_DAY).cloned())
        .unwrap_or_default();

    match parse_date(&last_day_str) {
        Some(last_day) => {
            update_config_files(week_num, &last_day, &config_path, config);
        }
        None => {
            error!("❌ 更新周数失败，请检查日期字符串是否有误: {}", last_day_str);
        }
    }
}

/// 更新配置文件（YAML + JSON）
fn update_config_files(
    week_num: i32,
    last_day: &NaiveDate,
    config_path: &Path,
    config: &mut YamlConfig,
) {
    let last_day_str = last_day.format(DATE_FORMAT).to_string();

    // 更新 YAML 配置
    config.set_property(section::REPORT, config_key::WEEK_NUM, &week_num.to_string());
    config.set_property(section::REPORT, config_key::LAST_DAY, &last_day_str);
    info!(
        "✅ 更新YAML配置文件成功：周数 = {}, 周结束日期 = {}",
        week_num, last_day_str
    );

    // 更新 JSON 配置
    if config_path.exists() {
        let json = serde_json::json!({
            "week_num": week_num,
            "last_day": last_day_str
        });
        match fs::write(config_path, json.to_string()) {
            Ok(_) => info!(
                "✅ 更新JSON配置文件成功：周数 = {}, 周结束日期 = {}",
                week_num, last_day_str
            ),
            Err(e) => error!("❌ 更新JSON配置文件时出错: {}", e),
        }
    }
}

/// 从 JSON 配置文件读取并同步到 YAML
fn load_config_from_json_and_sync(config_path: &Path, config: &mut YamlConfig) {
    if !config_path.exists() {
        error!("❌ 日报配置文件不存在：{:?}", config_path);
        return;
    }

    match fs::read_to_string(config_path) {
        Ok(content) => {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                let last_day = json
                    .get("last_day")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let week_num = json.get("week_num").and_then(|v| v.as_i64()).unwrap_or(1);

                info!(
                    "✅ 从日报配置文件中读取到：last_day = {}, week_num = {}",
                    last_day, week_num
                );

                if let Some(last_day_date) = parse_date(last_day) {
                    update_config_files(week_num as i32, &last_day_date, config_path, config);
                }
            } else {
                error!("❌ 解析日报配置文件时出错");
            }
        }
        Err(e) => error!("❌ 读取日报配置文件失败: {}", e),
    }
}

fn parse_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s, DATE_FORMAT).ok()
}

fn append_to_file(path: &Path, content: &str) {
    use std::fs::OpenOptions;
    use std::io::Write;
    match OpenOptions::new().create(true).append(true).open(path) {
        Ok(mut f) => {
            if let Err(e) = f.write_all(content.as_bytes()) {
                error!("❌ 写入文件失败: {}", e);
            }
        }
        Err(e) => error!("❌ 打开文件失败: {}", e),
    }
}

// ========== set-url 命令 ==========

/// 处理 reportctl set-url 命令：设置 git 仓库地址
fn handle_set_url(url: Option<&str>, config: &mut YamlConfig) {
    match url {
        Some(u) if !u.is_empty() => {
            let old = config.get_property(section::REPORT, config_key::GIT_REPO).cloned();
            config.set_property(section::REPORT, config_key::GIT_REPO, u);

            // 如果日报目录已有 .git，同步更新 remote origin
            if let Some(dir) = get_report_dir(config) {
                let git_dir = Path::new(&dir).join(".git");
                if git_dir.exists() {
                    sync_git_remote(config);
                }
            }

            match old {
                Some(old_url) if !old_url.is_empty() => {
                    info!("✅ git 仓库地址已更新: {} → {}", old_url, u);
                }
                _ => {
                    info!("✅ git 仓库地址已设置: {}", u);
                }
            }
        }
        _ => {
            // 无参数时显示当前配置
            match config.get_property(section::REPORT, config_key::GIT_REPO) {
                Some(url) if !url.is_empty() => {
                    info!("📦 当前 git 仓库地址: {}", url);
                }
                _ => {
                    info!("📦 尚未配置 git 仓库地址");
                    usage!("reportctl set-url <repo_url>");
                }
            }
        }
    }
}

// ========== push / pull 命令 ==========

/// 获取日报目录（report 文件所在的目录）
fn get_report_dir(config: &YamlConfig) -> Option<String> {
    let report_path = config.report_file_path();
    report_path.parent().map(|p| p.to_string_lossy().to_string())
}

/// 在日报目录下执行 git 命令
fn run_git_in_report_dir(args: &[&str], config: &YamlConfig) -> Option<std::process::ExitStatus> {
    let dir = match get_report_dir(config) {
        Some(d) => d,
        None => {
            error!("❌ 无法确定日报目录");
            return None;
        }
    };

    let result = Command::new("git")
        .args(args)
        .current_dir(&dir)
        .status();

    match result {
        Ok(status) => Some(status),
        Err(e) => {
            error!("💥 执行 git 命令失败: {}", e);
            None
        }
    }
}

/// 检查日报目录是否已初始化 git 仓库，如果没有则初始化并配置 remote
fn ensure_git_repo(config: &YamlConfig) -> bool {
    let dir = match get_report_dir(config) {
        Some(d) => d,
        None => {
            error!("❌ 无法确定日报目录");
            return false;
        }
    };

    let git_dir = Path::new(&dir).join(".git");
    if git_dir.exists() {
        // 已初始化，同步 remote URL（防止 set-url 后 remote 不一致）
        sync_git_remote(config);
        return true;
    }

    // 检查是否有配置 git_repo
    let git_repo = config.get_property(section::REPORT, config_key::GIT_REPO);
    if git_repo.is_none() || git_repo.unwrap().is_empty() {
        error!("❌ 尚未配置 git 仓库地址，请先执行: j reportctl set-url <repo_url>");
        return false;
    }
    let repo_url = git_repo.unwrap().clone();

    info!("📦 日报目录尚未初始化 git 仓库，正在初始化...");

    // git init -b main
    if let Some(status) = run_git_in_report_dir(&["init", "-b", "main"], config) {
        if !status.success() {
            error!("❌ git init 失败");
            return false;
        }
    } else {
        return false;
    }

    // git remote add origin <repo_url>
    if let Some(status) = run_git_in_report_dir(&["remote", "add", "origin", &repo_url], config) {
        if !status.success() {
            error!("❌ git remote add 失败");
            return false;
        }
    } else {
        return false;
    }

    info!("✅ git 仓库初始化完成，remote: {}", repo_url);
    true
}

/// 同步 git remote origin URL 与配置文件中的 git_repo 保持一致
fn sync_git_remote(config: &YamlConfig) {
    let git_repo = match config.get_property(section::REPORT, config_key::GIT_REPO) {
        Some(url) if !url.is_empty() => url.clone(),
        _ => return, // 没有配置就不同步
    };

    // 获取当前 remote origin url
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
                // URL 不一致，更新 remote
                let _ = run_git_in_report_dir(&["remote", "set-url", "origin", &git_repo], config);
                info!("🔄 已同步 remote origin: {} → {}", url, git_repo);
            }
        }
        _ => {
            // 没有 origin remote，添加一个
            let _ = run_git_in_report_dir(&["remote", "add", "origin", &git_repo], config);
        }
    }
}

/// 处理 reportctl push 命令：推送周报到远程仓库
fn handle_push(commit_msg: Option<&str>, config: &YamlConfig) {
    // 检查 git_repo 配置
    let git_repo = config.get_property(section::REPORT, config_key::GIT_REPO);
    if git_repo.is_none() || git_repo.unwrap().is_empty() {
        error!("❌ 尚未配置 git 仓库地址，请先执行: j reportctl set-url <repo_url>");
        return;
    }

    // 确保 git 仓库已初始化
    if !ensure_git_repo(config) {
        return;
    }

    let default_msg = format!("update report {}", Local::now().format("%Y-%m-%d %H:%M"));
    let msg = commit_msg.unwrap_or(&default_msg);

    info!("📤 正在推送周报到远程仓库...");

    // git add .
    if let Some(status) = run_git_in_report_dir(&["add", "."], config) {
        if !status.success() {
            error!("❌ git add 失败");
            return;
        }
    } else {
        return;
    }

    // git commit -m "<msg>"
    if let Some(status) = run_git_in_report_dir(&["commit", "-m", msg], config) {
        if !status.success() {
            // commit 可能因为没有变更而失败，这不一定是错误
            info!("ℹ️ git commit 返回非零退出码（可能没有新变更）");
        }
    } else {
        return;
    }

    // git push origin main
    if let Some(status) = run_git_in_report_dir(&["push", "-u", "origin", "main"], config) {
        if status.success() {
            info!("✅ 周报已成功推送到远程仓库");
        } else {
            error!("❌ git push 失败，请检查网络连接和仓库权限");
        }
    }
}

/// 处理 reportctl pull 命令：从远程仓库拉取周报
fn handle_pull(config: &YamlConfig) {
    // 检查 git_repo 配置
    let git_repo = config.get_property(section::REPORT, config_key::GIT_REPO);
    if git_repo.is_none() || git_repo.unwrap().is_empty() {
        error!("❌ 尚未配置 git 仓库地址，请先执行: j reportctl set-url <repo_url>");
        return;
    }

    let dir = match get_report_dir(config) {
        Some(d) => d,
        None => {
            error!("❌ 无法确定日报目录");
            return;
        }
    };

    let git_dir = Path::new(&dir).join(".git");

    if !git_dir.exists() {
        // 日报目录不是 git 仓库，尝试 clone
        let repo_url = git_repo.unwrap().clone();
        info!("📥 日报目录尚未初始化，正在从远程仓库克隆...");

        // 先备份已有文件（如果有的话）
        let report_path = config.report_file_path();
        let has_existing = report_path.exists() && fs::metadata(&report_path).map(|m| m.len() > 0).unwrap_or(false);

        if has_existing {
            // 备份现有文件
            let backup_path = report_path.with_extension("md.bak");
            if let Err(e) = fs::copy(&report_path, &backup_path) {
                error!("⚠️ 备份现有日报文件失败: {}", e);
            } else {
                info!("📋 已备份现有日报到: {:?}", backup_path);
            }
        }

        // 清空目录内容后 clone
        // 使用 git clone 到一个临时目录再移动
        let temp_dir = Path::new(&dir).with_file_name(".report_clone_tmp");
        let _ = fs::remove_dir_all(&temp_dir);

        let result = Command::new("git")
            .args(["clone", "-b", "main", &repo_url, &temp_dir.to_string_lossy()])
            .status();

        match result {
            Ok(status) if status.success() => {
                // 将 clone 出来的内容移到 report 目录
                let _ = fs::remove_dir_all(&dir);
                if let Err(e) = fs::rename(&temp_dir, &dir) {
                    error!("❌ 移动克隆仓库失败: {}，临时目录: {:?}", e, temp_dir);
                    return;
                }
                info!("✅ 成功从远程仓库克隆周报");
            }
            Ok(_) => {
                error!("❌ git clone 失败，请检查仓库地址和网络连接");
                let _ = fs::remove_dir_all(&temp_dir);
            }
            Err(e) => {
                error!("💥 执行 git clone 失败: {}", e);
                let _ = fs::remove_dir_all(&temp_dir);
            }
        }
    } else {
        // 已经是 git 仓库，先同步 remote URL
        sync_git_remote(config);

        // 检测是否是空仓库（unborn branch，没有任何 commit）
        let has_commits = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&dir)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if !has_commits {
            // 空仓库（git init 后未 commit），通过 fetch + checkout 来拉取
            info!("📥 本地仓库尚无提交，正在从远程仓库拉取...");

            // 备份本地已有的未跟踪文件
            let report_path = config.report_file_path();
            if report_path.exists() && fs::metadata(&report_path).map(|m| m.len() > 0).unwrap_or(false) {
                let backup_path = report_path.with_extension("md.bak");
                let _ = fs::copy(&report_path, &backup_path);
                info!("📋 已备份本地日报到: {:?}", backup_path);
            }

            // git fetch origin main
            if let Some(status) = run_git_in_report_dir(&["fetch", "origin", "main"], config) {
                if !status.success() {
                    error!("❌ git fetch 失败，请检查网络连接和仓库地址");
                    return;
                }
            } else {
                return;
            }

            // git reset --hard origin/main（强制用远程覆盖本地）
            if let Some(status) = run_git_in_report_dir(&["reset", "--hard", "origin/main"], config) {
                if status.success() {
                    info!("✅ 成功从远程仓库拉取周报");
                } else {
                    error!("❌ git reset 失败");
                }
            }
        } else {
            // 正常仓库，先 stash 再 pull
            info!("📥 正在从远程仓库拉取最新周报...");

            // 先暂存本地未跟踪/修改的文件，防止 pull 时冲突
            let _ = run_git_in_report_dir(&["add", "-A"], config);
            let stash_result = Command::new("git")
                .args(["stash", "push", "-m", "auto-stash-before-pull"])
                .current_dir(&dir)
                .output();
            let has_stash = match &stash_result {
                Ok(output) => {
                    let msg = String::from_utf8_lossy(&output.stdout);
                    !msg.contains("No local changes")
                }
                Err(_) => false,
            };

            // 执行 pull
            let pull_ok = if let Some(status) = run_git_in_report_dir(&["pull", "origin", "main", "--rebase"], config) {
                if status.success() {
                    info!("✅ 周报已更新到最新版本");
                    true
                } else {
                    error!("❌ git pull 失败，请检查网络连接或手动解决冲突");
                    false
                }
            } else {
                false
            };

            // 恢复 stash
            if has_stash {
                if let Some(status) = run_git_in_report_dir(&["stash", "pop"], config) {
                    if !status.success() && pull_ok {
                        info!("⚠️ stash pop 存在冲突，请手动合并本地修改（已保存在 git stash 中）");
                    }
                }
            }
        }
    }
}

// ========== check 命令 ==========

/// 处理 check 命令: j check [line_count]
pub fn handle_check(line_count: Option<&str>, config: &YamlConfig) {
    let num = match line_count {
        Some(s) => match s.parse::<usize>() {
            Ok(n) if n > 0 => n,
            _ => {
                error!("❌ 无效的行数参数: {}，请输入正整数", s);
                return;
            }
        },
        None => DEFAULT_CHECK_LINES,
    };

    let report_path = match get_report_path(config) {
        Some(p) => p,
        None => return,
    };

    info!("📂 正在读取周报文件路径: {}", report_path);

    let path = Path::new(&report_path);
    if !path.is_file() {
        error!("❌ 文件不存在或不是有效文件: {}", report_path);
        return;
    }

    let lines = read_last_n_lines(path, num);
    info!("📄 最近的 {} 行内容如下：", lines.len());
    for line in &lines {
        info!("{}", line);
    }
}

// ========== search 命令 ==========

/// 处理 search 命令: j search <line_count|all> <target> [-f|-fuzzy]
pub fn handle_search(line_count: &str, target: &str, fuzzy_flag: Option<&str>, config: &YamlConfig) {
    let num = if line_count == "all" {
        usize::MAX
    } else {
        match line_count.parse::<usize>() {
            Ok(n) if n > 0 => n,
            _ => {
                error!("❌ 无效的行数参数: {}，请输入正整数或 all", line_count);
                return;
            }
        }
    };

    let report_path = match get_report_path(config) {
        Some(p) => p,
        None => return,
    };

    info!("📂 正在读取周报文件路径: {}", report_path);

    let path = Path::new(&report_path);
    if !path.is_file() {
        error!("❌ 文件不存在或不是有效文件: {}", report_path);
        return;
    }

    let is_fuzzy = matches!(fuzzy_flag, Some(f) if f == search_flag::FUZZY_SHORT || f == search_flag::FUZZY);
    if is_fuzzy {
        info!("启用模糊匹配...");
    }

    let lines = read_last_n_lines(path, num);
    info!("🔍 搜索目标关键字: {}", target.green());

    let mut index = 0;
    for line in &lines {
        let matched = if is_fuzzy {
            fuzzy::fuzzy_match(line, target)
        } else {
            line.contains(target)
        };

        if matched {
            index += 1;
            let highlighted = fuzzy::highlight_matches(line, target, is_fuzzy);
            info!("[{}] {}", index, highlighted);
        }
    }

    if index == 0 {
        info!("nothing found 😢");
    }
}

/// 从文件尾部读取最后 N 行（高效实现，不需要读取整个文件）
fn read_last_n_lines(path: &Path, n: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let buffer_size: usize = 16384; // 16KB

    let mut file = match fs::File::open(path) {
        Ok(f) => f,
        Err(e) => {
            error!("❌ 读取文件时发生错误: {}", e);
            return lines;
        }
    };

    let file_len = match file.metadata() {
        Ok(m) => m.len() as usize,
        Err(_) => return lines,
    };

    if file_len == 0 {
        return lines;
    }

    // 对于较小的文件或者需要读取全部内容的情况，直接全部读取
    if n == usize::MAX || file_len <= buffer_size * 2 {
        let mut content = String::new();
        let _ = file.seek(SeekFrom::Start(0));
        if file.read_to_string(&mut content).is_ok() {
            let all_lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
            if n >= all_lines.len() {
                return all_lines;
            }
            return all_lines[all_lines.len() - n..].to_vec();
        }
        return lines;
    }

    // 从文件尾部逐块读取
    let mut pointer = file_len;
    let mut remainder = Vec::new();

    while pointer > 0 && lines.len() < n {
        let bytes_to_read = pointer.min(buffer_size);
        pointer -= bytes_to_read;

        let _ = file.seek(SeekFrom::Start(pointer as u64));
        let mut buffer = vec![0u8; bytes_to_read];
        if file.read_exact(&mut buffer).is_err() {
            break;
        }

        // 将 remainder（上次剩余的不完整行）追加到这个块的末尾
        buffer.extend(remainder.drain(..));

        // 从后向前按行分割
        let text = String::from_utf8_lossy(&buffer).to_string();
        let mut block_lines: Vec<&str> = text.split('\n').collect();

        // 第一行可能是不完整的（跨块）
        if pointer > 0 {
            remainder = block_lines.remove(0).as_bytes().to_vec();
        }

        for line in block_lines.into_iter().rev() {
            if !line.is_empty() {
                lines.push(line.to_string());
                if lines.len() >= n {
                    break;
                }
            }
        }
    }

    // 处理文件最开头的那行
    if !remainder.is_empty() && lines.len() < n {
        let line = String::from_utf8_lossy(&remainder).to_string();
        if !line.is_empty() {
            lines.push(line);
        }
    }

    lines.reverse();
    lines
}
