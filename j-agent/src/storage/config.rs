use crate::constants::{
    DEFAULT_MAX_CONTEXT_TOKENS, DEFAULT_MAX_HISTORY_MESSAGES, DEFAULT_MAX_TOOL_ROUNDS,
};
use crate::context::compact::CompactConfig;
use crate::theme_name::ThemeName;
use crate::util::shell_runtime::ShellRuntime;
use serde::{Deserialize, Deserializer, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallMode {
    #[default]
    Native,
    Disabled,
}

impl<'de> Deserialize<'de> for ToolCallMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(Self::parse(&value))
    }
}

impl ToolCallMode {
    pub const ALL: &[ToolCallMode] = &[ToolCallMode::Native, ToolCallMode::Disabled];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Disabled => "disabled",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Native => "原生工具调用",
            Self::Disabled => "禁用工具调用",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "disabled" | "off" | "false" => Self::Disabled,
            _ => Self::Native,
        }
    }

    pub fn next(&self) -> Self {
        let idx = Self::ALL.iter().position(|m| m == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn allows_tools(&self) -> bool {
        matches!(self, Self::Native)
    }
}

/// 单个模型提供方配置
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelProvider {
    /// 显示名称（如 "GPT-4o", "DeepSeek-V3"）
    pub name: String,
    /// API Base URL（如 "https://api.openai.com/v1"）
    pub api_base: String,
    /// API Key
    pub api_key: String,
    /// 模型名称（如 "gpt-4o", "deepseek-chat"）
    pub model: String,
    /// 是否支持视觉/多模态（默认 false）
    #[serde(default)]
    pub supports_vision: bool,
    /// 工具调用协议模式（默认 native）
    #[serde(default)]
    pub tool_call_mode: ToolCallMode,
    /// 单次请求最大输出 token 数（None = 用 API 默认值）
    ///
    /// 推理模型（如 deepseek-r1、ark-code-latest）常把 token 花在 reasoning
    /// 上导致可见输出为空，建议显式调大（如 8192、16384）。
    #[serde(default)]
    pub max_tokens: Option<u32>,
    /// 思考强度（reasoning_effort），留空 = 不发送思考参数（使用 API 默认行为）。
    ///
    /// 常用值：`low` / `high` / `max` / `xhigh`，也可填入 API 支持的任意字符串。
    /// 非空时会同时发送 `reasoning_effort` 和 `thinking: {"type":"enabled"}` 两个
    /// 顶层参数，兼容 DeepSeek 官方 API 与火山方舟 Ark API。
    #[serde(default)]
    pub thinking_effort: String,
}

impl ModelProvider {
    pub fn tools_allowed(&self, global_tools_enabled: bool) -> bool {
        global_tools_enabled && self.tool_call_mode.allows_tools()
    }
}

/// 思考指示器动画风格
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThinkingStyle {
    /// Braille 点阵旋转（默认）
    #[default]
    Braille,
    /// 经典圆点（原版 ◍ + 颜色脉冲）
    Classic,
    /// 圆环呼吸（渐变大小）
    Pulse,
    /// 三点波浪
    Wave,
    /// 光标闪烁
    Blink,
    /// 渐变彗星（拖尾字符密度渐变）
    Comet,
}

impl ThinkingStyle {
    /// 所有可能值，用于 config panel 循环切换
    pub const ALL: &[ThinkingStyle] = &[
        ThinkingStyle::Braille,
        ThinkingStyle::Classic,
        ThinkingStyle::Pulse,
        ThinkingStyle::Wave,
        ThinkingStyle::Blink,
        ThinkingStyle::Comet,
    ];

    /// 显示名称（中文）
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Braille => "旋转点阵",
            Self::Classic => "经典圆点",
            Self::Pulse => "呼吸圆点",
            Self::Wave => "波浪三连",
            Self::Blink => "闪烁光标",
            Self::Comet => "渐变彗星",
        }
    }

    /// 序列化名称
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Braille => "braille",
            Self::Classic => "classic",
            Self::Pulse => "pulse",
            Self::Wave => "wave",
            Self::Blink => "blink",
            Self::Comet => "comet",
        }
    }

    /// 从字符串解析，支持英文标识和中文名
    pub fn parse(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "braille" => Self::Braille,
            "classic" => Self::Classic,
            "pulse" => Self::Pulse,
            "wave" => Self::Wave,
            "blink" => Self::Blink,
            "comet" => Self::Comet,
            // 中文名映射
            "旋转点阵" => Self::Braille,
            "经典圆点" => Self::Classic,
            "呼吸圆点" => Self::Pulse,
            "波浪三连" => Self::Wave,
            "闪烁光标" => Self::Blink,
            "渐变彗星" => Self::Comet,
            _ => Self::default(),
        }
    }

    /// 切换到下一个风格
    pub fn next(&self) -> Self {
        let idx = Self::ALL.iter().position(|s| s == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    /// 基于 tick（每 100ms 递增 1）返回当前帧的显示字符
    pub fn frame(&self, tick: u64) -> &'static str {
        match self {
            Self::Braille => {
                const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
                FRAMES[(tick as usize) % FRAMES.len()]
            }
            Self::Classic => "◍",
            Self::Pulse => {
                const FRAMES: &[&str] = &["·", "◦", "○", "◔", "◕", "●", "◕", "◔", "○", "◦"];
                FRAMES[(tick as usize) % FRAMES.len()]
            }
            Self::Wave => {
                const FRAMES: &[&str] = &["● · ·", "· ● ·", "· · ●", "· ● ·"];
                FRAMES[(tick as usize) % FRAMES.len()]
            }
            Self::Blink => {
                const FRAMES: &[&str] = &["█", " "];
                FRAMES[(tick as usize / 5) % FRAMES.len()]
            }
            Self::Comet => {
                // 宽度 13 的轨道上，密度渐变的 "██▓▒░" 彗星左右来回弹跳（ping-pong）
                const FRAMES: &[&str] = &[
                    "██▓▒░        ",
                    " ██▓▒░       ",
                    "  ██▓▒░      ",
                    "   ██▓▒░     ",
                    "    ██▓▒░    ",
                    "     ██▓▒░   ",
                    "      ██▓▒░  ",
                    "       ██▓▒░ ",
                    "        ██▓▒░",
                    "       ██▓▒░ ",
                    "      ██▓▒░  ",
                    "     ██▓▒░   ",
                    "    ██▓▒░    ",
                    "   ██▓▒░     ",
                    "  ██▓▒░      ",
                    " ██▓▒░       ",
                ];
                FRAMES[(tick as usize) % FRAMES.len()]
            }
        }
    }
}

