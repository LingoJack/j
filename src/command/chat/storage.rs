use super::compact::CompactConfig;
use super::constants::{
    DEFAULT_MAX_CONTEXT_TOKENS, DEFAULT_MAX_HISTORY_MESSAGES, DEFAULT_MAX_TOOL_ROUNDS,
    MESSAGE_PREVIEW_MAX_LEN,
};
use super::theme::ThemeName;
use crate::config::YamlConfig;
use crate::error;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

// ========== 数据结构 ==========

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
    /// 被禁用的 skill 名称列表（列表中的 skill 不会包含在系统提示词中）
    #[serde(default)]
    pub disabled_skills: Vec<String>,
    /// 被禁用的 command 名称列表
    #[serde(default)]
    pub disabled_commands: Vec<String>,
    /// Context compact 配置
    #[serde(default)]
    pub compact: CompactConfig,
    /// 启动时是否自动恢复最近的 session
    #[serde(default)]
    pub auto_restore_session: bool,
}

fn default_max_history_messages() -> usize {
    DEFAULT_MAX_HISTORY_MESSAGES
}

fn default_max_context_tokens() -> usize {
    DEFAULT_MAX_CONTEXT_TOKENS
}

/// 默认工具调用最大轮数
fn default_max_tool_rounds() -> usize {
    DEFAULT_MAX_TOOL_ROUNDS
}

/// 单次工具调用请求（序列化到历史记录）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallItem {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

/// 图片数据（用于多模态消息，序列化时跳过以节省存储空间）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageData {
    /// base64 编码的图片数据
    pub base64: String,
    /// MIME 类型（如 "image/png", "image/jpeg"）
    pub media_type: String,
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
    /// 图片数据（用于多模态 user message，不持久化到 session 文件）
    #[serde(skip)]
    pub images: Option<Vec<ImageData>>,
}

impl ChatMessage {
    /// 创建普通文本消息
    pub fn text(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            images: None,
        }
    }

    /// 创建带图片的 user 消息
    #[allow(dead_code)]
    pub fn with_images(
        role: impl Into<String>,
        content: impl Into<String>,
        images: Vec<ImageData>,
    ) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            images: if images.is_empty() {
                None
            } else {
                Some(images)
            },
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
    Msg {
        #[serde(flatten)]
        message: ChatMessage,
        /// 消息产生时刻（epoch milliseconds）；老数据反序列化为 0。
        #[serde(default, skip_serializing_if = "is_zero_u64")]
        timestamp_ms: u64,
    },
    /// 对话清空
    Clear,
    /// 归档还原（messages 为还原后的完整消息列表）
    Restore { messages: Vec<ChatMessage> },
}

fn is_zero_u64(v: &u64) -> bool {
    *v == 0
}

/// 当前时刻（epoch milliseconds）
pub fn current_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

impl SessionEvent {
    /// 构造一条带当前时间戳的 Msg 事件
    pub fn msg(message: ChatMessage) -> Self {
        Self::Msg {
            message,
            timestamp_ms: current_millis(),
        }
    }
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

/// 获取单个 session 的 JSONL 文件路径（兼容别名，指向新布局主文件）
pub fn session_file_path(session_id: &str) -> PathBuf {
    SessionPaths::new(session_id).transcript()
}

/// Session 目录布局抽象。
///
/// 布局：`sessions/<id>/transcript.jsonl`。
#[derive(Debug)]
pub struct SessionPaths {
    id: String,
    dir: PathBuf,
}

impl SessionPaths {
    pub fn new(session_id: &str) -> Self {
        let dir = sessions_dir().join(session_id);
        Self {
            id: session_id.to_string(),
            dir,
        }
    }

    #[allow(dead_code)]
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// 主数据文件：`sessions/<id>/transcript.jsonl`
    pub fn transcript(&self) -> PathBuf {
        self.dir.join("transcript.jsonl")
    }

