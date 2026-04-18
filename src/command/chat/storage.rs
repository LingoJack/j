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

/// 获取单个 session 的 JSONL 文件路径（兼容别名，指向新布局主文件）
pub fn session_file_path(session_id: &str) -> PathBuf {
    SessionPaths::new(session_id).transcript()
}

/// Session 目录布局抽象。
///
/// 新布局：`sessions/<id>/transcript.jsonl`。
/// 老扁平布局 `sessions/<id>.jsonl` 仍可通过 [`SessionPaths::legacy_flat`] 读取，
/// 迁移完成后应当不存在。
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

    /// 老扁平路径：`sessions/<id>.jsonl`（仅用于读 fallback + 迁移检测）
    pub fn legacy_flat(&self) -> PathBuf {
        sessions_dir().join(format!("{}.jsonl", self.id))
    }

    /// 读取时实际应使用的 transcript 文件：
    /// - 新布局存在 → 新
    /// - 否则老扁平存在 → 老
    /// - 都不存在 → 新（作为后续写入目标）
    pub fn resolve_for_read(&self) -> PathBuf {
        let new_path = self.transcript();
        if new_path.exists() {
            return new_path;
        }
        let legacy = self.legacy_flat();
        if legacy.exists() {
            return legacy;
        }
        new_path
    }

    pub fn ensure_dir(&self) -> std::io::Result<()> {
        fs::create_dir_all(&self.dir)
    }
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
///
/// 写入永远走新布局 `sessions/<id>/transcript.jsonl`。若老扁平文件仍存在
/// （迁移中断等异常情况），这里不追加到老文件，避免分叉。
pub fn append_session_event(session_id: &str, event: &SessionEvent) -> bool {
    let paths = SessionPaths::new(session_id);
    if paths.ensure_dir().is_err() {
        return false;
    }
    let path = paths.transcript();
    match serde_json::to_string(event) {
        Ok(line) => match fs::OpenOptions::new().create(true).append(true).open(&path) {
            Ok(mut file) => writeln!(file, "{}", line).is_ok(),
            Err(_) => false,
        },
        Err(_) => false,
    }
}

/// 查找最近修改的 session ID（用于 --continue）
///
/// 同时枚举新布局（`sessions/<id>/transcript.jsonl`）和老扁平布局
/// （`sessions/<id>.jsonl`），按各自的 mtime 排序。
pub fn find_latest_session_id() -> Option<String> {
    let dir = sessions_dir();
    let mut entries: Vec<(std::time::SystemTime, String)> = Vec::new();
    let read_dir = match fs::read_dir(&dir) {
        Ok(rd) => rd,
        Err(_) => return None,
    };
    for entry in read_dir.flatten() {
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        if file_type.is_dir() {
            // 新布局：<id>/transcript.jsonl 的 mtime 代表会话的最后活动
            let Some(id) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            let transcript = path.join("transcript.jsonl");
            if let Ok(meta) = transcript.metadata()
                && let Ok(modified) = meta.modified()
            {
                entries.push((modified, id.to_string()));
            }
        } else if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
            // 老扁平布局
            if let (Ok(meta), Some(stem)) =
                (path.metadata(), path.file_stem().and_then(|s| s.to_str()))
                && let Ok(modified) = meta.modified()
            {
                entries.push((modified, stem.to_string()));
            }
        }
    }
    entries.sort_by(|a, b| b.0.cmp(&a.0));
    entries.into_iter().next().map(|(_, id)| id)
}