/// Agent 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// 模型提供方列表
    #[serde(default)]
    pub providers: Vec<ModelProvider>,
    /// 当前选中的 provider 索引
    #[serde(default)]
    pub active_index: usize,
    /// 系统提示词（可选）
    #[serde(default)]
    pub system_prompt: Option<String>,
    /// 发送给 API 的历史消息数量限制（默认 20 条，避免 token 消耗过大）
    #[serde(default = "default_max_history_messages")]
    pub max_history_messages: usize,
    /// 上下文 token 预算（优先级选择时的 token 上限，默认 100K）
    #[serde(default = "default_max_context_tokens")]
    pub max_context_tokens: usize,
    /// 主题名称（dark / light / midnight）
    #[serde(default)]
    pub theme: ThemeName,
    /// 是否启用工具调用（默认关闭）
    #[serde(default)]
    pub tools_enabled: bool,
    /// 工具调用最大轮数（默认 10，防止无限循环）
    #[serde(default = "default_max_tool_rounds")]
    pub max_tool_rounds: usize,
    /// 回复风格（可选）
    #[serde(default)]
    pub style: Option<String>,
    /// 工具确认超时秒数（0 表示不超时，需手动确认；>0 则超时后自动执行）
    #[serde(default)]
    pub tool_confirm_timeout: u64,
    /// 被禁用的工具名称列表（tools_enabled=true 时，此列表中的工具不会发送给 LLM）
    #[serde(default)]
    pub disabled_tools: Vec<String>,
    /// 延迟加载的工具名称列表（需要 LoadTool 加载后才可用）
    #[serde(default = "default_deferred_tools")]
    pub deferred_tools: Vec<String>,
    /// 被禁用的 skill 名称列表（列表中的 skill 不会包含在系统提示词中）
    #[serde(default)]
    pub disabled_skills: Vec<String>,
    /// 被禁用的 command 名称列表
    #[serde(default)]
    pub disabled_commands: Vec<String>,
    /// 被禁用的 hook 标识列表（格式：`source:unique_id`，如 `user:my_hook`、`session:0`）
    #[serde(default)]
    pub disabled_hooks: Vec<String>,
    /// Context compact 配置
    #[serde(default)]
    pub compact: CompactConfig,
    /// 启动时是否自动恢复最近的 session
    #[serde(default)]
    pub auto_restore_session: bool,
    /// 气泡背景色与主背景色一致（扁平效果）
    #[serde(default = "default_true")]
    pub flat_bubble: bool,
    /// 思考指示器动画风格
    #[serde(default)]
    pub thinking_style: ThinkingStyle,
    /// 欢迎界面是否显示诗句
    #[serde(default = "default_true")]
    pub welcome_quote: bool,
}

