use crate::command::chat::app::ChatApp;

// ========== 斜杠命令定义 ==========

/// / 斜杠命令类型
#[derive(Clone, Debug, PartialEq)]
pub enum SlashCommand {
    /// 复制最后一条 AI 回复
    Copy,
    /// 打开日志窗口
    Log,
    /// 浏览消息
    Browse,
    /// 打开配置界面
    Config,
    /// 切换模型
    Model,
    /// 归档对话
    Archive,
    /// 清空当前对话
    Clear,
    /// 切换主题
    Theme,
    /// 恢复历史会话
    Resume,
    /// 导出原始 session messages（未经 window/compact/hooks/sanitize 处理）
    Dump,
    /// 导出经完整处理管线（window → micro_compact → hooks → sanitize）后的最终请求数据
    DumpProcessed,
    /// Teammate 面板
    Teammate,
}

impl SlashCommand {
    /// 返回显示标签（如 "/copy"）
    pub fn display_label(&self) -> String {
        match self {
            SlashCommand::Copy => "/copy".to_string(),
            SlashCommand::Log => "/log".to_string(),
            SlashCommand::Browse => "/browse".to_string(),
            SlashCommand::Config => "/config".to_string(),
            SlashCommand::Model => "/model".to_string(),
            SlashCommand::Archive => "/archive".to_string(),
            SlashCommand::Clear => "/clear".to_string(),
            SlashCommand::Theme => "/theme".to_string(),
            SlashCommand::Resume => "/resume".to_string(),
            SlashCommand::Dump => "/dump".to_string(),
            SlashCommand::DumpProcessed => "/dump-processed".to_string(),
            SlashCommand::Teammate => "/teammate".to_string(),
        }
    }

    /// 返回命令描述
    pub fn description(&self) -> String {
        match self {
            SlashCommand::Copy => "复制最后一条 AI 回复".to_string(),
            SlashCommand::Log => "打开日志窗口".to_string(),
            SlashCommand::Browse => "浏览历史消息".to_string(),
            SlashCommand::Config => "打开配置界面".to_string(),
            SlashCommand::Model => "切换模型".to_string(),
            SlashCommand::Archive => "归档当前对话".to_string(),
            SlashCommand::Clear => "新建对话".to_string(),
            SlashCommand::Theme => "切换主题".to_string(),
            SlashCommand::Resume => "恢复历史会话".to_string(),
            SlashCommand::Dump => "导出原始 session messages（未经处理管线）".to_string(),
            SlashCommand::DumpProcessed => {
                "导出经完整处理管线（window → compact → hooks → sanitize）后的最终数据".to_string()
            }
            SlashCommand::Teammate => "Teammate 面板".to_string(),
        }
    }

    /// 返回所有可用命令
    pub fn all() -> Vec<SlashCommand> {
        vec![
            SlashCommand::Copy,
            SlashCommand::Log,
            SlashCommand::Browse,
            SlashCommand::Config,
            SlashCommand::Model,
            SlashCommand::Archive,
            SlashCommand::Clear,
            SlashCommand::Theme,
            SlashCommand::Resume,
            SlashCommand::Dump,
            SlashCommand::DumpProcessed,
            SlashCommand::Teammate,
        ]
    }
}

/// 根据过滤文本返回匹配的斜杠命令列表
pub fn get_filtered_slash_commands(filter: &str) -> Vec<SlashCommand> {
    let filter_lower = filter.to_lowercase();
    SlashCommand::all()
        .into_iter()
        .filter(|cmd| {
            if filter_lower.is_empty() {
                return true;
            }
            cmd.display_label().to_lowercase().contains(&filter_lower)
        })
        .collect()
}

// ========== @ 弹窗定义 ==========

/// @ 弹窗中的混合搜索结果类型
#[derive(Clone, Debug)]
pub enum AtPopupItem {
    /// 分类入口: "skill:", "command:", "file:"
    Category(String),
    /// 匹配到的技能名
    Skill(String),
    /// 匹配到的命令名
    Command(String),
    /// 匹配到的文件路径
    File(String),
}

impl AtPopupItem {}

/// 根据当前输入光标位置更新 @ 弹窗的过滤关键字。
pub fn update_at_filter(app: &mut ChatApp) {
    let chars: Vec<char> = app.ui.input_text().chars().collect();
    let cursor_pos = app.ui.cursor_char_idx();
    let start = app.ui.at_popup_start_pos + 1; // @ 之后
    if start <= cursor_pos && cursor_pos <= chars.len() {
        app.ui.at_popup_filter = chars[start..cursor_pos].iter().collect();
    } else {
        app.ui.at_popup_filter.clear();
    }
    // 重置选中索引
    app.ui.at_popup_selected = 0;
}

