use std::fs;

use serde::Deserialize;

use crate::config::YamlConfig;
use crate::constants::section;

use super::Assets;

/// 安装预设 skills 到用户数据目录
///
/// 遍历编译时嵌入的 `assets/skills/` 目录下的所有文件，
/// 将其写入 `~/.jdata/agent/skills/` 对应路径（仅当 skill 目录不存在时才写入，不覆盖用户修改）。
pub fn install_default_skills(skills_dir: &std::path::Path) -> Result<(), std::io::Error> {
    for filename in Assets::iter() {
        let filename = filename.as_ref();

        // 只处理 skills/ 前缀的文件
        if !filename.starts_with("skills/") {
            continue;
        }

        // 提取相对路径，如 "skills/my-skill/SKILL.md" → "my-skill/SKILL.md"
        let rel_path = &filename["skills/".len()..];
        if rel_path.is_empty() {
            continue;
        }

        // skills 目录原封不动复制到用户数据目录
        let dst_path = skills_dir.join(rel_path);
        if dst_path.exists() {
            continue;
        }

        let asset = Assets::get(filename).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("asset not found: {}", filename),
            )
        })?;

        if let Some(parent) = dst_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&dst_path, asset.data)?;
    }
    Ok(())
}

/// 安装预设 commands 到用户数据目录
///
/// 遍历编译时嵌入的 `assets/commands/` 目录下的所有文件，
/// 将其写入 `~/.jdata/agent/commands/` 对应路径（仅当目标文件不存在时才写入，不覆盖用户修改）。
pub fn install_default_commands(commands_dir: &std::path::Path) -> Result<(), std::io::Error> {
    for filename in Assets::iter() {
        let filename = filename.as_ref();

        // 只处理 commands/ 前缀的文件
        if !filename.starts_with("commands/") {
            continue;
        }

        // 提取相对路径，如 "commands/review/COMMAND.md" → "review/COMMAND.md"
        let rel_path = &filename["commands/".len()..];
        if rel_path.is_empty() {
            continue;
        }

        // commands 目录原封不动复制到用户数据目录
        let dst_path = commands_dir.join(rel_path);
        if dst_path.exists() {
            continue;
        }

        let asset = Assets::get(filename).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("asset not found: {}", filename),
            )
        })?;

        if let Some(parent) = dst_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&dst_path, asset.data)?;
    }
    Ok(())
}

// ========== 预置脚本清单条目 ==========

#[derive(Deserialize)]
struct PresetEntry {
    name: String,
    file: String,
}

/// 安装预置脚本到用户数据目录并注册到配置
///
/// 从编译时嵌入的 `assets/presets/manifest.yaml` 读取脚本清单，
/// 将对应的 `.sh` 文件写入 `~/.jdata/scripts/{name}.sh`，
/// 并注册到 config.yaml 的 `[path]` 和 `[script]` section。
///
/// - 如果脚本文件已存在，跳过（不覆盖用户修改）
/// - 失败时返回错误，调用方应静默处理
pub fn install_default_scripts(config: &mut YamlConfig) -> Result<(), Box<dyn std::error::Error>> {
    // 读取 manifest
    let manifest_asset = Assets::get("presets/manifest.yaml")
        .ok_or("presets/manifest.yaml not found in embedded assets")?;
    let manifest_str = std::str::from_utf8(&manifest_asset.data)?;
    let entries: Vec<PresetEntry> = serde_yaml::from_str(manifest_str)?;

    let scripts_dir = YamlConfig::scripts_dir();

    for entry in &entries {
        let dst_path = scripts_dir.join(&entry.file);

        // 文件已存在则跳过
        if dst_path.exists() {
            continue;
        }

        // 从嵌入资源读取脚本内容
        let asset_path = format!("presets/{}", entry.file);
        let asset = Assets::get(&asset_path)
            .ok_or_else(|| format!("preset script not found in embedded assets: {}", asset_path))?;

        // 确保目录存在
        if let Some(parent) = dst_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // 写入文件
        fs::write(&dst_path, &asset.data)?;

        // 设置可执行权限（Unix）
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o755);
            fs::set_permissions(&dst_path, perms)?;
        }

        // 注册到 config: [path] 和 [script]
        let path_str = dst_path.to_string_lossy().to_string();
        config
            .set_property(section::PATH, &entry.name, &path_str)
            .map_err(|e| format!("注册 {} 到 PATH 失败: {}", entry.name, e))?;
        config
            .set_property(section::SCRIPT, &entry.name, &path_str)
            .map_err(|e| format!("注册 {} 到 SCRIPT 失败: {}", entry.name, e))?;
    }

    Ok(())
}