fn default_max_history_messages() -> usize {
    DEFAULT_MAX_HISTORY_MESSAGES
}

fn default_max_context_tokens() -> usize {
    DEFAULT_MAX_CONTEXT_TOKENS
}

fn default_max_tool_rounds() -> usize {
    DEFAULT_MAX_TOOL_ROUNDS
}

fn default_true() -> bool {
    true
}

fn default_deferred_tools() -> Vec<String> {
    vec![
        "Task".to_string(),
        "RegisterHook".to_string(),
        "ComputerUse".to_string(),
        "Browser".to_string(),
    ]
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            providers: Vec::new(),
            active_index: 0,
            system_prompt: None,
            max_history_messages: DEFAULT_MAX_HISTORY_MESSAGES,
            max_context_tokens: DEFAULT_MAX_CONTEXT_TOKENS,
            theme: ThemeName::default(),
            tools_enabled: false,
            max_tool_rounds: DEFAULT_MAX_TOOL_ROUNDS,
            style: None,
            tool_confirm_timeout: 0,
            disabled_tools: Vec::new(),
            deferred_tools: default_deferred_tools(),
            disabled_skills: Vec::new(),
            disabled_commands: Vec::new(),
            disabled_hooks: Vec::new(),
            compact: CompactConfig::default(),
            auto_restore_session: false,
            flat_bubble: true,
            thinking_style: ThinkingStyle::default(),
            welcome_quote: true,
        }
    }
}

pub fn default_shell_runtime() -> ShellRuntime {
    ShellRuntime::Auto
}

// ========== 通用文本文件读写辅助 ==========

/// 从文件加载文本内容，trim 后返回；文件不存在或内容为空返回 None
fn load_text_file(path: &Path) -> Option<String> {
    if !path.exists() {
        return None;
    }
    match fs::read_to_string(path) {
        Ok(content) => {
            let trimmed = content.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        Err(e) => {
            eprintln!("[ERROR] ✖️ 读取 {} 失败: {}", path.display(), e);
            None
        }
    }
}

/// 保存文本内容到文件（空字符串删除文件）
fn save_text_file(path: &Path, content: &str) -> bool {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let trimmed = content.trim();
    if trimmed.is_empty() {
        return match fs::remove_file(path) {
            Ok(_) => true,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => true,
            Err(e) => {
                eprintln!("[ERROR] ✖️ 删除 {} 失败: {}", path.display(), e);
                false
            }
        };
    }

    match fs::write(path, trimmed) {
        Ok(_) => true,
        Err(e) => {
            eprintln!("[ERROR] ✖️ 保存 {} 失败: {}", path.display(), e);
            false
        }
    }
}

/// 获取 agent 数据目录: ~/.jdata/agent/data/
pub fn agent_data_dir() -> PathBuf {
    let dir = crate::constants::data_root().join("agent").join("data");
    let _ = fs::create_dir_all(&dir);
    dir
}

/// 获取 agent 配置文件路径
pub fn agent_config_path() -> PathBuf {
    agent_data_dir().join("agent_config.json")
}

/// 获取系统提示词文件路径
pub fn system_prompt_path() -> PathBuf {
    agent_data_dir().join("system_prompt.md")
}

/// 获取回复风格文件路径
pub fn style_path() -> PathBuf {
    agent_data_dir().join("style.md")
}

/// 获取记忆文件路径
pub fn memory_path() -> PathBuf {
    agent_data_dir().join("memory.md")
}

/// 获取灵魂文件路径
pub fn soul_path() -> PathBuf {
    agent_data_dir().join("soul.md")
}

/// 加载 Agent 配置
pub fn load_agent_config() -> AgentConfig {
    let path = agent_config_path();
    if !path.exists() {
        return AgentConfig::default();
    }
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|e| {
            eprintln!("[ERROR] ✖️ 解析 agent_config.json 失败: {}", e);
            AgentConfig::default()
        }),
        Err(e) => {
            eprintln!("[ERROR] ✖️ 读取 agent_config.json 失败: {}", e);
            AgentConfig::default()
        }
    }
}

