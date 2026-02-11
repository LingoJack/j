use crate::config::YamlConfig;
use crate::{error, info};
use std::fs;

/// 处理 concat 命令: j concat <script_name> "<script_content>"
/// 创建一个脚本文件并注册为别名，脚本持久化在 ~/.jdata/scripts/ 下
pub fn handle_concat(name: &str, content: &str, config: &mut YamlConfig) {
    // 检查脚本名是否已存在
    if config.contains("path", name) {
        error!("❌ 失败！脚本名 {{{}}} 已经存在", name);
        return;
    }

    // 脚本统一存储在 ~/.jdata/scripts/ 下
    let scripts_dir = YamlConfig::scripts_dir();

    // 生成脚本文件路径
    let ext = if std::env::consts::OS == "windows" {
        ".cmd"
    } else {
        ".sh"
    };
    let script_path = scripts_dir.join(format!("{}{}", name, ext));
    let script_path_str = script_path.to_string_lossy().to_string();

    // 去除 content 两端的引号
    let script_content = content
        .trim()
        .trim_start_matches('"')
        .trim_end_matches('"');

    // 确保目录存在（scripts_dir() 已保证，这里冗余保护）
    if let Some(parent) = script_path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            error!("❌ 创建目录失败: {}", e);
            return;
        }
    }

    // 写入脚本内容
    match fs::write(&script_path, script_content) {
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
    config.set_property("path", name, &script_path_str);
    config.set_property("script", name, &script_path_str);

    info!(
        "✅ 成功创建脚本 {{{}}} 并写入内容: {}",
        name, script_content
    );
}
