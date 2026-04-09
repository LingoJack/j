//! 本地笔记管理
//!
//! 笔记存储于 ~/.jdata/notebook/ 目录下，每个笔记为独立的 `.md` 文件。
//! 使用 Markdown 编辑器进行编辑。

use crate::command::chat::theme::{Theme, ThemeName};
use crate::config::YamlConfig;
use crate::constants::{notebook_action, shell};
use crate::util::fuzzy;
use crate::{error, info};
use chrono::{DateTime, Local};
use colored::Colorize;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

/// notebook 命令入口
pub fn handle_notebook(args: &[String], _config: &YamlConfig) {
    if args.is_empty() {
        handle_select();
        return;
    }

    let first = args[0].as_str();
    match first {
        f if f == notebook_action::LIST => handle_list(),
        f if f == notebook_action::SEARCH => {
            if let Some(keyword) = args.get(1) {
                handle_search(keyword);
            } else {
                error!("用法: notebook search <关键词>");
            }
        }
        f if f == notebook_action::DELETE => {
            if let Some(title) = args.get(1) {
                handle_delete(title);
            } else {
                error!("用法: notebook delete <笔记名>");
            }
        }
        f if f == notebook_action::OPEN => handle_open(),
        f if f == notebook_action::RENAME => {
            if args.len() >= 3 {
                handle_rename(&args[1], &args[2]);
            } else {
                error!("用法: notebook rename <旧名称> <新名称>");
            }
        }
        _ => {
            // 其余参数视为笔记标题
            let title = args.join(" ");
            handle_edit(&title);
        }
    }
}

// ========== 内部函数 ==========

/// 获取 notebook 目录路径（~/.jdata/notebook/，自动创建）
fn get_notebook_dir() -> std::path::PathBuf {
    YamlConfig::notebook_dir()
}

/// 列出 notebook 目录下所有 .md 文件，按修改时间倒序
fn list_notes(dir: &Path) -> Vec<(String, std::time::SystemTime)> {
    let mut notes = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "md") {
                let name = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let mtime = entry
                    .metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .unwrap_or(std::time::UNIX_EPOCH);
                notes.push((name, mtime));
            }
        }
    }
    // 按修改时间倒序
    notes.sort_by(|a, b| b.1.cmp(&a.1));
    notes
}

/// 格式化 SystemTime 为可读字符串
fn format_time(time: std::time::SystemTime) -> String {
    let dt: DateTime<Local> = time.into();
    dt.format("%Y-%m-%d %H:%M").to_string()
}

/// 无参数时：列出笔记并交互选择
fn handle_select() {
    let dir = get_notebook_dir();
    let notes = list_notes(&dir);
    if notes.is_empty() {
        info!("📓 notebook 为空，使用 `j nb <标题>` 创建第一篇笔记");
        return;
    }

    println!("{}", "📓 笔记列表：".bold());
    for (i, (name, mtime)) in notes.iter().enumerate() {
        println!(
            "  {} {}  {}",
            format!("[{}]", i + 1).cyan(),
            name,
            format_time(*mtime).dimmed()
        );
    }
    println!();
    print!("{}", "输入编号或名称（模糊匹配）: ".dimmed());
    let _ = io::stdout().flush();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return;
    }
    let input = input.trim();
    if input.is_empty() {
        return;
    }

    // 尝试按编号解析
    if let Ok(num) = input.parse::<usize>()
        && num >= 1
        && num <= notes.len()
    {
        handle_edit(&notes[num - 1].0);
        return;
    }

    // 模糊匹配
    let matched: Vec<&str> = notes
        .iter()
        .map(|(name, _)| name.as_str())
        .filter(|name| fuzzy::fuzzy_match(name, input))
        .collect();

    match matched.len() {
        0 => {
            error!("未找到匹配的笔记: {}", input);
        }
        1 => {
            handle_edit(matched[0]);
        }
        _ => {
            println!("{}", "匹配到多个笔记：".yellow());
            for name in &matched {
                println!("  - {}", name);
            }
            info!("请使用更精确的名称");
        }
    }
}

