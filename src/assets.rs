//! 编译时嵌入资源统一管理
//!
//! 使用 `rust-embed` 实现资源嵌入，支持运行时动态查找和迭代。
//!
//! # 资源清单
//!
//! | 资源名称 | 类型 | 路径 | 用途 |
//! |---------|------|------|------|
//! | `HELP_TABS` | 文本 | `assets/help/*.md` | 帮助命令 Tab 内容（frontmatter 驱动） |
//! | `VERSION_TEMPLATE` | 文本 | `assets/version.md` | 版本命令模板 |
//! | `DEFAULT_SYSTEM_PROMPT` | 文本 | `assets/system_prompt_default.md` | 默认系统提示词模板 |
//! | `DEFAULT_MEMORY` | 文本 | `assets/memory_default.md` | 默认记忆占位文件 |
//! | `DEFAULT_SOUL` | 文本 | `assets/soul_default.md` | 默认灵魂占位文件 |
//! | `DEFAULT_AGENT_MD` | 文本 | `assets/agent_md_default.md` | 默认 AGENT.md 模板 |
//! | `TEAMMATE_SYSTEM_PROMPT` | 文本 | `assets/teammate_system_prompt.md` | Teammate system prompt 模板 |
//! | `SUB_AGENT_SYSTEM_PROMPT` | 文本 | `assets/sub_agent_system_prompt.md` | SubAgent system prompt 模板 |

use crate::config::YamlConfig;
use crate::constants::section;
use rust_embed::RustEmbed;
use serde::Deserialize;
use std::borrow::Cow;
use std::fs;

/// 帮助 Tab 数据
#[derive(Debug, Clone)]
pub struct HelpTab {
    /// Tab 显示名称
    pub name: String,
    /// Tab Markdown 内容
    pub content: String,
}

/// 帮助 Tab frontmatter
#[derive(Deserialize)]
struct HelpFrontmatter {
    name: String,
    order: u32,
}

/// 编译时嵌入资源统一管理
///
/// 所有 assets 目录下的文件都会被嵌入到二进制中
#[derive(Debug, RustEmbed)]
#[folder = "assets/"]
#[exclude = "remote/node_modules/*"]
#[exclude = "remote/dist/*"]
pub struct Assets;

// ========== 便捷访问函数 ==========

/// 从嵌入资源加载帮助 Tab 列表
///
/// 遍历 `assets/help/` 目录下的 `.md` 文件，解析 YAML frontmatter
/// 中的 `name` 和 `order` 字段，按 `order` 排序后返回。
pub fn load_help_tabs() -> Vec<HelpTab> {
    let mut tabs: Vec<(u32, HelpTab)> = Vec::new();

    for filename in Assets::iter() {
        let filename = filename.as_ref();

        // 只处理 help/ 前缀的 .md 文件
        if !filename.starts_with("help/") || !filename.ends_with(".md") {
            continue;
        }

        let asset = match Assets::get(filename) {
            Some(a) => a,
            None => continue,
        };

        let content = String::from_utf8_lossy(&asset.data);

        // 解析 frontmatter
        if let Some((fm_str, body)) = split_help_frontmatter(&content)
            && let Ok(fm) = serde_yaml::from_str::<HelpFrontmatter>(&fm_str)
        {
            tabs.push((
                fm.order,
                HelpTab {
                    name: fm.name,
                    content: body,
                },
            ));
        }
    }

    // 按 order 排序
    tabs.sort_by_key(|(order, _)| *order);
    tabs.into_iter().map(|(_, tab)| tab).collect()
}

/// 按 `---` 分隔 frontmatter 和 body
///
/// 格式与 SKILL.md 一致：首行 `---`，YAML frontmatter，`---` 结束，剩余为 body。
fn split_help_frontmatter(content: &str) -> Option<(String, String)> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return None;
    }
    let after_first = &trimmed[3..];
    let end = after_first.find("\n---")?;
    let fm = after_first[..end].trim().to_string();
    let body = after_first[end + 4..].trim_start().to_string();
    Some((fm, body))
}