    /// 元数据文件：`sessions/<id>/session.json`
    pub fn meta_file(&self) -> PathBuf {
        self.dir.join("session.json")
    }

    /// compact 快照目录：`sessions/<id>/.transcripts/`
    pub fn transcripts_dir(&self) -> PathBuf {
        self.dir.join(".transcripts")
    }

    /// Teammate 状态文件：`sessions/<id>/teammates.json`
    pub fn teammates_file(&self) -> PathBuf {
        self.dir.join("teammates.json")
    }

    /// Teammate 独立目录根：`sessions/<id>/teammates/`
    pub fn teammates_dir(&self) -> PathBuf {
        self.dir.join("teammates")
    }

    /// 单个 teammate 的独立子目录：`sessions/<id>/teammates/<sanitized_name>/`
    pub fn teammate_dir(&self, sanitized_name: &str) -> PathBuf {
        self.teammates_dir().join(sanitized_name)
    }

    /// 单个 teammate 的 transcript JSONL 路径：`sessions/<id>/teammates/<sanitized_name>/transcript.jsonl`
    pub fn teammate_transcript(&self, sanitized_name: &str) -> PathBuf {
        self.teammate_dir(sanitized_name).join("transcript.jsonl")
    }

    /// 单个 teammate 的 todo 文件路径：`sessions/<id>/teammates/<sanitized_name>/todos.json`
    pub fn teammate_todos_file(&self, sanitized_name: &str) -> PathBuf {
        self.teammate_dir(sanitized_name).join("todos.json")
    }

    /// SubAgent 状态文件：`sessions/<id>/subagents.json`
    pub fn subagents_file(&self) -> PathBuf {
        self.dir.join("subagents.json")
    }

    /// SubAgent 独立目录根：`sessions/<id>/subagents/`
    pub fn subagents_dir(&self) -> PathBuf {
        self.dir.join("subagents")
    }

    /// 单个 subagent 的独立子目录：`sessions/<id>/subagents/<sub_id>/`
    pub fn subagent_dir(&self, sub_id: &str) -> PathBuf {
        self.subagents_dir().join(sub_id)
    }

    /// 单个 subagent 的 transcript JSONL 路径：`sessions/<id>/subagents/<sub_id>/transcript.jsonl`
    pub fn subagent_transcript(&self, sub_id: &str) -> PathBuf {
        self.subagent_dir(sub_id).join("transcript.jsonl")
    }

    /// 单个 subagent 的 todo 文件路径：`sessions/<id>/subagents/<sub_id>/todos.json`
    pub fn subagent_todos_file(&self, sub_id: &str) -> PathBuf {
        self.subagent_dir(sub_id).join("todos.json")
    }

    /// Task 状态文件：`sessions/<id>/tasks.json`
    pub fn tasks_file(&self) -> PathBuf {
        self.dir.join("tasks.json")
    }

    /// Todo 状态文件：`sessions/<id>/todos.json`
    pub fn todos_file(&self) -> PathBuf {
        self.dir.join("todos.json")
    }

    /// Plan 状态文件：`sessions/<id>/plan.json`
    pub fn plan_file(&self) -> PathBuf {
        self.dir.join("plan.json")
    }

    /// InvokedSkills 状态文件：`sessions/<id>/skills.json`
    pub fn skills_file(&self) -> PathBuf {
        self.dir.join("skills.json")
    }

    /// Session Hook 状态文件：`sessions/<id>/hooks.json`
    pub fn hooks_file(&self) -> PathBuf {
        self.dir.join("hooks.json")
    }

    /// Sandbox 状态文件：`sessions/<id>/sandbox.json`
    pub fn sandbox_file(&self) -> PathBuf {
        self.dir.join("sandbox.json")
    }

