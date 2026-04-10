use super::api::create_openai_client;
use super::constants::{
    COMPACT_KEEP_RECENT, COMPACT_TOKEN_THRESHOLD, MICRO_COMPACT_BYTES_THRESHOLD, ROLE_ASSISTANT,
    ROLE_TOOL,
};
use super::storage::{ChatMessage, ModelProvider, agent_data_dir};
use super::tools::ask::AskTool;
use super::tools::skill::LoadSkillTool;
use super::tools::task::TaskTool;
use super::tools::todo::{TodoReadTool, TodoWriteTool};
use crate::command::chat::tools::agent::AgentTool;
use crate::command::chat::tools::agent_team::AgentTeamTool;
use crate::command::chat::tools::plan::{EnterPlanModeTool, ExitPlanModeTool};
use crate::util::log::{write_error_log, write_info_log};
use async_openai::types::chat::{
    ChatCompletionRequestMessage, ChatCompletionRequestUserMessageArgs,
    CreateChatCompletionRequestArgs,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

/// Context compact 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactConfig {
    /// 是否启用 context compact
    #[serde(default = "default_compact_enabled")]
    pub enabled: bool,
    /// 触发 auto_compact 的 token 阈值
    #[serde(default = "default_token_threshold")]
    pub token_threshold: usize,
    /// micro_compact 保留最近几个 tool result 不替换
    #[serde(default = "default_keep_recent")]
    pub keep_recent: usize,
}

fn default_compact_enabled() -> bool {
    true
}

fn default_token_threshold() -> usize {
    COMPACT_TOKEN_THRESHOLD
}

fn default_keep_recent() -> usize {
    COMPACT_KEEP_RECENT
}

impl Default for CompactConfig {
    fn default() -> Self {
        Self {
            enabled: default_compact_enabled(),
            token_threshold: default_token_threshold(),
            keep_recent: default_keep_recent(),
        }
    }
}

/// 粗略估算 messages 的 token 数（~4 chars per token）
pub fn estimate_tokens(messages: &[ChatMessage]) -> usize {
    serde_json::to_string(messages).unwrap_or_default().len() / 4
}

/// Layer 1: micro_compact - 替换旧 tool result 为占位符，保留最近 keep_recent 个
///
/// 纯内存操作，零 API 成本。
/// 将较早的 role="tool" 消息中内容长度 > MICRO_COMPACT_BYTES_THRESHOLD 的替换为 "[Previous: used {tool_name}]"
pub fn micro_compact(messages: &mut [ChatMessage], keep_recent: usize) {
    // 1. 从 assistant 消息的 tool_calls 构建 tool_call_id → tool_name 映射
    let mut tool_name_map: HashMap<String, String> = HashMap::new();
    for msg in messages.iter() {
        if msg.role == ROLE_ASSISTANT
            && let Some(ref tcs) = msg.tool_calls
        {
            for tc in tcs {
                tool_name_map.insert(tc.id.clone(), tc.name.clone());
            }
        }
    }

    // 2. 找出所有 role="tool" 的消息索引
    let tool_indices: Vec<usize> = messages
        .iter()
        .enumerate()
        .filter(|(_, msg)| msg.role == ROLE_TOOL)
        .map(|(i, _)| i)
        .collect();

    if tool_indices.len() <= keep_recent {
        return;
    }

    // 3. 除最近 keep_recent 个外，content.len() > MICRO_COMPACT_BYTES_THRESHOLD 的替换为占位符
    let to_compact = &tool_indices[..tool_indices.len() - keep_recent];
    let mut compacted_count = 0;
    // 不压缩的 tool 名称（如 LoadSkill 的结果承载完整工作流指令）
    const EXEMPT_TOOLS: &[&str] = &[
        LoadSkillTool::NAME,
        TaskTool::NAME,
        TodoWriteTool::NAME,
        TodoReadTool::NAME,
        EnterPlanModeTool::NAME,
        ExitPlanModeTool::NAME,
        AgentTool::NAME,
        AgentTeamTool::NAME,
        AskTool::NAME,
    ];

    for &idx in to_compact {
        let msg = &messages[idx];
        if msg.content.chars().count() > MICRO_COMPACT_BYTES_THRESHOLD {
            let tool_call_id = msg.tool_call_id.clone().unwrap_or_default();
            let tool_name = tool_name_map
                .get(&tool_call_id)
                .cloned()
                .unwrap_or_else(|| "unknown".to_string());
            if EXEMPT_TOOLS.iter().any(|&t| t == tool_name) {
                continue;
            }
            messages[idx].content = format!("[Previous: used {}]", tool_name);
            compacted_count += 1;
        }
    }

    if compacted_count > 0 {
        write_info_log(
            "micro_compact",
            &format!(
                "压缩了 {} 个旧 tool result（保留最近 {} 个）",
                compacted_count, keep_recent
            ),
        );
    }
}

