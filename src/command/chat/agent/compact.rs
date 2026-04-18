use super::super::constants::{
    COMPACT_KEEP_RECENT, COMPACT_SKILL_PER_SKILL_TOKEN_BUDGET, COMPACT_SKILL_TOKEN_BUDGET,
    COMPACT_TOKEN_THRESHOLD, MICRO_COMPACT_BYTES_THRESHOLD, ROLE_ASSISTANT, ROLE_TOOL,
};
use super::super::storage::{ChatMessage, ModelProvider, SessionPaths};
use super::super::tools::ask::AskTool;
use super::super::tools::skill::LoadSkillTool;
use super::super::tools::task::TaskTool;
use super::super::tools::todo::{TodoReadTool, TodoWriteTool};
use super::api::create_openai_client;
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
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

// ========== InvokedSkills 追踪 ==========

/// 记录一次技能调用的完整信息（用于 auto_compact 后恢复）
#[derive(Debug, Clone)]
pub struct InvokedSkill {
    /// 技能名称
    pub name: String,
    /// 技能目录路径
    pub dir_path: String,
    /// 完整的解析后内容（含 $ARGUMENTS 替换、references/scripts 列表）
    pub content: String,
    /// 调用时间戳（用于 LRU 排序，最近调用的优先保留）
    pub invoked_at: u64,
}

/// 会话内已调用技能的共享状态（Agent 线程写入，auto_compact 读取）
/// 使用 Arc<Mutex<HashMap>> 以便跨线程共享
pub type InvokedSkillsMap = Arc<Mutex<HashMap<String, InvokedSkill>>>;

/// 创建空的 InvokedSkillsMap
pub fn new_invoked_skills_map() -> InvokedSkillsMap {
    Arc::new(Mutex::new(HashMap::new()))
}

/// 记录一次技能调用（由 LoadSkill 工具执行后调用）
pub fn record_skill_invocation(
    map: &InvokedSkillsMap,
    name: String,
    dir_path: String,
    content: String,
) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    if let Ok(mut skills) = map.lock() {
        let log_name = name.clone();
        skills.insert(
            name.clone(),
            InvokedSkill {
                name,
                dir_path,
                content,
                invoked_at: now,
            },
        );
        write_info_log("invoked_skills", &format!("记录技能调用: {}", log_name));
    }
}

/// 构建 auto_compact 后需恢复的技能附件内容
/// 按最近调用时间排序，总预算 COMPACT_SKILL_TOKEN_BUDGET tokens，
/// 每个技能截断到 COMPACT_SKILL_PER_SKILL_TOKEN_BUDGET tokens
pub fn build_invoked_skills_attachment(map: &InvokedSkillsMap) -> Option<String> {
    let skills = map.lock().ok()?;
    if skills.is_empty() {
        return None;
    }

    // 按最近调用时间排序（新→旧）
    let mut sorted: Vec<&InvokedSkill> = skills.values().collect();
    sorted.sort_by(|a, b| b.invoked_at.cmp(&a.invoked_at));

    let mut result =
        String::from("Skills invoked in this session (preserved across compaction):\n\n");
    let mut total_tokens = 0usize;
    let per_skill_budget = COMPACT_SKILL_PER_SKILL_TOKEN_BUDGET;
    let total_budget = COMPACT_SKILL_TOKEN_BUDGET;

    for skill in sorted {
        let skill_tokens = skill.content.len() / 4; // 粗略估算
        let available = if total_tokens + per_skill_budget > total_budget {
            total_budget.saturating_sub(total_tokens)
        } else {
            per_skill_budget
        };
        if available == 0 {
            break;
        }

        result.push_str(&format!("### Skill: {}\n", skill.name));
        result.push_str(&format!("Path: {}\n", skill.dir_path));

        if skill_tokens <= available {
            result.push_str(&skill.content);
            total_tokens += skill_tokens;
        } else {
            // 截断到 available tokens (~4 chars/token)，保留头部（通常包含最关键的使用说明）
            let truncation_point = available * 4;
            let truncated: String = skill.content.chars().take(truncation_point).collect();
            result.push_str(&truncated);
            result.push_str("\n\n[... skill content truncated for compaction ...]");
            total_tokens += available;
        }
        result.push_str("\n\n---\n\n");
    }

    Some(result)
}

// ========== Compact 配置 ==========

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
    /// micro_compact 中不压缩的工具名称列表（用户可扩展，与内置 EXEMPT_TOOLS 合并）
    #[serde(default)]
    pub micro_compact_exempt_tools: Vec<String>,
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
            micro_compact_exempt_tools: Vec::new(),
        }
    }
}

/// 粗略估算 messages 的 token 数（~4 chars per token）
pub fn estimate_tokens(messages: &[ChatMessage]) -> usize {
    serde_json::to_string(messages).unwrap_or_default().len() / 4
}