    pub fn ensure_dir(&self) -> std::io::Result<()> {
        fs::create_dir_all(&self.dir)
    }
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
///
/// 同时增量更新 `session.json` 元数据。
pub fn append_session_event(session_id: &str, event: &SessionEvent) -> bool {
    let paths = SessionPaths::new(session_id);
    if paths.ensure_dir().is_err() {
        return false;
    }
    let path = paths.transcript();
    let ok = match serde_json::to_string(event) {
        Ok(line) => match fs::OpenOptions::new().create(true).append(true).open(&path) {
            Ok(mut file) => writeln!(file, "{}", line).is_ok(),
            Err(_) => false,
        },
        Err(_) => false,
    };
    if ok {
        update_session_meta_on_event(session_id, event);
    }
    ok
}

/// 增量更新 session.json 元数据（追加事件后调用）
fn update_session_meta_on_event(session_id: &str, event: &SessionEvent) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let mut meta = load_session_meta_file(session_id).unwrap_or_else(|| SessionMetaFile {
        id: session_id.to_string(),
        title: String::new(),
        message_count: 0,
        created_at: now,
        updated_at: now,
        model: None,
    });
    meta.updated_at = now;
    match event {
        SessionEvent::Msg { message: msg, .. } => {
            meta.message_count += 1;
            if meta.title.is_empty() && msg.role == "user" && !msg.content.is_empty() {
                meta.title = msg.content.chars().take(MESSAGE_PREVIEW_MAX_LEN).collect();
            }
        }
        SessionEvent::Clear => {
            meta.message_count = 0;
        }
        SessionEvent::Restore { messages } => {
            meta.message_count = messages.len();
            if meta.title.is_empty()
                && let Some(first_user) = messages
                    .iter()
                    .find(|m| m.role == "user" && !m.content.is_empty())
            {
                meta.title = first_user
                    .content
                    .chars()
                    .take(MESSAGE_PREVIEW_MAX_LEN)
                    .collect();
            }
        }
    }
    let _ = save_session_meta_file(&meta);
}

/// 查找最近修改的 session ID（用于 --continue）
pub fn find_latest_session_id() -> Option<String> {
    let dir = sessions_dir();
    let mut entries: Vec<(std::time::SystemTime, String)> = Vec::new();
    let read_dir = match fs::read_dir(&dir) {
        Ok(rd) => rd,
        Err(_) => return None,
    };
    for entry in read_dir.flatten() {
        let path = entry.path();
        if !entry.file_type().is_ok_and(|ft| ft.is_dir()) {
            continue;
        }
        let Some(id) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        let transcript = path.join("transcript.jsonl");
        if let Ok(meta) = transcript.metadata()
            && let Ok(modified) = meta.modified()
        {
            entries.push((modified, id.to_string()));
        }
    }
    entries.sort_by(|a, b| b.0.cmp(&a.0));
    entries.into_iter().next().map(|(_, id)| id)
}

/// 从 JSONL 文件 replay 出 ChatSession（供 resume 等功能使用）
pub fn load_session(session_id: &str) -> ChatSession {
    let path = SessionPaths::new(session_id).transcript();
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
                SessionEvent::Msg { message, .. } => messages.push(message),
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

/// 从 JSONL 文件按出现顺序读取 `(ChatMessage, timestamp_ms)` 列表。
///
/// 供 teammate / subagent 等独立 transcript 的读取使用：保留时间戳、不做 Clear/Restore 处理。
pub fn read_transcript_with_timestamps(path: &Path) -> Vec<(ChatMessage, u64)> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let mut out: Vec<(ChatMessage, u64)> = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(SessionEvent::Msg {
            message,
            timestamp_ms,
        }) = serde_json::from_str::<SessionEvent>(line)
        {
            out.push((message, timestamp_ms));
        }
    }
    out
}

