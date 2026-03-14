use super::app::ChatApp;

pub fn update_at_filter(app: &mut ChatApp) {
    let chars: Vec<char> = app.input.chars().collect();
    let start = app.at_popup_start_pos + 1; // @ 之后
    if start <= app.cursor_pos && app.cursor_pos <= chars.len() {
        app.at_popup_filter = chars[start..app.cursor_pos].iter().collect();
    } else {
        app.at_popup_filter.clear();
    }
    // 重置选中索引
    app.at_popup_selected = 0;
}

/// 根据 filter 过滤 loaded_skills 的 name 列表
pub fn get_filtered_skills(app: &ChatApp) -> Vec<String> {
    let filter = app.at_popup_filter.to_lowercase();
    let mut items: Vec<String> = app
        .loaded_skills
        .iter()
        .filter(|s| {
            !app.agent_config
                .disabled_skills
                .iter()
                .any(|d| d == &s.frontmatter.name)
        })
        .map(|s| s.frontmatter.name.clone())
        .filter(|name| filter.is_empty() || name.to_lowercase().contains(&filter))
        .collect();
    // 添加 file: 选项
    let file_label = "file:".to_string();
    if filter.is_empty() || file_label.contains(&filter) {
        items.push(file_label);
    }
    items
}

/// 替换 input 中 @... 为 @skill_name 并加空格
pub fn complete_at_mention(app: &mut ChatApp, skill_name: &str) {
    let chars: Vec<char> = app.input.chars().collect();
    let before: String = chars[..app.at_popup_start_pos].iter().collect();
    let after: String = if app.cursor_pos < chars.len() {
        chars[app.cursor_pos..].iter().collect()
    } else {
        String::new()
    };
    let replacement = format!("@{} ", skill_name);
    let new_cursor = before.chars().count() + replacement.chars().count();
    app.input = format!("{}{}{}", before, replacement, after);
    app.cursor_pos = new_cursor;
}

/// 更新文件补全弹窗的过滤文本
pub fn update_file_filter(app: &mut ChatApp) {
    let chars: Vec<char> = app.input.chars().collect();
    // @file: 占 6 个字符 (@file:), 过滤文本从 start_pos + 6 开始
    let start = app.file_popup_start_pos + 6;
    if start <= app.cursor_pos && app.cursor_pos <= chars.len() {
        app.file_popup_filter = chars[start..app.cursor_pos].iter().collect();
    } else {
        app.file_popup_filter.clear();
    }
    app.file_popup_selected = 0;
}

/// 获取文件补全列表
pub fn get_filtered_files(app: &ChatApp) -> Vec<String> {
    let filter = &app.file_popup_filter;

    // 解析 filter 为目录部分 + 文件名前缀
    let (dir_part, prefix) = if let Some(last_slash) = filter.rfind('/') {
        (&filter[..=last_slash], &filter[last_slash + 1..])
    } else {
        ("", filter.as_str())
    };

    let dir_path = if dir_part.is_empty() {
        std::path::PathBuf::from(".")
    } else {
        std::path::PathBuf::from(dir_part)
    };

    let prefix_lower = prefix.to_lowercase();

    let mut entries: Vec<String> = Vec::new();
    if let Ok(read_dir) = std::fs::read_dir(&dir_path) {
        for entry in read_dir.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            // 跳过隐藏文件（以 . 开头），除非用户已输入 .
            if name.starts_with('.') && !prefix.starts_with('.') {
                continue;
            }
            if !prefix_lower.is_empty() && !name.to_lowercase().starts_with(&prefix_lower) {
                continue;
            }
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            if is_dir {
                entries.push(format!("{}/", name));
            } else {
                entries.push(name);
            }
        }
    }

    // 排序：目录优先，然后按名称
    entries.sort_by(|a, b| {
        let a_dir = a.ends_with('/');
        let b_dir = b.ends_with('/');
        match (a_dir, b_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.to_lowercase().cmp(&b.to_lowercase()),
        }
    });

    entries.truncate(12);
    entries
}

/// 替换 input 中 @file:filter 为 @file:完整路径 + 空格
pub fn complete_file_mention(app: &mut ChatApp, file_path: &str) {
    let chars: Vec<char> = app.input.chars().collect();
    let before: String = chars[..app.file_popup_start_pos].iter().collect();
    let after: String = if app.cursor_pos < chars.len() {
        chars[app.cursor_pos..].iter().collect()
    } else {
        String::new()
    };
    let replacement = format!("@file:{} ", file_path);
    let new_cursor = before.chars().count() + replacement.chars().count();
    app.input = format!("{}{}{}", before, replacement, after);
    app.cursor_pos = new_cursor;
}
