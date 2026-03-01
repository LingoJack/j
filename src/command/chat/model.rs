use super::theme::ThemeName;
use crate::config::YamlConfig;
use crate::error;
use serde::{Deserialize, Serialize};
use std::fs;
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
    10
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

// ========== 文件路径 ==========

/// 获取 agent 数据目录: ~/.jdata/agent/data/
pub fn agent_data_dir() -> PathBuf {
    let dir = YamlConfig::data_dir().join("agent").join("data");
    let _ = fs::create_dir_all(&dir);
    dir
}

/// 获取 agent 配置文件路径
pub fn agent_config_path() -> PathBuf {
    agent_data_dir().join("agent_config.json")
}

/// 获取对话历史文件路径
pub fn chat_history_path() -> PathBuf {
    agent_data_dir().join("chat_history.json")
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
            error!("❌ 解析 agent_config.json 失败: {}", e);
            AgentConfig::default()
        }),
        Err(e) => {
            error!("❌ 读取 agent_config.json 失败: {}", e);
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
    match serde_json::to_string_pretty(config) {
        Ok(json) => match fs::write(&path, json) {
            Ok(_) => true,
            Err(e) => {
                error!("❌ 保存 agent_config.json 失败: {}", e);
                false
            }
        },
        Err(e) => {
            error!("❌ 序列化 agent 配置失败: {}", e);
            false
        }
    }
}

/// 加载对话历史
pub fn load_chat_session() -> ChatSession {
    let path = chat_history_path();
    if !path.exists() {
        return ChatSession::default();
    }
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| ChatSession::default()),
        Err(_) => ChatSession::default(),
    }
}

/// 保存对话历史
pub fn save_chat_session(session: &ChatSession) -> bool {
    let path = chat_history_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    match serde_json::to_string_pretty(session) {
        Ok(json) => fs::write(&path, json).is_ok(),
        Err(_) => false,
    }
}