/// 保存 Agent 配置
pub fn save_agent_config(config: &AgentConfig) -> bool {
    let path = agent_config_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    // system_prompt 和 style 统一存放在独立文件，不再写入 agent_config.json
    let mut config_to_save = config.clone();
    config_to_save.system_prompt = None;
    config_to_save.style = None;
    match serde_json::to_string_pretty(&config_to_save) {
        Ok(json) => match fs::write(&path, json) {
            Ok(_) => true,
            Err(e) => {
                eprintln!("[ERROR] ✖️ 保存 agent_config.json 失败: {}", e);
                false
            }
        },
        Err(e) => {
            eprintln!("[ERROR] ✖️ 序列化 agent 配置失败: {}", e);
            false
        }
    }
}

/// 加载系统提示词（来自独立文件）
pub fn load_system_prompt() -> Option<String> {
    load_text_file(&system_prompt_path())
}

/// 保存系统提示词到独立文件（空字符串会删除文件）
pub fn save_system_prompt(prompt: &str) -> bool {
    save_text_file(&system_prompt_path(), prompt)
}

/// 加载回复风格（来自独立文件）
pub fn load_style() -> Option<String> {
    load_text_file(&style_path())
}

/// 保存回复风格到独立文件（空字符串会删除文件）
pub fn save_style(style: &str) -> bool {
    save_text_file(&style_path(), style)
}

/// 加载记忆（来自独立文件）
pub fn load_memory() -> Option<String> {
    load_text_file(&memory_path())
}

/// 加载灵魂（来自独立文件）
pub fn load_soul() -> Option<String> {
    load_text_file(&soul_path())
}

/// 保存记忆到独立文件
pub fn save_memory(content: &str) -> bool {
    let path = memory_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    match fs::write(path, content) {
        Ok(_) => true,
        Err(e) => {
            eprintln!("[ERROR] ✖️ 保存 memory.md 失败: {}", e);
            false
        }
    }
}

/// 保存灵魂到独立文件
pub fn save_soul(content: &str) -> bool {
    let path = soul_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    match fs::write(path, content) {
        Ok(_) => true,
        Err(e) => {
            eprintln!("[ERROR] ✖️ 保存 soul.md 失败: {}", e);
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ModelProvider, ToolCallMode};

    #[test]
    fn provider_tools_allowed_respects_global_switch() {
        let provider = ModelProvider {
            name: "test".to_string(),
            api_base: "https://example.com/v1".to_string(),
            api_key: "key".to_string(),
            model: "model".to_string(),
            supports_vision: false,
            tool_call_mode: ToolCallMode::Native,
            max_tokens: None,
            thinking_effort: String::new(),
        };

        assert!(provider.tools_allowed(true));
        assert!(!provider.tools_allowed(false));
    }

    #[test]
    fn tool_call_mode_deserializes_alias_values() {
        let provider: ModelProvider = serde_json::from_value(serde_json::json!({
            "name": "test",
            "api_base": "https://example.com/v1",
            "api_key": "key",
            "model": "model",
            "supports_vision": false,
            "tool_call_mode": "off"
        }))
        .unwrap();

        assert_eq!(provider.tool_call_mode, ToolCallMode::Disabled);
    }
}