/// 混合搜索：当 filter 非空时，同时在 skill/command/file 三个来源中搜索
pub fn get_filtered_all_items(app: &ChatApp) -> Vec<AtPopupItem> {
    let raw_filter = app.ui.at_popup_filter.as_str();

    // filter 为空时只返回三个分类入口
    if raw_filter.is_empty() {
        return vec![
            AtPopupItem::Category("skill:".to_string()),
            AtPopupItem::Category("command:".to_string()),
            AtPopupItem::Category("file:".to_string()),
        ];
    }

    let mut items: Vec<AtPopupItem> = Vec::new();
    let filter = raw_filter.to_lowercase();

    // 1. 匹配的分类入口
    for label in &["skill:", "command:", "file:"] {
        if label.contains(&filter) {
            items.push(AtPopupItem::Category(label.to_string()));
        }
    }

    // 2. 匹配的技能名（最多 5 个）
    let skills: Vec<String> = app
        .state
        .loaded_skills
        .iter()
        .filter(|s| {
            !app.state
                .agent_config
                .disabled_skills
                .iter()
                .any(|d| d == &s.frontmatter.name)
        })
        .map(|s| s.frontmatter.name.clone())
        .filter(|name| name.to_lowercase().contains(&filter))
        .take(5)
        .collect();
    for s in skills {
        items.push(AtPopupItem::Skill(s));
    }

    // 3. 匹配的命令名（最多 5 个）
    let commands: Vec<String> = app
        .state
        .loaded_commands
        .iter()
        .filter(|c| {
            !app.state
                .agent_config
                .disabled_commands
                .iter()
                .any(|d| d == &c.frontmatter.name)
        })
        .map(|c| c.frontmatter.name.clone())
        .filter(|name| name.to_lowercase().contains(&filter))
        .take(5)
        .collect();
    for c in commands {
        items.push(AtPopupItem::Command(c));
    }

    // 4. 匹配的文件（增强版：支持路径导航、~ 展开、优化评分）
    // 使用 file_index 缓存进行内存过滤，避免每帧 WalkBuilder 扫描
    let file_items = get_filtered_files_for_at(app, raw_filter);
    for path in file_items {
        items.push(AtPopupItem::File(path));
    }

    items.truncate(20);
    items
}

/// 为 @ 弹窗获取文件列表（使用 FileIndex 缓存）
fn get_filtered_files_for_at(app: &ChatApp, raw_filter: &str) -> Vec<String> {
    // 索引未就绪时返回空（后台线程仍在扫描）
    if !app.file_index.is_ready() {
        return Vec::new();
    }

    // 处理 ~ 路径展开：将 ~ 替换为 home 目录后进行目录浏览
    let is_tilde = raw_filter == "~" || raw_filter.starts_with("~/");
    let expanded_filter;
    let filter: &str = if is_tilde {
        if let Some(home) = dirs::home_dir() {
            let home_str = home.to_string_lossy().to_string();
            expanded_filter = if raw_filter == "~" {
                format!("{}/", home_str)
            } else {
                format!("{}{}", home_str, &raw_filter[1..])
            };
            &expanded_filter
        } else {
            return Vec::new();
        }
    } else {
        raw_filter
    };

    let filter_lower = filter.to_lowercase();

    // 如果 filter 包含 /，先尝试精确路径补全（逐层浏览模式）
    // 这里仍然用 read_dir 做实时读取（保证目录浏览的准确性）
    if let Some(last_slash) = filter.rfind('/') {
        let dir_part = &filter[..=last_slash];
        let prefix = &filter[last_slash + 1..];
        let dir_path = if dir_part.is_empty() {
            std::path::PathBuf::from(".")
        } else {
            std::path::PathBuf::from(dir_part)
        };

        if dir_path.is_dir() {
            let prefix_lower = prefix.to_lowercase();
            let mut entries: Vec<String> = Vec::new();
            if let Ok(read_dir) = std::fs::read_dir(&dir_path) {
                for entry in read_dir.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.starts_with('.') && !prefix.starts_with('.') {
                        continue;
                    }
                    if !prefix_lower.is_empty() && !name.to_lowercase().starts_with(&prefix_lower) {
                        continue;
                    }
                    let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                    if is_dir {
                        entries.push(format!("{}{}/", dir_part, name));
                    } else {
                        entries.push(format!("{}{}", dir_part, name));
                    }
                }
            }

            entries.sort_by(|a, b| {
                let a_dir = a.ends_with('/');
                let b_dir = b.ends_with('/');
                match (a_dir, b_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.to_lowercase().cmp(&b.to_lowercase()),
                }
            });
            entries.truncate(10);
            return entries;
        }
        // 目录不存在时，fallback 到用最后一个路径段做模糊搜索
    }

    // 提取搜索关键词
    let search_filter = if filter_lower.ends_with('/') {
        let trimmed = &filter_lower[..filter_lower.len() - 1];
        if let Some(last_slash) = trimmed.rfind('/') {
            &trimmed[last_slash + 1..]
        } else {
            trimmed
        }
    } else if let Some(last_slash) = filter_lower.rfind('/') {
        &filter_lower[last_slash + 1..]
    } else {
        &filter_lower
    };

    if search_filter.is_empty() {
        return Vec::new();
    }

    // 使用 FileIndex 缓存进行内存模糊搜索（替代 WalkBuilder 全目录扫描）
    app.file_index.fuzzy_search(search_filter, 10)
}

