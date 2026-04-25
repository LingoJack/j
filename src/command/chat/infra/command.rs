use crate::command::chat::permission::JcliConfig;
use crate::config::YamlConfig;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

// ========== 常量 ==========

/// 消息中引用自定义命令的前缀标记
const COMMAND_MENTION_PREFIX: &str = "@command:";

// ========== 数据结构 ==========

/// Command 来源层级
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandSource {
    /// 用户级: ~/.jdata/agent/commands/
    User,
    /// 项目级: .jcli/commands/
    Project,
}

impl CommandSource {
    /// 返回当前来源的中文标签
    pub fn label(&self) -> &'static str {
        match self {
            CommandSource::User => "用户",
            CommandSource::Project => "项目",
        }
    }
}

/// 自定义命令的 YAML frontmatter 元数据
#[derive(Debug, Clone, Deserialize)]
pub struct CommandFrontmatter {
    pub name: String,
    pub description: String,
}

/// 自定义命令，包含 frontmatter 元数据、提示词正文和来源层级
#[derive(Debug, Clone)]
pub struct CustomCommand {
    pub frontmatter: CommandFrontmatter,
    /// 提示词正文
    pub body: String,
    /// 来源层级
    pub source: CommandSource,
}

// ========== 加载与解析 ==========

/// 返回用户级 commands 目录: ~/.jdata/agent/commands/
pub fn commands_dir() -> PathBuf {
    let dir = YamlConfig::data_dir().join("agent").join("commands");
    let _ = fs::create_dir_all(&dir);
    dir
}

/// 返回项目级 commands 目录: .jcli/commands/（如果存在）
pub fn project_commands_dir() -> Option<PathBuf> {
    let config_dir = JcliConfig::find_config_dir()?;
    let dir = config_dir.join("commands");
    if dir.is_dir() { Some(dir) } else { None }
}

/// 解析 COMMAND.md: YAML frontmatter + body
fn parse_command_md(path: &Path, source: CommandSource) -> Option<CustomCommand> {
    let content = fs::read_to_string(path).ok()?;
    let (fm_str, body) = super::skill::split_frontmatter(&content)?;
    let frontmatter: CommandFrontmatter = serde_yaml::from_str(&fm_str).ok()?;

    if frontmatter.name.is_empty() {
        return None;
    }

    Some(CustomCommand {
        frontmatter,
        body: body.trim().to_string(),
        source,
    })
}

/// 从指定目录加载 commands
fn load_commands_from_dir(dir: &Path, source: CommandSource) -> Vec<CustomCommand> {
    let mut commands = Vec::new();
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return commands,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // 目录制: commands/review/COMMAND.md
            let cmd_md = path.join("COMMAND.md");
            if cmd_md.exists()
                && let Some(cmd) = parse_command_md(&cmd_md, source)
            {
                commands.push(cmd);
            }
        } else if path.extension().is_some_and(|ext| ext == "md") {
            // 单文件制: commands/review.md
            if let Some(cmd) = parse_command_md(&path, source) {
                commands.push(cmd);
            }
        }
    }
    commands
}

/// 加载所有 commands（用户级 + 项目级，同名时项目级覆盖）
pub fn load_all_commands() -> Vec<CustomCommand> {
    let mut map: HashMap<String, CustomCommand> = HashMap::new();

    // 1. 用户级
    for cmd in load_commands_from_dir(&commands_dir(), CommandSource::User) {
        map.insert(cmd.frontmatter.name.clone(), cmd);
    }

    // 2. 项目级（覆盖同名）
    if let Some(dir) = project_commands_dir() {
        for cmd in load_commands_from_dir(&dir, CommandSource::Project) {
            map.insert(cmd.frontmatter.name.clone(), cmd);
        }
    }

    let mut commands: Vec<CustomCommand> = map.into_values().collect();
    commands.sort_by(|a, b| a.frontmatter.name.cmp(&b.frontmatter.name));
    commands
}

/// 展开消息中的 @command:name 引用，替换为 command body
pub fn expand_command_mentions(
    text: &str,
    commands: &[CustomCommand],
    disabled: &[String],
) -> String {
    let mut result = text.to_string();
    // 反复查找 @command: 模式
    loop {
        let Some(start) = result.find(COMMAND_MENTION_PREFIX) else {
            break;
        };
        let name_start = start + COMMAND_MENTION_PREFIX.len();
        // 名称到空白字符或字符串末尾为止
        let name_end = result[name_start..]
            .find(|c: char| c.is_whitespace())
            .map(|i| name_start + i)
            .unwrap_or(result.len());
        let name = &result[name_start..name_end];

        if let Some(cmd) = commands
            .iter()
            .find(|c| c.frontmatter.name == name && !disabled.iter().any(|d| d == name))
        {
            result.replace_range(start..name_end, &cmd.body);
        } else {
            // 未找到匹配的 command，跳过避免死循环
            break;
        }
    }
    result
}
