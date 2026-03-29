use super::compact::CompactConfig;
use super::theme::ThemeName;
use crate::config::YamlConfig;
use crate::error;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

// ========== 数据结构 ==========

/// 单个模型提供方配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProvider {
    /// 显示名称（如 "GPT-4o", "DeepSeek-V3"）
    pub name: String,
    /// API Base URL（如 "https://api.openai.com/v1"）
    pub api_base: String,
    /// API Key
    pub api_key: String,
    /// 模型名称（如 "gpt-4o", "deepseek-chat"）
    pub model: String,
}

/// Agent 配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
    /// 是否使用流式输出（默认 true，设为 false 则等回复完整后再显示）
    #[serde(default = "default_stream_mode")]
    pub stream_mode: bool,
    /// 发送给 API 的历史消息数量限制（默认 20 条，避免 token 消耗过大）
    #[serde(default = "default_max_history_messages")]
    pub max_history_messages: usize,
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
    /// 被禁用的 skill 名称列表（列表中的 skill 不会包含在系统提示词中）
    #[serde(default)]
    pub disabled_skills: Vec<String>,
    /// Context compact 配置
    #[serde(default)]
    pub compact: CompactConfig,
}

fn default_max_history_messages() -> usize {
    20
}

/// 默认流式输出
fn default_stream_mode() -> bool {
    true
}

/// 默认工具调用最大轮数
fn default_max_tool_rounds() -> usize {
    100
}

/// 单次工具调用请求（序列化到历史记录）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallItem {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

/// 对话消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String, // "user" | "assistant" | "system" | "tool"
    /// 消息内容（tool_call 类消息可为空）
    #[serde(default)]
    pub content: String,
    /// LLM 发起的工具调用列表（仅 assistant 角色且有 tool_calls 时非 None）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallItem>>,
    /// 工具执行结果对应的 tool_call_id（仅 tool 角色时非 None）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl ChatMessage {
    /// 创建普通文本消息
    pub fn text(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
        }
    }
}

/// 对话会话
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatSession {
    pub messages: Vec<ChatMessage>,
}

// ========== JSONL 会话事件 ==========

/// Session JSONL 事件类型（每行一个事件，append-only）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionEvent {
    /// 新增一条消息
    Msg(ChatMessage),
    /// 对话清空
    Clear,
    /// 归档还原（messages 为还原后的完整消息列表）
    Restore { messages: Vec<ChatMessage> },
}

// ========== 文件路径 ==========

/// 获取 agent 数据目录: ~/.jdata/agent/data/
pub fn agent_data_dir() -> PathBuf {
    let dir = YamlConfig::data_dir().join("agent").join("data");
    let _ = fs::create_dir_all(&dir);
    dir
}

/// 获取 sessions 目录: ~/.jdata/agent/data/sessions/
pub fn sessions_dir() -> PathBuf {
    let dir = agent_data_dir().join("sessions");
    let _ = fs::create_dir_all(&dir);
    dir
}

/// 获取单个 session 的 JSONL 文件路径
pub fn session_file_path(session_id: &str) -> PathBuf {
    sessions_dir().join(format!("{}.jsonl", session_id))
}

/// 获取 agent 配置文件路径
pub fn agent_config_path() -> PathBuf {
    agent_data_dir().join("agent_config.json")
}

/// 已废弃：旧的单文件对话历史路径（仅用于迁移检测）
pub fn legacy_chat_history_path() -> PathBuf {
    agent_data_dir().join("chat_history.json")
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

/// 获取用户级 hooks 配置文件路径: ~/.jdata/agent/hooks.yaml
pub fn hooks_config_path() -> PathBuf {
    let dir = YamlConfig::data_dir().join("agent");
    let _ = fs::create_dir_all(&dir);
    dir.join("hooks.yaml")
}

// ========== 配置读写 ==========

/// 加载 Agent 配置
pub fn load_agent_config() -> AgentConfig {
    let path = agent_config_path();
    if !path.exists() {
        return AgentConfig::default();
    }
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|e| {
            error!("✖️ 解析 agent_config.json 失败: {}", e);
            AgentConfig::default()
        }),
        Err(e) => {
            error!("✖️ 读取 agent_config.json 失败: {}", e);
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
                error!("✖️ 保存 agent_config.json 失败: {}", e);
                false
            }
        },
        Err(e) => {
            error!("✖️ 序列化 agent 配置失败: {}", e);
            false
        }
    }
}