/// 在 @ 弹窗中直接选中一个混合搜索结果时，替换输入框内容
pub fn complete_at_direct(app: &mut ChatApp, item: &AtPopupItem) {
    let mention = match item {
        AtPopupItem::Skill(name) => format!("@skill:{} ", name),
        AtPopupItem::Command(name) => format!("@command:{} ", name),
        AtPopupItem::File(path) => format!("@file:{} ", path),
        AtPopupItem::Category(_) => return, // 分类不在此处处理
    };
    let chars: Vec<char> = app.ui.input_text().chars().collect();
    let cursor_pos = app.ui.cursor_char_idx();
    let before: String = chars[..app.ui.at_popup_start_pos].iter().collect();
    let after: String = if cursor_pos < chars.len() {
        chars[cursor_pos..].iter().collect()
    } else {
        String::new()
    };
    let new_cursor = before.chars().count() + mention.chars().count();
    app.ui
        .set_input_text(&format!("{}{}{}", before, mention, after), new_cursor);
}

/// 更新技能补全弹窗的过滤文本
pub fn update_skill_filter(app: &mut ChatApp) {
    let chars: Vec<char> = app.ui.input_text().chars().collect();
    let cursor_pos = app.ui.cursor_char_idx();
    // @skill: 占 7 个字符, 过滤文本从 start_pos + 7 开始
    let start = app.ui.skill_popup_start_pos + 7;
    if start <= cursor_pos && cursor_pos <= chars.len() {
        app.ui.skill_popup_filter = chars[start..cursor_pos].iter().collect();
    } else {
        app.ui.skill_popup_filter.clear();
    }
    app.ui.skill_popup_selected = 0;
}

/// 根据 skill_popup_filter 过滤技能名称列表
pub fn get_filtered_skill_names(app: &ChatApp) -> Vec<String> {
    let filter = app.ui.skill_popup_filter.to_lowercase();
    app.state
        .loaded_skills
        .iter()
        .filter(|s| {
            !app.state
                .agent_config
                .disabled_skills
                .iter()
                .any(|d| d == &s.frontmatter.name)
        })
        .map(|s| s.frontmatter.name.clone())
        .filter(|name| filter.is_empty() || name.to_lowercase().contains(&filter))
        .collect()
}

/// 替换 input 中 @skill:filter 为 @skill:完整名称 + 空格
pub fn complete_skill_mention(app: &mut ChatApp, skill_name: &str) {
    let chars: Vec<char> = app.ui.input_text().chars().collect();
    let cursor_pos = app.ui.cursor_char_idx();
    let before: String = chars[..app.ui.skill_popup_start_pos].iter().collect();
    let after: String = if cursor_pos < chars.len() {
        chars[cursor_pos..].iter().collect()
    } else {
        String::new()
    };
    let replacement = format!("@skill:{} ", skill_name);
    let new_cursor = before.chars().count() + replacement.chars().count();
    app.ui
        .set_input_text(&format!("{}{}{}", before, replacement, after), new_cursor);
}

/// 更新文件补全弹窗的过滤文本
pub fn update_file_filter(app: &mut ChatApp) {
    let chars: Vec<char> = app.ui.input_text().chars().collect();
    let cursor_pos = app.ui.cursor_char_idx();
    // @file: 占 6 个字符 (@file:), 过滤文本从 start_pos + 6 开始
    let start = app.ui.file_popup_start_pos + 6;
    if start <= cursor_pos && cursor_pos <= chars.len() {
        app.ui.file_popup_filter = chars[start..cursor_pos].iter().collect();
    } else {
        app.ui.file_popup_filter.clear();
    }
    app.ui.file_popup_selected = 0;
}