/// 向任意路径的 JSONL 文件 append 一条事件（append-only；用于 teammate/subagent 独立 transcript）。
pub fn append_event_to_path(path: &Path, event: &SessionEvent) -> bool {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let line = match serde_json::to_string(event) {
        Ok(s) => s,
        Err(_) => return false,
    };
    match fs::OpenOptions::new().create(true).append(true).open(path) {
        Ok(mut file) => writeln!(file, "{}", line).is_ok(),
        Err(_) => false,
    }
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

// ========== 会话元数据 ==========

/// session.json 元数据文件内容（持久化到 `sessions/<id>/session.json`）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetaFile {
    /// 会话 ID
    pub id: String,
    /// 会话标题（首条 user 消息截断）
    #[serde(default)]
    pub title: String,
    /// 消息计数
    pub message_count: usize,
    /// 创建时间戳（epoch seconds）
    pub created_at: u64,
    /// 最后更新时间戳（epoch seconds）
    pub updated_at: u64,
    /// 使用的模型名称
    #[serde(default)]
    pub model: Option<String>,
}

/// 会话元数据（用于会话列表展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub id: String,
    /// 会话标题（从 session.json 读取）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub message_count: usize,
    pub first_message_preview: Option<String>,
    pub updated_at: u64,
}

/// 加载 session.json 元数据（不存在返回 None）
pub fn load_session_meta_file(session_id: &str) -> Option<SessionMetaFile> {
    let path = SessionPaths::new(session_id).meta_file();
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

/// 保存 session.json 元数据
pub fn save_session_meta_file(meta: &SessionMetaFile) -> bool {
    let paths = SessionPaths::new(&meta.id);
    if paths.ensure_dir().is_err() {
        return false;
    }
    match serde_json::to_string_pretty(meta) {
        Ok(json) => fs::write(paths.meta_file(), json).is_ok(),
        Err(_) => false,
    }
}

/// 从 transcript.jsonl 逐行扫描生成元数据（懒生成 / 迁移用）
fn derive_session_meta_from_transcript(session_id: &str) -> Option<SessionMetaFile> {
    let paths = SessionPaths::new(session_id);
    let transcript = paths.transcript();
    let content = fs::read_to_string(&transcript).ok()?;

    let mut message_count: usize = 0;
    let mut first_user_preview: Option<String> = None;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(event) = serde_json::from_str::<SessionEvent>(line) {
            match event {
                SessionEvent::Msg {
                    message: ref msg, ..
                } => {
                    message_count += 1;
                    if first_user_preview.is_none() && msg.role == "user" && !msg.content.is_empty()
                    {
                        first_user_preview =
                            Some(msg.content.chars().take(MESSAGE_PREVIEW_MAX_LEN).collect());
                    }
                }
                SessionEvent::Clear => {
                    message_count = 0;
                    first_user_preview = None;
                }
                SessionEvent::Restore { ref messages } => {
                    message_count = messages.len();
                    first_user_preview = messages
                        .iter()
                        .find(|m| m.role == "user" && !m.content.is_empty())
                        .map(|m| m.content.chars().take(MESSAGE_PREVIEW_MAX_LEN).collect());
                }
            }
        }
    }

    let updated_at = transcript
        .metadata()
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    Some(SessionMetaFile {
        id: session_id.to_string(),
        title: first_user_preview.clone().unwrap_or_default(),
        message_count,
        created_at: updated_at, // 无法回溯精确创建时间，用 mtime 近似
        updated_at,
        model: None,
    })
}

