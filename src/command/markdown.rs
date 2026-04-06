//! Markdown 编辑器命令
//!
//! `j md <file>` 用 Markdown 编辑器打开文件

use crate::command::chat::theme::{Theme, ThemeName};
use crate::config::YamlConfig;
use crate::tui::editor_markdown::open_markdown_editor;
use crate::{error, info};
use std::fs;
use std::path::PathBuf;

/// 处理 `j md` 命令
pub fn handle_md(file: Option<&str>, _config: &YamlConfig) {
    // 1. 解析文件路径
    let (content, file_path) = match file {
        Some(path) => {
            let expanded = expand_tilde(path);
            let path = PathBuf::from(&expanded);

            if path.exists() {
                match fs::read_to_string(&path) {
                    Ok(c) => (c, Some(path)),
                    Err(e) => {
                        error!("读取文件失败: {} - {}", path.display(), e);
                        return;
                    }
                }
            } else {
                // 文件不存在，创建新文件
                (String::new(), Some(path))
            }
        }
        None => (String::new(), None),
    };

    // 2. 获取主题
    let theme = Theme::from_name(&ThemeName::default());

    // 3. 构建标题
    let title = file_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "新文件".to_string());

    // 4. 打开编辑器
    match open_markdown_editor(&title, &content, &theme) {
        Ok(Some(new_content)) => {
            // 5. 保存文件
            if let Some(ref path) = file_path {
                // 检查内容是否变化
                if new_content != content {
                    if let Some(parent) = path.parent() {
                        if !parent.exists() {
                            if let Err(e) = fs::create_dir_all(parent) {
                                error!("创建目录失败: {} - {}", parent.display(), e);
                                return;
                            }
                        }
                    }

                    match fs::write(path, &new_content) {
                        Ok(()) => info!("文件已保存: {}", path.display()),
                        Err(e) => error!("保存文件失败: {} - {}", path.display(), e),
                    }
                } else {
                    info!("内容未变化，跳过保存");
                }
            } else {
                // 无文件路径时，输出到 stdout
                print!("{}", new_content);
            }
        }
        Ok(None) => info!("已取消编辑"),
        Err(e) => error!("编辑器启动失败: {}", e),
    }
}

/// 展开 ~ 为 home 目录
fn expand_tilde(path: &str) -> String {
    if (path == "~" || path.starts_with("~/"))
        && let Some(home) = dirs::home_dir()
    {
        if path == "~" {
            home.display().to_string()
        } else {
            format!("{}{}", home.display(), &path[1..])
        }
    } else {
        path.to_string()
    }
}
