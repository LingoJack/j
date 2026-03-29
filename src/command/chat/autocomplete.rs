use super::app::ChatApp;

pub fn update_at_filter(app: &mut ChatApp) {
    let chars: Vec<char> = app.ui.input.chars().collect();
    let start = app.ui.at_popup_start_pos + 1; // @ 之后
    if start <= app.ui.cursor_pos && app.ui.cursor_pos <= chars.len() {
        app.ui.at_popup_filter = chars[start..app.ui.cursor_pos].iter().collect();
    } else {
        app.ui.at_popup_filter.clear();
    }
    // 重置选中索引
    app.ui.at_popup_selected = 0;
}

/// 根据 filter 过滤 @ 弹窗的顶级选项（skill: 和 file:）
pub fn get_filtered_skills(app: &ChatApp) -> Vec<String> {
    let filter = app.ui.at_popup_filter.to_lowercase();
    let mut items: Vec<String> = Vec::new();
    let skill_label = "skill:".to_string();
    if filter.is_empty() || skill_label.contains(&filter) {
        items.push(skill_label);
    }
    let file_label = "file:".to_string();
    if filter.is_empty() || file_label.contains(&filter) {
        items.push(file_label);
    }
    items
}

/// 更新技能补全弹窗的过滤文本
pub fn update_skill_filter(app: &mut ChatApp) {
    let chars: Vec<char> = app.ui.input.chars().collect();
    // @skill: 占 7 个字符, 过滤文本从 start_pos + 7 开始
    let start = app.ui.skill_popup_start_pos + 7;
    if start <= app.ui.cursor_pos && app.ui.cursor_pos <= chars.len() {
        app.ui.skill_popup_filter = chars[start..app.ui.cursor_pos].iter().collect();
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
    let chars: Vec<char> = app.ui.input.chars().collect();
    let before: String = chars[..app.ui.skill_popup_start_pos].iter().collect();
    let after: String = if app.ui.cursor_pos < chars.len() {
        chars[app.ui.cursor_pos..].iter().collect()
    } else {
        String::new()
    };
    let replacement = format!("@skill:{} ", skill_name);
    let new_cursor = before.chars().count() + replacement.chars().count();
    app.ui.input = format!("{}{}{}", before, replacement, after);
    app.ui.cursor_pos = new_cursor;
}

/// 替换 input 中 @... 为 @skill_name 并加空格
pub fn complete_at_mention(app: &mut ChatApp, skill_name: &str) {
    let chars: Vec<char> = app.ui.input.chars().collect();
    let before: String = chars[..app.ui.at_popup_start_pos].iter().collect();
    let after: String = if app.ui.cursor_pos < chars.len() {
        chars[app.ui.cursor_pos..].iter().collect()
    } else {
        String::new()
    };
    let replacement = format!("@{} ", skill_name);
    let new_cursor = before.chars().count() + replacement.chars().count();
    app.ui.input = format!("{}{}{}", before, replacement, after);
    app.ui.cursor_pos = new_cursor;
}

/// 更新文件补全弹窗的过滤文本
pub fn update_file_filter(app: &mut ChatApp) {
    let chars: Vec<char> = app.ui.input.chars().collect();
    // @file: 占 6 个字符 (@file:), 过滤文本从 start_pos + 6 开始
    let start = app.ui.file_popup_start_pos + 6;
    if start <= app.ui.cursor_pos && app.ui.cursor_pos <= chars.len() {
        app.ui.file_popup_filter = chars[start..app.ui.cursor_pos].iter().collect();
    } else {
        app.ui.file_popup_filter.clear();
    }
    app.ui.file_popup_selected = 0;
}

/// 将 ~ 展开为用户 home 目录
fn expand_tilde(path: &str) -> String {
    if (path == "~" || path.starts_with("~/"))
        && let Some(home) = dirs::home_dir()
    {
        return format!("{}{}", home.display(), &path[1..]);
    }
    path.to_string()
}

/// 模糊匹配：filter 的每个字符按顺序出现在 text 中即可匹配，返回匹配分数（越小越好）
fn fuzzy_match(text: &str, filter: &str) -> Option<i32> {
    if filter.is_empty() {
        return Some(0);
    }
    let text_lower: Vec<char> = text.to_lowercase().chars().collect();
    let filter_lower: Vec<char> = filter.to_lowercase().chars().collect();
    let mut ti = 0;
    let mut score: i32 = 0;
    let mut last_match: Option<usize> = None;
    for &fc in &filter_lower {
        let mut found = false;
        while ti < text_lower.len() {
            if text_lower[ti] == fc {
                // 连续匹配加分（间距小更好）
                if let Some(lm) = last_match {
                    score += (ti - lm - 1) as i32;
                }
                last_match = Some(ti);
                ti += 1;
                found = true;
                break;
            }
            ti += 1;
        }
        if !found {
            return None;
        }
    }
    // 匹配起始位置越靠前越好
    Some(score)
}

/// 获取文件补全列表（全目录递归搜索，支持模糊匹配）
pub fn get_filtered_files(app: &ChatApp) -> Vec<String> {
    let filter = &app.ui.file_popup_filter;

    // 处理 ~ 路径
    let expanded;
    let effective_filter = if filter == "~" {
        expanded = "~/".to_string();
        &expanded
    } else {
        filter
    };

    // 如果 filter 包含 /，先尝试精确路径补全（逐层浏览模式）
    if let Some(last_slash) = effective_filter.rfind('/') {
        let dir_part = &effective_filter[..=last_slash];
        let prefix = &effective_filter[last_slash + 1..];
        let dir_path = if dir_part.is_empty() {
            std::path::PathBuf::from(".")
        } else {
            std::path::PathBuf::from(expand_tilde(dir_part))
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

    // 无 / 时使用递归全目录模糊搜索
    let search_root = std::path::PathBuf::from(".");
    let mut scored: Vec<(i32, String)> = Vec::new();

    let walker = ignore::WalkBuilder::new(&search_root)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .max_depth(Some(8))
        .build();

    for entry in walker.flatten() {
        let path = entry.path();
        // 跳过根目录本身
        if path == std::path::Path::new(".") {
            continue;
        }
        let rel = path
            .strip_prefix(&search_root)
            .unwrap_or(path)
            .to_string_lossy();
        let rel_str = rel.as_ref();

        // 跳过隐藏路径段（除非 filter 以 . 开头）
        if !effective_filter.starts_with('.') && rel_str.split('/').any(|seg| seg.starts_with('.'))
        {
            continue;
        }

        let is_dir = path.is_dir();
        let display = if is_dir {
            format!("{}/", rel_str)
        } else {
            rel_str.to_string()
        };

        // 用文件名部分做模糊匹配
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        if let Some(score) = fuzzy_match(&file_name, effective_filter) {
            // 路径深度作为次要排序因素
            let depth = rel_str.matches('/').count() as i32;
            scored.push((score * 10 + depth, display));
        }
    }

    // 按分数排序
    scored.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| a.1.to_lowercase().cmp(&b.1.to_lowercase()))
    });
    scored.truncate(15);
    scored.into_iter().map(|(_, path)| path).collect()
}

/// 替换 input 中 @file:filter 为 @file:完整路径 + 空格
pub fn complete_file_mention(app: &mut ChatApp, file_path: &str) {
    let chars: Vec<char> = app.ui.input.chars().collect();
    let before: String = chars[..app.ui.file_popup_start_pos].iter().collect();
    let after: String = if app.ui.cursor_pos < chars.len() {
        chars[app.ui.cursor_pos..].iter().collect()
    } else {
        String::new()
    };
    let replacement = format!("@file:{} ", file_path);
    let new_cursor = before.chars().count() + replacement.chars().count();
    app.ui.input = format!("{}{}{}", before, replacement, after);
    app.ui.cursor_pos = new_cursor;
}