/// 编辑/新建笔记
fn handle_edit(title: &str) {
    let dir = get_notebook_dir();
    let file_path = dir.join(format!("{}.md", title));
    let (content, is_new) = if file_path.exists() {
        match fs::read_to_string(&file_path) {
            Ok(c) => (c, false),
            Err(e) => {
                error!("读取笔记失败: {}", e);
                return;
            }
        }
    } else {
        (String::new(), true)
    };

    let editor_title = if is_new {
        format!("{} (新笔记)", title)
    } else {
        title.to_string()
    };

    let theme = Theme::from_name(&ThemeName::default());
    match crate::tui::editor_markdown::open_markdown_editor(&editor_title, &content, &theme) {
        Ok(Some(new_content)) => {
            if new_content != content {
                match fs::write(&file_path, &new_content) {
                    Ok(()) => info!("笔记已保存: {}", title),
                    Err(e) => error!("保存笔记失败: {}", e),
                }
            } else {
                info!("内容未变化，跳过保存");
            }
        }
        Ok(None) => info!("已取消编辑"),
        Err(e) => error!("编辑器启动失败: {}", e),
    }
}

/// 列出所有笔记
fn handle_list() {
    let dir = get_notebook_dir();
    let notes = list_notes(&dir);
    if notes.is_empty() {
        info!("📓 notebook 为空");
        return;
    }

    println!("{}", format!("📓 共 {} 篇笔记：", notes.len()).bold());
    for (name, mtime) in &notes {
        println!("  {}  {}", name, format_time(*mtime).dimmed());
    }
}

/// 搜索笔记内容
fn handle_search(keyword: &str) {
    let dir = get_notebook_dir();
    let notes = list_notes(&dir);
    if notes.is_empty() {
        info!("📓 notebook 为空");
        return;
    }

    let mut found = false;
    for (name, _) in &notes {
        let file_path = dir.join(format!("{}.md", name));
        if let Ok(content) = fs::read_to_string(&file_path)
            && (fuzzy::fuzzy_match(&content, keyword) || fuzzy::fuzzy_match(name, keyword))
        {
            if !found {
                println!("{}", format!("🔍 搜索 \"{}\" 的结果：", keyword).bold());
                found = true;
            }
            // 显示匹配行
            println!("\n  {}", name.cyan().bold());
            for (line_num, line) in content.lines().enumerate() {
                if fuzzy::fuzzy_match(line, keyword) {
                    println!(
                        "    {}: {}",
                        format!("L{}", line_num + 1).dimmed(),
                        line.trim()
                    );
                }
            }
        }
    }

    if !found {
        info!("未找到包含 \"{}\" 的笔记", keyword);
    }
}

/// 删除笔记
fn handle_delete(title: &str) {
    let dir = get_notebook_dir();
    let file_path = dir.join(format!("{}.md", title));
    if !file_path.exists() {
        // 尝试模糊匹配
        let notes = list_notes(&dir);
        let matched: Vec<&str> = notes
            .iter()
            .map(|(name, _)| name.as_str())
            .filter(|name| fuzzy::fuzzy_match(name, title))
            .collect();

        if matched.is_empty() {
            error!("未找到笔记: {}", title);
        } else {
            println!("未找到精确匹配，你是否要删除以下笔记？");
            for name in &matched {
                println!("  - {}", name);
            }
            info!("请使用精确名称: notebook delete <名称>");
        }
        return;
    }

    // 确认删除
    print!("确认删除笔记 \"{}\"？(y/N): ", title);
    let _ = io::stdout().flush();
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return;
    }
    if input.trim().to_lowercase() == "y" {
        match fs::remove_file(&file_path) {
            Ok(()) => info!("已删除笔记: {}", title),
            Err(e) => error!("删除失败: {}", e),
        }
    } else {
        info!("已取消删除");
    }
}

/// 打开 notebook 目录
fn handle_open() {
    let dir = get_notebook_dir();
    let path = dir.to_string_lossy().to_string();
    let os = std::env::consts::OS;
    let result = if os == shell::MACOS_OS {
        Command::new("open").arg(&path).status()
    } else if os == shell::WINDOWS_OS {
        Command::new(shell::WINDOWS_CMD)
            .args([shell::WINDOWS_CMD_FLAG, "start", "", &path])
            .status()
    } else {
        Command::new("xdg-open").arg(&path).status()
    };

    if let Err(e) = result {
        error!("打开目录失败: {}", e);
    }
}

/// 重命名笔记
fn handle_rename(old_name: &str, new_name: &str) {
    let dir = get_notebook_dir();
    let old_path = dir.join(format!("{}.md", old_name));
    let new_path = dir.join(format!("{}.md", new_name));

    if !old_path.exists() {
        error!("未找到笔记: {}", old_name);
        return;
    }
    if new_path.exists() {
        error!("目标笔记已存在: {}", new_name);
        return;
    }

    match fs::rename(&old_path, &new_path) {
        Ok(()) => info!("已重命名: {} → {}", old_name, new_name),
        Err(e) => error!("重命名失败: {}", e),
    }
}