/// 保存完整 transcript 到 .transcripts/ 目录
fn save_transcript(messages: &[ChatMessage]) -> Option<String> {
    let transcript_dir = agent_data_dir().join("transcripts");
    if let Err(e) = fs::create_dir_all(&transcript_dir) {
        write_error_log(
            "save_transcript",
            &format!("创建 transcripts 目录失败: {}", e),
        );
        return None;
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let path = transcript_dir.join(format!("transcript_{}.jsonl", timestamp));

    let mut content = String::new();
    for msg in messages {
        if let Ok(line) = serde_json::to_string(msg) {
            content.push_str(&line);
            content.push('\n');
        }
    }

    match fs::write(&path, &content) {
        Ok(_) => {
            let path_str = path.display().to_string();
            write_info_log(
                "save_transcript",
                &format!("Transcript saved: {}", path_str),
            );
            Some(path_str)
        }
        Err(e) => {
            write_error_log("save_transcript", &format!("保存 transcript 失败: {}", e));
            None
        }
    }
}

/// Layer 2: auto_compact - 保存 transcript + LLM 摘要 + 替换消息
///
/// 需要调用 LLM（非流式，max_tokens=20000）。
/// 失败时 graceful degradation：log 错误，返回 Err，调用方可继续用原消息。
pub async fn auto_compact(
    messages: &mut Vec<ChatMessage>,
    provider: &ModelProvider,
) -> Result<(), String> {
    // 1. 保存 transcript
    let transcript_path = save_transcript(messages).unwrap_or_else(|| "(unsaved)".to_string());

    // 2. 构建摘要请求
    let conversation_text = serde_json::to_string(messages).unwrap_or_default();
    // 截断到 80000 chars
    let truncated: String = conversation_text.chars().take(80000).collect();

    let summary_prompt = format!(
        "Summarize this conversation for continuity. Include: \
         1) What was accomplished, 2) Current state, 3) Key decisions made. \
         4) If a skill/workflow was actively being followed, preserve its key steps and current progress so the model can continue following it. \
         Be concise but preserve critical details.\n\n{}",
        truncated
    );

    let user_msg = ChatCompletionRequestUserMessageArgs::default()
        .content(summary_prompt.as_str())
        .build()
        .map_err(|e| format!("构建摘要请求消息失败: {}", e))?;

    let request = CreateChatCompletionRequestArgs::default()
        .model(&provider.model)
        .messages(vec![ChatCompletionRequestMessage::User(user_msg)])
        .max_tokens(20000u32)
        .build()
        .map_err(|e| format!("构建摘要请求失败: {}", e))?;

    // 3. 调用 LLM（非流式）
    let client = create_openai_client(provider);
    let response = client
        .chat()
        .create(request)
        .await
        .map_err(|e| format!("auto_compact LLM 请求失败: {}", e))?;

    let summary = response
        .choices
        .first()
        .and_then(|c| c.message.content.clone())
        .unwrap_or_else(|| "(empty summary)".to_string());

    write_info_log(
        "auto_compact",
        &format!("摘要完成，长度: {} chars", summary.len()),
    );

    // 4. 替换 messages 为 [summary_user_msg, understood_assistant_msg]
    messages.clear();
    messages.push(ChatMessage {
        role: "user".to_string(),
        content: format!(
            "[Conversation compressed. Transcript: {}]\n\n{}",
            transcript_path, summary
        ),
        tool_calls: None,
        tool_call_id: None,
        images: None,
    });
    messages.push(ChatMessage {
        role: ROLE_ASSISTANT.to_string(),
        content: "Understood. I have the context from the summary. Continuing.".to_string(),
        tool_calls: None,
        tool_call_id: None,
        images: None,
    });

    // UI 提示：在消息区显示系统消息
    messages.push(ChatMessage {
        role: "system".to_string(),
        content: format!("📦 上下文已压缩 (transcript: {})", transcript_path),
        tool_calls: None,
        tool_call_id: None,
        images: None,
    });

    Ok(())
}