/// 列出所有会话的元数据，按更新时间倒序
///
/// 优先读 `session.json` 元数据文件（O(1)），不存在时 fallback 到逐行扫描
/// `transcript.jsonl` 并懒生成 `session.json`。
pub fn list_sessions() -> Vec<SessionMeta> {
    let dir = sessions_dir();
    let read_dir = match fs::read_dir(&dir) {
        Ok(rd) => rd,
        Err(_) => return Vec::new(),
    };

    let mut ids: Vec<String> = Vec::new();
    for entry in read_dir.flatten() {
        let path = entry.path();
        if !entry.file_type().is_ok_and(|ft| ft.is_dir()) {
            continue;
        }
        let Some(id) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if path.join("transcript.jsonl").exists() {
            ids.push(id.to_string());
        }
    }

    let mut sessions: Vec<SessionMeta> = Vec::new();
    for id in ids {
        // 优先读 session.json
        if let Some(meta_file) = load_session_meta_file(&id) {
            sessions.push(SessionMeta {
                id: meta_file.id,
                title: if meta_file.title.is_empty() {
                    None
                } else {
                    Some(meta_file.title)
                },
                message_count: meta_file.message_count,
                first_message_preview: None,
                updated_at: meta_file.updated_at,
            });
            continue;
        }

        // fallback：逐行扫描 transcript 并懒生成 session.json
        if let Some(derived) = derive_session_meta_from_transcript(&id) {
            let title = if derived.title.is_empty() {
                None
            } else {
                Some(derived.title.clone())
            };
            let preview_for_ui = title.clone();
            let _ = save_session_meta_file(&derived);
            sessions.push(SessionMeta {
                id: derived.id,
                title,
                message_count: derived.message_count,
                first_message_preview: preview_for_ui,
                updated_at: derived.updated_at,
            });
        }
    }
    sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    sessions
}

/// 生成会话 ID（时间戳微秒 + 进程 ID，无需外部依赖）
pub fn generate_session_id() -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros();
    let pid = std::process::id();
    format!("{:x}-{:x}", ts, pid)
}

/// 删除指定 session 目录
pub fn delete_session(session_id: &str) -> bool {
    let paths = SessionPaths::new(session_id);
    let dir = paths.dir().to_path_buf();
    if dir.exists()
        && let Err(e) = fs::remove_dir_all(&dir)
    {
        error!("✖️ 删除 session 目录失败: {}", e);
        return false;
    }
    true
}

// ========== Session 状态持久化类型 ==========

/// Teammate 快照（可序列化，用于 session 持久化）
///
/// 消息历史存储于独立 transcript JSONL：`sessions/<id>/teammates/<sanitized_name>.jsonl`。
/// 本结构只保存元数据 + 未消费的 pending（供将来 RespawnTeammate 灌回）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeammateSnapshotPersist {
    pub name: String,
    pub role: String,
    pub prompt: String,
    pub worktree: bool,
    pub worktree_branch: Option<String>,
    pub inherit_permissions: bool,
    pub status: crate::command::chat::teammate::TeammateStatusPersist,
    /// 未被消费的广播消息（将来 Respawn 时灌回）
    #[serde(default)]
    pub pending_user_messages: Vec<ChatMessage>,
    pub tool_calls_count: usize,
    pub current_tool: Option<String>,
    pub work_done: bool,
}

/// 把任意名字转成文件系统安全的文件名片段（去除非法字符，限长）。
pub fn sanitize_filename(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for c in name.chars() {
        if c.is_whitespace() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
            out.push('_');
        } else {
            out.push(c);
        }
    }
    // 限长 64，避免极端长名
    if out.chars().count() > 64 {
        out = out.chars().take(64).collect();
    }
    if out.is_empty() {
        out.push('_');
    }
    out
}

/// SubAgent 快照（只读历史，用于 session 恢复时展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentSnapshotPersist {
    pub id: String,
    pub description: String,
    pub mode: String,
    pub status: String,
    pub current_tool: Option<String>,
    pub tool_calls_count: usize,
    pub current_round: usize,
    pub started_at_epoch: u64,
    /// 独立 transcript 相对路径（相对于 session 根目录），例：`subagents/sub_0001/transcript.jsonl`
    #[serde(default)]
    pub transcript_file: String,
}

/// Plan 状态快照（可序列化，用于 session 持久化）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStatePersist {
    pub active: bool,
    pub plan_file_path: Option<String>,
    pub plan_content: Option<String>,
}

/// Session Hook 注册快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionHookPersist {
    pub event: crate::command::chat::hook::HookEvent,
    pub definition: crate::command::chat::hook::HookDef,
}

/// Sandbox 额外安全目录快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxStatePersist {
    pub extra_safe_dirs: Vec<PathBuf>,
}

