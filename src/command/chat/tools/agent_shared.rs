use crate::command::chat::api::{build_request_with_tools, create_openai_client};
use crate::command::chat::app::AskRequest;
use crate::command::chat::hook::HookManager;
use crate::command::chat::permission::JcliConfig;
use crate::command::chat::storage::{ChatMessage, ModelProvider, ToolCallItem};
use crate::command::chat::tools::ToolRegistry;
use crate::command::chat::tools::background::BackgroundManager;
use crate::command::chat::tools::task::TaskManager;
use crate::util::log::write_info_log;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};

// ========== AgentToolShared ==========

/// Agent 工具共享字段（AgentTool / AgentTeamTool / CreateTeammateTool 共用）
///
/// 所有字段均为 Arc 引用，Clone 开销极小。
/// 消除了三个 Tool struct 之间逐字段 hand-copy Arc 的重复代码。
#[derive(Clone)]
pub struct AgentToolShared {
    pub background_manager: Arc<BackgroundManager>,
    pub provider: Arc<Mutex<ModelProvider>>,
    pub system_prompt: Arc<Mutex<Option<String>>>,
    pub jcli_config: Arc<JcliConfig>,
    pub hook_manager: Arc<Mutex<HookManager>>,
    pub task_manager: Arc<TaskManager>,
    pub disabled_tools: Arc<Vec<String>>,
}

impl AgentToolShared {
    /// 构建子工具注册表（不含 skills，标准 ask channel）
    ///
    /// 返回未 Arc 包装的 ToolRegistry，调用者可在包装前注册额外工具（如 SendMessage）。
    pub fn build_sub_registry(&self) -> (ToolRegistry, mpsc::Receiver<AskRequest>) {
        let (ask_tx, ask_rx) = mpsc::channel::<AskRequest>();
        let registry = ToolRegistry::new(
            vec![], // 不传 skills
            ask_tx,
            Arc::clone(&self.background_manager),
            Arc::clone(&self.task_manager),
            Arc::clone(&self.hook_manager),
        );
        (registry, ask_rx)
    }
}

// ========== Headless Loop 共享 Helper ==========

/// 创建 tokio runtime 和 OpenAI client
///
/// 供 run_headless_agent_loop 和 run_teammate_loop 共用。
pub fn create_runtime_and_client(
    provider: &ModelProvider,
) -> Result<
    (
        tokio::runtime::Runtime,
        async_openai::Client<async_openai::config::OpenAIConfig>,
    ),
    String,
> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create async runtime: {}", e))?;
    let client = create_openai_client(provider);
    Ok((rt, client))
}

/// 非流式调用 LLM
///
/// 返回第一个 choice 的 message；出错时返回 Err(error_text)。
pub fn call_llm_non_stream(
    rt: &tokio::runtime::Runtime,
    client: &async_openai::Client<async_openai::config::OpenAIConfig>,
    provider: &ModelProvider,
    messages: &[ChatMessage],
    tools: &[async_openai::types::chat::ChatCompletionTools],
    system_prompt: Option<&str>,
) -> Result<async_openai::types::chat::ChatChoice, String> {
    let request = build_request_with_tools(provider, messages, tools.to_vec(), system_prompt)
        .map_err(|e| format!("Failed to build request: {}", e))?;

    let response = rt
        .block_on(async { client.chat().create(request).await })
        .map_err(|e| format!("API request failed: {}", e))?;

    response
        .choices
        .into_iter()
        .next()
        .ok_or_else(|| "[No response from API]".to_string())
}

/// 从 LLM response 的 tool_calls 中提取 ToolCallItem 列表
pub fn extract_tool_items(
    tool_calls: &[async_openai::types::chat::ChatCompletionMessageToolCalls],
) -> Vec<ToolCallItem> {
    tool_calls
        .iter()
        .filter_map(|tc| {
            if let async_openai::types::chat::ChatCompletionMessageToolCalls::Function(f) = tc {
                Some(ToolCallItem {
                    id: f.id.clone(),
                    name: f.function.name.clone(),
                    arguments: f.function.arguments.clone(),
                })
            } else {
                None
            }
        })
        .collect()
}

/// 执行单个工具调用（含权限检查）
///
/// 返回 tool role 的 ChatMessage。
/// - 被拒绝/需要确认时返回拒绝消息
/// - 正常执行时返回工具结果
/// - 被取消时返回 [Cancelled]
pub fn execute_tool_with_permission(
    item: &ToolCallItem,
    registry: &Arc<ToolRegistry>,
    jcli_config: &Arc<JcliConfig>,
    cancelled: &Arc<AtomicBool>,
    log_tag: &str,
    verbose: bool,
) -> ChatMessage {
    if cancelled.load(Ordering::Relaxed) {
        return ChatMessage {
            role: "tool".to_string(),
            content: "[Cancelled]".to_string(),
            tool_calls: None,
            tool_call_id: Some(item.id.clone()),
            images: None,
        };
    }

    // deny 检查
    if jcli_config.is_denied(&item.name, &item.arguments) {
        if verbose {
            write_info_log(log_tag, &format!("Tool denied by deny rule: {}", item.name));
        }
        return ChatMessage {
            role: "tool".to_string(),
            content: format!("Tool '{}' was denied by permission rules.", item.name),
            tool_calls: None,
            tool_call_id: Some(item.id.clone()),
            images: None,
        };
    }

    // 确认检查
    let tool_ref = registry.get(&item.name);
    let requires_confirm = tool_ref.map(|t| t.requires_confirmation()).unwrap_or(false);

    if requires_confirm && !jcli_config.is_allowed(&item.name, &item.arguments) {
        if verbose {
            write_info_log(
                log_tag,
                &format!(
                    "Tool '{}' requires confirmation but not auto-allowed, denying",
                    item.name
                ),
            );
        }
        return ChatMessage {
            role: "tool".to_string(),
            content: format!(
                "Tool '{}' requires user confirmation which is not available in sub-agent mode. \
                 Add a permission rule to allow this tool automatically.",
                item.name
            ),
            tool_calls: None,
            tool_call_id: Some(item.id.clone()),
            images: None,
        };
    }

    if verbose {
        write_info_log(
            log_tag,
            &format!("Executing tool: {} args: {}", item.name, item.arguments),
        );
    }

    let result = registry.execute(&item.name, &item.arguments, cancelled);

    if verbose {
        write_info_log(
            log_tag,
            &format!(
                "Tool result: {} is_error={} len={}",
                item.name,
                result.is_error,
                result.output.len()
            ),
        );
    }

    ChatMessage {
        role: "tool".to_string(),
        content: result.output,
        tool_calls: None,
        tool_call_id: Some(item.id.clone()),
        images: None,
    }
}
