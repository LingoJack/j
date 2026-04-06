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
pub fn handle_md(file: &str, _config: &YamlConfig) {
    // 1. 解析文件路径
    let expanded = expand_tilde(file);
    let path = PathBuf::from(&expanded);

    let (content, is_new_file) = if path.exists() {
        match fs::read_to_string(&path) {
            Ok(c) => (c, false),
            Err(e) => {
                error!("读取文件失败: {} - {}", path.display(), e);
                return;
            }
        }
    } else {
        // 文件不存在，创建新文件
        (String::new(), true)
    };

    // 3. 获取主题
    let theme = Theme::from_name(&ThemeName::default());

    // 4. 构建标题
    let title = if is_new_file {
        format!("{} (新文件)", path.display())
    } else {
        path.display().to_string()
    };

    // 5. 打开编辑器
    match open_markdown_editor(&title, &content, &theme) {
        Ok(Some(new_content)) => {
            // 6. 保存文件
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

                match fs::write(&path, &new_content) {
                    Ok(()) => info!("文件已保存: {}", path.display()),
                    Err(e) => error!("保存文件失败: {} - {}", path.display(), e),
                }
            } else {
                info!("内容未变化，跳过保存");
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
