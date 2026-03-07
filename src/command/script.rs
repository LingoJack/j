use crate::config::YamlConfig;
use crate::constants::{section, shell};
use crate::{error, info};
use std::fs;

/// 生成脚本末尾的「等待用户按键」模板内容
fn wait_for_key_template() -> String {
    if std::env::consts::OS == shell::WINDOWS_OS {
        "echo.\necho 脚本执行完毕，按任意键退出...\npause >nul".to_string()
    } else {
        "echo ''\necho '\\033[32m✅ 脚本执行完毕，按回车键退出...\\033[0m'\nread _".to_string()
    }
}

/// 处理 concat 命令: j concat <script_name> ["<script_content>"]
/// 创建一个脚本文件并注册为别名，脚本持久化在 ~/.jdata/scripts/ 下
/// 如果没有提供 content，则打开 TUI 编辑器让用户输入
pub fn handle_concat(name: &str, content: &[String], config: &mut YamlConfig) {
    // 检查脚本名是否已存在 → 如果存在则进入编辑模式
    if config.contains(section::PATH, name) {
        // 获取已有脚本路径
        let existing_path = match config
            .get_property(section::SCRIPT, name)
            .or_else(|| config.get_property(section::PATH, name))
        {
            Some(p) => p.clone(),
            None => {
                error!("❌ 别名 {{{}}} 已存在，但未找到对应的脚本路径", name);
                return;
            }
        };

        // 读取已有脚本内容
        let existing_content = match fs::read_to_string(&existing_path) {
            Ok(c) => c,
            Err(e) => {
                error!("❌ 读取已有脚本文件失败: {} (路径: {})", e, existing_path);
                return;
            }
        };

        // 打开 TUI 编辑器让用户修改
        let initial_lines: Vec<String> = existing_content.lines().map(|l| l.to_string()).collect();
        match crate::tui::editor::open_multiline_editor_with_content(
            &format!("📝 编辑脚本: {}", name),
            &initial_lines,
        ) {
            Ok(Some(new_content)) => {
                if new_content.trim().is_empty() {
                    error!("⚠️ 脚本内容为空，未保存修改");
                    return;
                }
                // 写回脚本文件
                match fs::write(&existing_path, &new_content) {
                    Ok(_) => info!("✅ 脚本 {{{}}} 已更新，路径: {}", name, existing_path),
                    Err(e) => error!("💥 写入脚本文件失败: {}", e),
                }
            }
            Ok(None) => {
                info!("已取消编辑脚本");
            }
            Err(e) => {
                error!("❌ 编辑器启动失败: {}", e);
            }
        }
        return;
    }

    // 获取脚本内容：有参数则直接使用，无参数则打开编辑器
    let script_content = if content.is_empty() {
        // 无内容参数：打开 TUI 编辑器
        let initial_lines = vec![
            "#!/bin/bash".to_string(),
            "".to_string(),
            "# 在此编写脚本内容...".to_string(),
            "".to_string(),
            "# --- 以下为等待按键模板（可删除） ---".to_string(),
            wait_for_key_template(),
        ];

        match crate::tui::editor::open_multiline_editor_with_content(
            &format!("📝 编写脚本: {}", name),
            &initial_lines,
        ) {
            Ok(Some(text)) => text,
            Ok(None) => {
                info!("已取消创建脚本");
                return;
            }
            Err(e) => {
                error!("❌ 编辑器启动失败: {}", e);
                return;
            }
        }
    } else {
        // 有内容参数：拼接并去除两端引号
        let text = content.join(" ");
        text.trim()
            .trim_start_matches('"')
            .trim_end_matches('"')
            .to_string()
    };

    if script_content.trim().is_empty() {
        error!("⚠️ 脚本内容为空，无法创建");
        return;
    }

    // 脚本统一存储在 ~/.jdata/scripts/ 下
    let scripts_dir = YamlConfig::scripts_dir();

    // 生成脚本文件路径
    let ext = if std::env::consts::OS == shell::WINDOWS_OS {
        ".cmd"
    } else {
        ".sh"
    };
    let script_path = scripts_dir.join(format!("{}{}", name, ext));
    let script_path_str = script_path.to_string_lossy().to_string();

    // 确保目录存在（scripts_dir() 已保证，这里冗余保护）
    if let Some(parent) = script_path.parent()
        && let Err(e) = fs::create_dir_all(parent)
    {
        error!("❌ 创建目录失败: {}", e);
        return;
    }

    // 写入脚本内容
    match fs::write(&script_path, &script_content) {
        Ok(_) => {
            info!("🎉 文件创建成功: {}", script_path_str);
        }
        Err(e) => {
            error!("💥 写入脚本文件失败: {}", e);
            return;
        }
    }

    // 设置执行权限（非 Windows）
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = fs::metadata(&script_path) {
            let mut perms = metadata.permissions();
            perms.set_mode(perms.mode() | 0o111); // 添加执行权限
            if let Err(e) = fs::set_permissions(&script_path, perms) {
                error!("❌ 设置执行权限失败: {}", e);
            } else {
                info!("🔧 已为脚本 {{{}}} 设置执行权限", name);
            }
        }
    }

    // 注册到 path 和 script
    config.set_property(section::PATH, name, &script_path_str);
    config.set_property(section::SCRIPT, name, &script_path_str);

    info!("✅ 成功创建脚本 {{{}}}，路径: {}", name, script_path_str);
}