/// 获取文件补全列表（使用 FileIndex 缓存进行内存过滤，支持模糊匹配）
pub fn get_filtered_files(app: &ChatApp) -> Vec<String> {
    // 索引未就绪时返回空（后台线程仍在扫描）
    if !app.file_index.is_ready() {
        return Vec::new();
    }

    let raw_filter = &app.ui.file_popup_filter;

    // 处理 ~ 路径展开：将 ~ 替换为 home 目录后进行目录浏览
    let is_tilde = raw_filter == "~" || raw_filter.starts_with("~/");
    let expanded_filter;
    let filter: &str = if is_tilde {
        if let Some(home) = dirs::home_dir() {
            let home_str = home.to_string_lossy().to_string();
            expanded_filter = if raw_filter == "~" {
                format!("{}/", home_str)
            } else {
                format!("{}{}", home_str, &raw_filter[1..])
            };
            &expanded_filter
        } else {
            return Vec::new();
        }
    } else {
        raw_filter
    };

    // 如果 filter 包含 /，先尝试精确路径补全（逐层浏览模式）
    // 这里仍然用 read_dir 做实时读取（保证目录浏览的准确性）
    if let Some(last_slash) = filter.rfind('/') {
        let dir_part = &filter[..=last_slash];
        let prefix = &filter[last_slash + 1..];
        let dir_path = if dir_part.is_empty() {
            std::path::PathBuf::from(".")
        } else {
            std::path::PathBuf::from(dir_part)
        };

        if dir_path.is_dir() {
            let prefix_lower = prefix.to_lowercase();
            let mut entries: Vec<String> = Vec::new();
            if let Ok(read_dir) = std::fs::read_dir(&dir_path) {
                for entry in read_dir.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.starts_with('.') && !prefix.starts_with('.') {
                        continue;
                    }
                    if !prefix_lower.is_empty() && !name.to_lowercase().starts_with(&prefix_lower) {
                        continue;
                    }
                    let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                    if is_dir {
                        entries.push(format!("{}{}/", dir_part, name));
                    } else {
                        entries.push(format!("{}{}", dir_part, name));
                    }
                }
            }

            entries.sort_by(|a, b| {
                let a_dir = a.ends_with('/');
                let b_dir = b.ends_with('/');
                match (a_dir, b_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.to_lowercase().cmp(&b.to_lowercase()),
                }
            });
            entries.truncate(15);
            return entries;
        }
    }

    // 无 / 时使用 FileIndex 缓存进行内存模糊搜索
    app.file_index.fuzzy_search(filter, 15)
}

/// 更新命令补全弹窗的过滤文本
pub fn update_command_filter(app: &mut ChatApp) {
    let chars: Vec<char> = app.ui.input_text().chars().collect();
    let cursor_pos = app.ui.cursor_char_idx();
    // @command: 占 9 个字符, 过滤文本从 start_pos + 9 开始
    let start = app.ui.command_popup_start_pos + 9;
    if start <= cursor_pos && cursor_pos <= chars.len() {
        app.ui.command_popup_filter = chars[start..cursor_pos].iter().collect();
    } else {
        app.ui.command_popup_filter.clear();
    }
    app.ui.command_popup_selected = 0;
}

/// 根据 command_popup_filter 过滤命令名称列表
pub fn get_filtered_command_names(app: &ChatApp) -> Vec<String> {
    let filter = app.ui.command_popup_filter.to_lowercase();
    app.state
        .loaded_commands
        .iter()
        .filter(|c| {
            !app.state
                .agent_config
                .disabled_commands
                .iter()
                .any(|d| d == &c.frontmatter.name)
        })
        .map(|c| c.frontmatter.name.clone())
        .filter(|name| filter.is_empty() || name.to_lowercase().contains(&filter))
        .collect()
}

/// 替换 input 中 @command:filter 为 @command:完整名称 + 空格
pub fn complete_command_mention(app: &mut ChatApp, command_name: &str) {
    let chars: Vec<char> = app.ui.input_text().chars().collect();
    let cursor_pos = app.ui.cursor_char_idx();
    let before: String = chars[..app.ui.command_popup_start_pos].iter().collect();
    let after: String = if cursor_pos < chars.len() {
        chars[cursor_pos..].iter().collect()
    } else {
        String::new()
    };
    let replacement = format!("@command:{} ", command_name);
    let new_cursor = before.chars().count() + replacement.chars().count();
    app.ui
        .set_input_text(&format!("{}{}{}", before, replacement, after), new_cursor);
}

/// 替换 input 中 @file:filter 为 @file:完整路径 + 空格
pub fn complete_file_mention(app: &mut ChatApp, file_path: &str) {
    let chars: Vec<char> = app.ui.input_text().chars().collect();
    let cursor_pos = app.ui.cursor_char_idx();
    let before: String = chars[..app.ui.file_popup_start_pos].iter().collect();
    let after: String = if cursor_pos < chars.len() {
        chars[cursor_pos..].iter().collect()
    } else {
        String::new()
    };
    let replacement = format!("@file:{} ", file_path);
    let new_cursor = before.chars().count() + replacement.chars().count();
    app.ui
        .set_input_text(&format!("{}{}{}", before, replacement, after), new_cursor);
}