/// 追加一个事件到 session JSONL 文件（append-only，POSIX 下原子安全）
pub fn append_session_event(session_id: &str, event: &SessionEvent) -> bool {
    let path = session_file_path(session_id);
    match serde_json::to_string(event) {
        Ok(line) => match fs::OpenOptions::new().create(true).append(true).open(&path) {
            Ok(mut file) => writeln!(file, "{}", line).is_ok(),
            Err(_) => false,
        },
        Err(_) => false,
    }
}

/// 从 JSONL 文件 replay 出 ChatSession（供 resume 等功能使用）
#[allow(dead_code)]
pub fn load_session(session_id: &str) -> ChatSession {
    let path = session_file_path(session_id);
    if !path.exists() {
        return ChatSession::default();
    }
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return ChatSession::default(),
    };
    let mut messages: Vec<ChatMessage> = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<SessionEvent>(line) {
            Ok(event) => match event {
                SessionEvent::Msg(msg) => messages.push(msg),
                SessionEvent::Clear => messages.clear(),
                SessionEvent::Restore { messages: restored } => messages = restored,
            },
            Err(_) => {
                // 损坏行直接跳过，继续处理剩余行
            }
        }
    }
    ChatSession { messages }
}

/// 加载系统提示词（来自独立文件）
pub fn load_system_prompt() -> Option<String> {
    let path = system_prompt_path();
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
            error!("✖️ 读取 system_prompt.md 失败: {}", e);
            None
        }
    }
}

/// 保存系统提示词到独立文件（空字符串会删除文件）
pub fn save_system_prompt(prompt: &str) -> bool {
    let path = system_prompt_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let trimmed = prompt.trim();
    if trimmed.is_empty() {
        return match fs::remove_file(&path) {
            Ok(_) => true,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => true,
            Err(e) => {
                error!("✖️ 删除 system_prompt.md 失败: {}", e);
                false
            }
        };
    }

    match fs::write(path, trimmed) {
        Ok(_) => true,
        Err(e) => {
            error!("✖️ 保存 system_prompt.md 失败: {}", e);
            false
        }
    }
}

/// 加载回复风格（来自独立文件）
pub fn load_style() -> Option<String> {
    let path = style_path();
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
            error!("✖️ 读取 style.md 失败: {}", e);
            None
        }
    }
}

/// 保存回复风格到独立文件（空字符串会删除文件）
pub fn save_style(style: &str) -> bool {
    let path = style_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let trimmed = style.trim();
    if trimmed.is_empty() {
        return match fs::remove_file(&path) {
            Ok(_) => true,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => true,
            Err(e) => {
                error!("✖️ 删除 style.md 失败: {}", e);
                false
            }
        };
    }

    match fs::write(path, trimmed) {
        Ok(_) => true,
        Err(e) => {
            error!("✖️ 保存 style.md 失败: {}", e);
            false
        }
    }
}

/// 加载记忆（来自独立文件）
pub fn load_memory() -> Option<String> {
    let path = memory_path();
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
            error!("✖️ 读取 memory.md 失败: {}", e);
            None
        }
    }
}

/// 加载灵魂（来自独立文件）
pub fn load_soul() -> Option<String> {
    let path = soul_path();
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
            error!("✖️ 读取 soul.md 失败: {}", e);
            None
        }
    }
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
            error!("✖️ 保存 memory.md 失败: {}", e);
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
            error!("✖️ 保存 soul.md 失败: {}", e);
            false
        }
    }
}