/// 内置豁免工具列表（不可配置，始终不压缩这些工具的返回结果）
pub const BUILTIN_EXEMPT_TOOLS: &[&str] = &[
    LoadSkillTool::NAME,
    TaskTool::NAME,
    TodoWriteTool::NAME,
    TodoReadTool::NAME,
    EnterPlanModeTool::NAME,
    ExitPlanModeTool::NAME,
    AgentTool::NAME,
    AgentTeamTool::NAME,
    AskTool::NAME,
    // Teammate 工具结果不压缩（承载协作上下文）
    crate::command::chat::tools::send_message::SendMessageTool::NAME,
    crate::command::chat::tools::create_teammate::CreateTeammateTool::NAME,
];

/// 判断工具名是否应被豁免（合并内置 + 用户扩展清单）
pub fn is_exempt_tool(tool_name: &str, extra_exempt_tools: &[String]) -> bool {
    BUILTIN_EXEMPT_TOOLS.contains(&tool_name) || extra_exempt_tools.iter().any(|t| t == tool_name)
}

/// Layer 1: micro_compact - 替换旧 tool result 为占位符，保留最近 keep_recent 个
///
/// 纯内存操作，零 API 成本。
/// 将较早的 role="tool" 消息中内容长度 > MICRO_COMPACT_BYTES_THRESHOLD 的替换为 "[Previous: used {tool_name}]"
pub fn micro_compact(
    messages: &mut [ChatMessage],
    keep_recent: usize,
    extra_exempt_tools: &[String],
) {
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

    for &idx in to_compact {
        let msg = &messages[idx];
        if msg.content.chars().count() > MICRO_COMPACT_BYTES_THRESHOLD {
            let tool_call_id = msg.tool_call_id.clone().unwrap_or_default();
            let tool_name = tool_name_map
                .get(&tool_call_id)
                .cloned()
                .unwrap_or_else(|| "unknown".to_string());
            if is_exempt_tool(&tool_name, extra_exempt_tools) {
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

/// 保存完整 transcript 到 `sessions/<id>/.transcripts/` 目录
fn save_transcript(messages: &[ChatMessage], session_id: &str) -> Option<String> {
    let paths = SessionPaths::new(session_id);
    let transcript_dir = paths.transcripts_dir();
    if let Err(e) = fs::create_dir_all(&transcript_dir) {
        write_error_log(
            "save_transcript",
            &format!("创建 .transcripts 目录失败: {}", e),
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
///
/// `invoked_skills`: 会话内已调用技能的共享状态，auto_compact 后将技能指令作为附件重新注入，
/// 确保模型在压缩后仍能遵循正在执行的技能/工作流。
pub async fn auto_compact(
    messages: &mut Vec<ChatMessage>,
    provider: &ModelProvider,
    invoked_skills: &InvokedSkillsMap,
    session_id: &str,
) -> Result<(), String> {
    // 1. 保存 transcript 到 session 级 .transcripts/ 目录
    let transcript_path =
        save_transcript(messages, session_id).unwrap_or_else(|| "(unsaved)".to_string());

    // 2. 构建结构化摘要请求（9 段式模板，确保技能/工作流进度被保留）
    let conversation_text = serde_json::to_string(messages).unwrap_or_default();
    // 截断到 80000 chars
    let truncated: String = conversation_text.chars().take(80000).collect();

    let summary_prompt = format!(
        "Summarize this conversation for continuity. Use this structured format:\n\
         1) **Primary Request**: What the user originally asked for.\n\
         2) **Key Concepts**: Important technical concepts, domain knowledge, or constraints discovered.\n\
         3) **Files and Code**: Key files read or modified, with important code snippets or decisions.\n\
         4) **Errors and Fixes**: Any errors encountered and how they were resolved.\n\
         5) **Problem Solving**: Reasoning steps and approach taken.\n\
         6) **Active Skills/Workflows**: If a skill or workflow was being followed, list its name, key steps, and current progress. Include direct quotes showing exactly where you left off.\n\
         7) **Pending Tasks**: Things that still need to be done.\n\
         8) **Current Work**: What was being worked on most recently. Include direct quotes from the most recent conversation showing exactly what task you were working on and where you left off.\n\
         9) **Next Step**: What should happen next to continue the work.\n\
         \n\
         Be concise but preserve critical details. Section 6 (Active Skills/Workflows) is especially important — preserve all skill instructions and progress so the model can continue following them without re-loading.\n\n\
         {}",
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
    //    如果有已调用技能，将技能指令作为附件重新注入，确保压缩后仍可遵循
    messages.clear();
    let mut summary_content = format!(
        "[Conversation compressed. Transcript: {}]\n\n{}",
        transcript_path, summary
    );

    // 注入已调用技能附件（结构化保留，类似 Claude Code 的 invoked_skills 机制）
    if let Some(skills_attachment) = build_invoked_skills_attachment(invoked_skills) {
        summary_content.push_str(&format!(
            "\n\n<system-reminder>\n{}\n</system-reminder>",
            skills_attachment
        ));
        write_info_log(
            "auto_compact",
            "已注入 invoked_skills 附件，确保压缩后技能指令可继续遵循",
        );
    }

    messages.push(ChatMessage {
        role: "user".to_string(),
        content: summary_content,
        tool_calls: None,
        tool_call_id: None,
        images: None,
    });
    messages.push(ChatMessage {
        role: ROLE_ASSISTANT.to_string(),
        content: "Understood. I have the context from the summary and any active skill instructions. Continuing to follow them.".to_string(),
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