/// 修复历史消息中 tool_call_id 配对不完整的问题（旧格式兼容）。
///
/// 旧版本在序列化时可能遗漏了 role="tool" 消息的 tool_call_id 字段，
/// 或将 assistant tool_calls[].id 存为空字符串。
/// 此函数通过位置对应关系（assistant tool_calls 与后续 tool results 一一对应）
/// 修复这些配对，使消息序列满足 OpenAI API 要求。
fn repair_tool_call_ids(messages: &mut [ChatMessage]) {
    use rand::Rng;
    let mut i = 0;
    while i < messages.len() {
        let has_tool_calls = messages[i].role == "assistant"
            && messages[i]
                .tool_calls
                .as_ref()
                .is_some_and(|tc| !tc.is_empty());
        if !has_tool_calls {
            i += 1;
            continue;
        }
        let call_count = messages[i].tool_calls.as_ref().map_or(0, |tc| tc.len());

        // 收集紧跟在后面的 role="tool" 消息索引
        let result_start = i + 1;
        let mut result_end = result_start;
        while result_end < messages.len() && messages[result_end].role == "tool" {
            result_end += 1;
        }
        let result_count = result_end - result_start;

        // 只在数量完全匹配时做位置对应修复（数量不匹配交由 sanitize_messages 处理）
        if result_count == call_count {
            for k in 0..call_count {
                let result_idx = result_start + k;
                let call_id = messages[i].tool_calls.as_ref().unwrap()[k].id.clone();
                let result_id = messages[result_idx]
                    .tool_call_id
                    .clone()
                    .unwrap_or_default();

                match (call_id.is_empty(), result_id.is_empty()) {
                    (true, true) => {
                        // 两端都没有 ID → 生成随机 ID，保证双方一致
                        let new_id = format!("call_{:016x}", rand::thread_rng().r#gen::<u64>());
                        messages[i].tool_calls.as_mut().unwrap()[k].id = new_id.clone();
                        messages[result_idx].tool_call_id = Some(new_id);
                    }
                    (true, false) => {
                        // assistant 侧缺 ID，以 result 侧为准
                        messages[i].tool_calls.as_mut().unwrap()[k].id = result_id;
                    }
                    (false, true) => {
                        // result 侧缺 ID，以 assistant 侧为准
                        messages[result_idx].tool_call_id = Some(call_id);
                    }
                    (false, false) if call_id != result_id => {
                        // ID 不一致（异常情况），以 assistant 侧为准
                        messages[result_idx].tool_call_id = Some(call_id);
                    }
                    _ => {} // 两端 ID 一致，无需处理
                }
            }
        }

        i = result_end; // 跳过已处理的 tool result 消息
    }
}

/// 从 JSONL 文件 replay 出 ChatSession（供 resume 等功能使用）
///
/// 新布局优先；不存在则回退到老扁平文件（迁移未完成场景）。
pub fn load_session(session_id: &str) -> ChatSession {
    let path = SessionPaths::new(session_id).resolve_for_read();
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
    // 修复旧格式中 tool_call_id 配对不完整的消息
    repair_tool_call_ids(&mut messages);
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

// ========== 会话元数据 ==========

/// 会话元数据（用于会话列表展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub id: String,
    pub message_count: usize,
    pub first_message_preview: Option<String>,
    pub updated_at: u64,
}

/// 列出所有会话的元数据，按更新时间倒序
///
/// 混合枚举：新布局 `sessions/<id>/transcript.jsonl` 和老扁平 `sessions/<id>.jsonl`。
/// 同一 id 若同时存在（迁移未完成），以新布局为准（`resolve_for_read` 已保证）。
pub fn list_sessions() -> Vec<SessionMeta> {
    let dir = sessions_dir();
    let read_dir = match fs::read_dir(&dir) {
        Ok(rd) => rd,
        Err(_) => return Vec::new(),
    };

    // 先收集 (id, transcript_path) 列表，去重以新布局为先
    let mut entries: Vec<(String, PathBuf)> = Vec::new();
    let mut seen_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut legacy_entries: Vec<(String, PathBuf)> = Vec::new();
    for entry in read_dir.flatten() {
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        if file_type.is_dir() {
            let Some(id) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            let transcript = path.join("transcript.jsonl");
            if transcript.exists() {
                seen_ids.insert(id.to_string());
                entries.push((id.to_string(), transcript));
            }
        } else if path.extension().and_then(|e| e.to_str()) == Some("jsonl")
            && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
        {
            legacy_entries.push((stem.to_string(), path));
        }
    }
    for (id, path) in legacy_entries {
        if !seen_ids.contains(&id) {
            entries.push((id, path));
        }
    }

    let mut sessions: Vec<SessionMeta> = Vec::new();
    for (id, path) in entries {
        let updated_at = path
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue, // 损坏的文件跳过
        };

        let mut message_count: usize = 0;
        let mut first_user_preview: Option<String> = None;
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(event) = serde_json::from_str::<SessionEvent>(line) {
                match event {
                    SessionEvent::Msg(ref msg) => {
                        message_count += 1;
                        if first_user_preview.is_none()
                            && msg.role == "user"
                            && !msg.content.is_empty()
                        {
                            let preview: String =
                                msg.content.chars().take(MESSAGE_PREVIEW_MAX_LEN).collect();
                            first_user_preview = Some(preview);
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

        sessions.push(SessionMeta {
            id,
            message_count,
            first_message_preview: first_user_preview,
            updated_at,
        });
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

/// 删除指定 session（新布局整目录 + 顺手清理残留老扁平文件）
pub fn delete_session(session_id: &str) -> bool {
    let paths = SessionPaths::new(session_id);
    let mut ok = true;

    let dir = paths.dir().to_path_buf();
    if dir.exists()
        && let Err(e) = fs::remove_dir_all(&dir)
    {
        error!("✖️ 删除 session 目录失败: {}", e);
        ok = false;
    }

    let legacy = paths.legacy_flat();
    if legacy.exists()
        && let Err(e) = fs::remove_file(&legacy)
        && e.kind() != std::io::ErrorKind::NotFound
    {
        error!("✖️ 删除老 session 文件失败: {}", e);
        ok = false;
    }

    ok
}

/// 一次性迁移：`sessions/<id>.jsonl` → `sessions/<id>/transcript.jsonl`。
///
/// 幂等：已存在新 transcript 时跳过并清理残留老文件；失败只 log 不 panic。
/// 返回 (迁移数, 错误数)。
pub fn migrate_flat_sessions_to_nested() -> (usize, usize) {
    let dir = sessions_dir();
    let read_dir = match fs::read_dir(&dir) {
        Ok(rd) => rd,
        Err(_) => return (0, 0),
    };
    let mut migrated = 0usize;
    let mut errors = 0usize;
    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let paths = SessionPaths::new(stem);

        // 幂等：新 transcript 已存在 → 把残留的老文件删掉就好
        if paths.transcript().exists() {
            let _ = fs::remove_file(&path);
            continue;
        }

        if let Err(e) = paths.ensure_dir() {
            error!("✖️ 迁移 session {} 失败（mkdir）: {}", stem, e);
            errors += 1;
            continue;
        }

        // 同分区下 rename 是原子的；跨分区走 copy + remove 兜底
        match fs::rename(&path, paths.transcript()) {
            Ok(_) => {
                migrated += 1;
            }
            Err(rename_err) => match fs::copy(&path, paths.transcript()) {
                Ok(_) => {
                    let _ = fs::remove_file(&path);
                    migrated += 1;
                }
                Err(copy_err) => {
                    error!(
                        "✖️ 迁移 session {} 失败: rename={}, copy={}",
                        stem, rename_err, copy_err
                    );
                    errors += 1;
                }
            },
        }
    }
    (migrated, errors)
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
        assert_eq!(paths.legacy_flat().file_name().unwrap(), "abc.jsonl");
        assert!(paths.transcript().parent().unwrap().ends_with("abc"));
    }

    #[test]
    fn resolve_for_read_prefers_new_then_legacy() {
        let _tmp = TempDataDir::new();
        let paths = SessionPaths::new("session1");

        // 两者都不存在 → 返回新路径
        assert_eq!(paths.resolve_for_read(), paths.transcript());

        // 只有老扁平文件 → 返回老路径
        fs::write(paths.legacy_flat(), b"").unwrap();
        assert_eq!(paths.resolve_for_read(), paths.legacy_flat());

        // 新布局出现 → 优先返回新路径
        paths.ensure_dir().unwrap();
        fs::write(paths.transcript(), b"").unwrap();
        assert_eq!(paths.resolve_for_read(), paths.transcript());
    }

    #[test]
    fn migrate_is_idempotent_and_preserves_content() {
        let _tmp = TempDataDir::new();
        let dir = sessions_dir();
        let legacy = dir.join("mig-id.jsonl");
        let msg = ChatMessage::text("user", "你好".to_string());
        let line = serde_json::to_string(&SessionEvent::Msg(msg)).unwrap();
        fs::write(&legacy, format!("{}\n", line)).unwrap();

        let (migrated, errors) = migrate_flat_sessions_to_nested();
        assert_eq!((migrated, errors), (1, 0));
        assert!(!legacy.exists());
        let paths = SessionPaths::new("mig-id");
        assert!(paths.transcript().exists());

        // 第二次跑是 no-op
        let (m2, e2) = migrate_flat_sessions_to_nested();
        assert_eq!((m2, e2), (0, 0));

        // load_session 能完整恢复
        let session = load_session("mig-id");
        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.messages[0].content, "你好");
    }

    #[test]
    fn list_sessions_merges_new_and_legacy() {
        let _tmp = TempDataDir::new();
        let dir = sessions_dir();

        // 老扁平 session
        let legacy = dir.join("old.jsonl");
        let msg_a = ChatMessage::text("user", "老的消息".to_string());
        let line_a = serde_json::to_string(&SessionEvent::Msg(msg_a)).unwrap();
        fs::write(&legacy, format!("{}\n", line_a)).unwrap();

        // 新布局 session（手工构造，绕过 append_session_event 以保证测试独立）
        let new_paths = SessionPaths::new("new");
        new_paths.ensure_dir().unwrap();
        let msg_b = ChatMessage::text("user", "新的消息".to_string());
        let line_b = serde_json::to_string(&SessionEvent::Msg(msg_b)).unwrap();
        fs::write(new_paths.transcript(), format!("{}\n", line_b)).unwrap();

        let metas = list_sessions();
        let ids: std::collections::HashSet<String> = metas.iter().map(|m| m.id.clone()).collect();
        assert!(ids.contains("old"));
        assert!(ids.contains("new"));
        assert_eq!(metas.len(), 2);
    }

    #[test]
    fn append_event_writes_to_new_layout_only() {
        let _tmp = TempDataDir::new();
        let paths = SessionPaths::new("append-id");

        let msg = ChatMessage::text("user", "hello".to_string());
        assert!(append_session_event("append-id", &SessionEvent::Msg(msg)));

        assert!(paths.transcript().exists());
        assert!(!paths.legacy_flat().exists());
    }

    #[test]
    fn delete_session_removes_both_layouts() {
        let _tmp = TempDataDir::new();
        let paths = SessionPaths::new("del-id");

        // 同时放新旧两份残留
        paths.ensure_dir().unwrap();
        fs::write(paths.transcript(), b"").unwrap();
        fs::write(paths.legacy_flat(), b"").unwrap();

        assert!(delete_session("del-id"));
        assert!(!paths.dir().exists());
        assert!(!paths.legacy_flat().exists());
    }
}