// ========== Session 状态 Save/Load 通用辅助 ==========

/// 通用：将数据序列化为 JSON 写入指定路径
fn save_session_json<T: Serialize + ?Sized>(path: &Path, data: &T) -> bool {
    match serde_json::to_string_pretty(data) {
        Ok(json) => fs::write(path, json).is_ok(),
        Err(_) => false,
    }
}

/// 通用：从指定路径读取 JSON 并反序列化
fn load_session_json<T: serde::de::DeserializeOwned>(path: &Path) -> Option<T> {
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

// ========== Session 状态专用 Save/Load ==========

/// 保存 Teammates 状态
pub fn save_teammates_state(session_id: &str, data: &[TeammateSnapshotPersist]) -> bool {
    let paths = SessionPaths::new(session_id);
    let _ = paths.ensure_dir();
    save_session_json(&paths.teammates_file(), data)
}

/// 加载 Teammates 状态
pub fn load_teammates_state(session_id: &str) -> Option<Vec<TeammateSnapshotPersist>> {
    let paths = SessionPaths::new(session_id);
    load_session_json(&paths.teammates_file())
}

/// 保存 SubAgents 状态
pub fn save_subagents_state(session_id: &str, data: &[SubAgentSnapshotPersist]) -> bool {
    let paths = SessionPaths::new(session_id);
    let _ = paths.ensure_dir();
    save_session_json(&paths.subagents_file(), data)
}

/// 加载 SubAgents 状态
#[allow(dead_code)]
pub fn load_subagents_state(session_id: &str) -> Option<Vec<SubAgentSnapshotPersist>> {
    let paths = SessionPaths::new(session_id);
    load_session_json(&paths.subagents_file())
}

/// 保存 Tasks 状态
pub fn save_tasks_state(
    session_id: &str,
    data: &[crate::command::chat::tools::task::AgentTask],
) -> bool {
    let paths = SessionPaths::new(session_id);
    let _ = paths.ensure_dir();
    save_session_json(&paths.tasks_file(), data)
}

/// 加载 Tasks 状态
pub fn load_tasks_state(
    session_id: &str,
) -> Option<Vec<crate::command::chat::tools::task::AgentTask>> {
    let paths = SessionPaths::new(session_id);
    load_session_json(&paths.tasks_file())
}

/// 保存 Todos 状态
pub fn save_todos_state(
    session_id: &str,
    data: &[crate::command::chat::tools::todo::TodoItem],
) -> bool {
    let paths = SessionPaths::new(session_id);
    let _ = paths.ensure_dir();
    save_session_json(&paths.todos_file(), data)
}

/// 加载 Todos 状态
pub fn load_todos_state(
    session_id: &str,
) -> Option<Vec<crate::command::chat::tools::todo::TodoItem>> {
    let paths = SessionPaths::new(session_id);
    load_session_json(&paths.todos_file())
}

/// 保存 Plan 状态
pub fn save_plan_state(session_id: &str, data: &PlanStatePersist) -> bool {
    let paths = SessionPaths::new(session_id);
    let _ = paths.ensure_dir();
    save_session_json(&paths.plan_file(), data)
}

/// 加载 Plan 状态
pub fn load_plan_state(session_id: &str) -> Option<PlanStatePersist> {
    let paths = SessionPaths::new(session_id);
    load_session_json(&paths.plan_file())
}

/// 保存 InvokedSkills 状态
pub fn save_skills_state(
    session_id: &str,
    data: &std::collections::HashMap<String, crate::command::chat::compact::InvokedSkill>,
) -> bool {
    let paths = SessionPaths::new(session_id);
    let _ = paths.ensure_dir();
    save_session_json(&paths.skills_file(), data)
}

/// 加载 InvokedSkills 状态
pub fn load_skills_state(
    session_id: &str,
) -> Option<std::collections::HashMap<String, crate::command::chat::compact::InvokedSkill>> {
    let paths = SessionPaths::new(session_id);
    load_session_json(&paths.skills_file())
}

/// 保存 Session Hooks 状态
pub fn save_hooks_state(session_id: &str, data: &[SessionHookPersist]) -> bool {
    let paths = SessionPaths::new(session_id);
    let _ = paths.ensure_dir();
    save_session_json(&paths.hooks_file(), data)
}

/// 加载 Session Hooks 状态
pub fn load_hooks_state(session_id: &str) -> Option<Vec<SessionHookPersist>> {
    let paths = SessionPaths::new(session_id);
    load_session_json(&paths.hooks_file())
}

/// 保存 Sandbox 状态
pub fn save_sandbox_state(session_id: &str, data: &SandboxStatePersist) -> bool {
    let paths = SessionPaths::new(session_id);
    let _ = paths.ensure_dir();
    save_session_json(&paths.sandbox_file(), data)
}

/// 加载 Sandbox 状态
pub fn load_sandbox_state(session_id: &str) -> Option<SandboxStatePersist> {
    let paths = SessionPaths::new(session_id);
    load_session_json(&paths.sandbox_file())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    /// 测试互斥：`J_DATA_PATH` 是进程级 env var，测试间必须串行化。
    fn test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    /// 给当前测试创建一个隔离的临时 `J_DATA_PATH` 目录并返回 sessions/ 根路径。
    /// 返回的 guard drop 时会清理。
    struct TempDataDir {
        root: PathBuf,
        prev: Option<String>,
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl TempDataDir {
        fn new() -> Self {
            let lock = test_lock().lock().unwrap_or_else(|e| e.into_inner());
            let pid = std::process::id();
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let root = std::env::temp_dir().join(format!("jcli-storage-test-{}-{}", pid, nanos));
            let _ = fs::create_dir_all(&root);
            let prev = std::env::var("J_DATA_PATH").ok();
            // SAFETY: 测试加锁串行，其他测试线程此刻不会读 env
            unsafe {
                std::env::set_var("J_DATA_PATH", &root);
            }
            Self {
                root,
                prev,
                _lock: lock,
            }
        }
    }

    impl Drop for TempDataDir {
        fn drop(&mut self) {
            // SAFETY: 测试用 Drop，仅在测试串行执行期间恢复 J_DATA_PATH 环境变量；
            // 测试加锁保证此刻无其他线程读写该 env
            unsafe {
                match &self.prev {
                    Some(v) => std::env::set_var("J_DATA_PATH", v),
                    None => std::env::remove_var("J_DATA_PATH"),
                }
            }
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn session_paths_construction() {
        let _tmp = TempDataDir::new();
        let paths = SessionPaths::new("abc");
        assert_eq!(paths.id(), "abc");
        assert_eq!(paths.dir().file_name().unwrap(), "abc");
        assert_eq!(paths.transcript().file_name().unwrap(), "transcript.jsonl");
        assert_eq!(paths.meta_file().file_name().unwrap(), "session.json");
        assert!(paths.transcript().parent().unwrap().ends_with("abc"));
    }

    #[test]
    fn append_event_writes_to_new_layout() {
        let _tmp = TempDataDir::new();
        let paths = SessionPaths::new("append-id");

        let msg = ChatMessage::text("user", "hello".to_string());
        assert!(append_session_event("append-id", &SessionEvent::msg(msg)));

        assert!(paths.transcript().exists());
    }

    #[test]
    fn load_session_round_trip() {
        let _tmp = TempDataDir::new();

        let msg = ChatMessage::text("user", "round trip test");
        assert!(append_session_event("rt-id", &SessionEvent::msg(msg)));

        let session = load_session("rt-id");
        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.messages[0].content, "round trip test");
    }

    #[test]
    fn list_sessions_finds_sessions() {
        let _tmp = TempDataDir::new();

        let paths = SessionPaths::new("ls-test");
        paths.ensure_dir().unwrap();
        let msg = ChatMessage::text("user", "list test");
        let line = serde_json::to_string(&SessionEvent::msg(msg)).unwrap();
        fs::write(paths.transcript(), format!("{}\n", line)).unwrap();

        let metas = list_sessions();
        assert_eq!(metas.len(), 1);
        assert_eq!(metas[0].id, "ls-test");
    }

    #[test]
    fn delete_session_removes_dir() {
        let _tmp = TempDataDir::new();
        let paths = SessionPaths::new("del-id");

        paths.ensure_dir().unwrap();
        fs::write(paths.transcript(), b"").unwrap();

        assert!(delete_session("del-id"));
        assert!(!paths.dir().exists());
    }

    #[test]
    fn session_meta_file_round_trip() {
        let _tmp = TempDataDir::new();
        let meta = SessionMetaFile {
            id: "meta-test".to_string(),
            title: "你好世界".to_string(),
            message_count: 5,
            created_at: 1000,
            updated_at: 2000,
            model: Some("gpt-4o".to_string()),
        };
        assert!(save_session_meta_file(&meta));
        let loaded = load_session_meta_file("meta-test").expect("should load");
        assert_eq!(loaded.id, "meta-test");
        assert_eq!(loaded.title, "你好世界");
        assert_eq!(loaded.message_count, 5);
        assert_eq!(loaded.created_at, 1000);
        assert_eq!(loaded.updated_at, 2000);
        assert_eq!(loaded.model.as_deref(), Some("gpt-4o"));
    }

    #[test]
    fn append_event_updates_meta() {
        let _tmp = TempDataDir::new();
        let msg1 = ChatMessage::text("user", "hello world");
        assert!(append_session_event("meta-upd", &SessionEvent::msg(msg1)));

        let meta = load_session_meta_file("meta-upd").expect("meta should exist");
        assert_eq!(meta.id, "meta-upd");
        assert_eq!(meta.message_count, 1);
        assert_eq!(meta.title, "hello world");
        assert!(meta.updated_at > 0);

        // 追加第二条消息
        let msg2 = ChatMessage::text("assistant", "hi there");
        assert!(append_session_event("meta-upd", &SessionEvent::msg(msg2)));

        let meta2 = load_session_meta_file("meta-upd").expect("meta should exist");
        assert_eq!(meta2.message_count, 2);
        // title 不变（已有首条 user 消息）
        assert_eq!(meta2.title, "hello world");

        // Clear 事件重置 message_count
        assert!(append_session_event("meta-upd", &SessionEvent::Clear));
        let meta3 = load_session_meta_file("meta-upd").expect("meta should exist");
        assert_eq!(meta3.message_count, 0);
    }

    #[test]
    fn list_sessions_lazy_generates_meta() {
        let _tmp = TempDataDir::new();

        // 手工构造一个只有 transcript.jsonl 没有 session.json 的 session
        let paths = SessionPaths::new("lazy-gen");
        paths.ensure_dir().unwrap();
        let msg = ChatMessage::text("user", "lazy generation test");
        let line = serde_json::to_string(&SessionEvent::msg(msg)).unwrap();
        fs::write(paths.transcript(), format!("{}\n", line)).unwrap();

        // session.json 不存在
        assert!(!paths.meta_file().exists());

        // list_sessions 应该懒生成 session.json
        let sessions = list_sessions();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, "lazy-gen");
        assert_eq!(sessions[0].message_count, 1);
        assert_eq!(sessions[0].title.as_deref(), Some("lazy generation test"));

        // session.json 现在应该存在了
        assert!(paths.meta_file().exists());
    }

    #[test]
    fn session_paths_transcripts_dir() {
        let _tmp = TempDataDir::new();
        let paths = SessionPaths::new("tx-test");
        assert!(paths.transcripts_dir().ends_with(".transcripts"));
        assert_eq!(paths.transcripts_dir().parent().unwrap(), paths.dir());
    }
}