/// 版本信息模板
///
/// 用途: `j version` 命令输出
/// 占位符: `{version}`, `{os}`, `{extra}`
/// 格式: Markdown 表格
pub fn version_template() -> Cow<'static, str> {
    Assets::get("version.md")
        .map(|f| String::from_utf8_lossy(&f.data).into_owned().into())
        .unwrap_or_else(|| Cow::Borrowed(""))
}

/// 默认系统提示词模板
///
/// 用途: 首次运行时写入 `~/.jdata/agent/data/system_prompt.md`
/// 占位符: `{{.tools}}`, `{{.skills}}`, `{{.style}}`, `{{.memory}}`, `{{.soul}}`
/// 格式: Markdown
pub fn default_system_prompt() -> Cow<'static, str> {
    Assets::get("system_prompt_default.md")
        .map(|f| String::from_utf8_lossy(&f.data).into_owned().into())
        .unwrap_or_else(|| Cow::Borrowed(""))
}

/// 默认记忆占位文件
///
/// 用途: 首次运行时写入 `~/.jdata/agent/data/memory.md`
/// 格式: Markdown
pub fn default_memory() -> Cow<'static, str> {
    Assets::get("memory_default.md")
        .map(|f| String::from_utf8_lossy(&f.data).into_owned().into())
        .unwrap_or_else(|| Cow::Borrowed(""))
}

/// 默认灵魂占位文件
///
/// 用途: 首次运行时写入 `~/.jdata/agent/data/soul.md`
/// 格式: Markdown
pub fn default_soul() -> Cow<'static, str> {
    Assets::get("soul_default.md")
        .map(|f| String::from_utf8_lossy(&f.data).into_owned().into())
        .unwrap_or_else(|| Cow::Borrowed(""))
}

/// 默认 AGENT.md 模板
///
/// 用途: 首次运行时写入 `~/.jdata/agent/AGENT.md`
/// 格式: Markdown
pub fn default_agent_md() -> Cow<'static, str> {
    Assets::get("agent_md_default.md")
        .map(|f| String::from_utf8_lossy(&f.data).into_owned().into())
        .unwrap_or_else(|| Cow::Borrowed(""))
}

/// Teammate system prompt 模板
///
/// 用途: 构建 teammate 专用的 system prompt
/// 占位符: `{{.base_prompt}}`, `{{.name}}`, `{{.role}}`, `{{.team_summary}}`
/// 格式: Markdown
pub fn teammate_system_prompt_template() -> Cow<'static, str> {
    Assets::get("teammate_system_prompt.md")
        .map(|f| String::from_utf8_lossy(&f.data).into_owned().into())
        .unwrap_or_else(|| Cow::Borrowed(""))
}

/// SubAgent system prompt 模板
///
/// 用途: 构建 SubAgent 专用的 system prompt（替代直接复用父 agent prompt）
/// 占位符: `{{.base_prompt}}`
/// 格式: Markdown
pub fn sub_agent_system_prompt_template() -> Cow<'static, str> {
    Assets::get("sub_agent_system_prompt.md")
        .map(|f| String::from_utf8_lossy(&f.data).into_owned().into())
        .unwrap_or_else(|| Cow::Borrowed(""))
}

/// 诗句语录文本
///
/// 用途: Chat UI 欢迎框随机展示一句诗句
/// 格式: 纯文本，每行一句
///
/// 使用 `include_str!` 而非 `include_dir!` 的 `Assets::get`，
/// 因为 proc macro 增量编译缓存可能导致新增文件不被重新扫描。
pub fn quotes_text() -> &'static str {
    include_str!("../assets/quotes.txt")
}

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
        config.set_property(section::PATH, &entry.name, &path_str);
        config.set_property(section::SCRIPT, &entry.name, &path_str);
    }

    Ok(())
}
