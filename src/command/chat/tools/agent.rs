use crate::command::chat::permission::JcliConfig;
use crate::command::chat::storage::{ChatMessage, ModelProvider};
use crate::command::chat::tools::agent_shared::{
    AgentToolShared, call_llm_non_stream, create_runtime_and_client, execute_tool_with_permission,
    extract_tool_items,
};
use crate::command::chat::tools::{
    Tool, ToolRegistry, ToolResult, parse_tool_args, schema_to_tool_params,
};
use crate::util::log::write_info_log;
use crate::util::safe_lock;
use async_openai::types::chat::ChatCompletionTools;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

/// AgentTool 参数
#[derive(Deserialize, JsonSchema)]
struct AgentParams {
    /// The task for the sub-agent to perform
    prompt: String,
    /// A short (3-5 word) description of the task
    #[serde(default)]
    description: Option<String>,
    /// Set to true to run in background. Returns task_id immediately.
    #[serde(default)]
    run_in_background: bool,
}

// ========== AgentTool ==========

/// Agent 工具：启动子代理执行复杂多步任务
#[allow(dead_code)]
pub struct AgentTool {
    pub shared: AgentToolShared,
}

impl AgentTool {
    pub const NAME: &'static str = "Agent";
}

impl Tool for AgentTool {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn description(&self) -> &str {
        r#"
        Launch a sub-agent to handle complex, multi-step tasks autonomously.
        The sub-agent runs with a fresh context (system prompt + your prompt as user message).
        It can use all tools except Agent (to prevent recursion).

        When NOT to use the Agent tool:
        - If you want to read a specific file path, use Read or Glob instead
        - If you are searching for a specific class/function definition, use Grep or Glob instead
        - If you are searching code within a specific file or 2-3 files, use Read instead

        Usage notes:
        - Always include a short description (3-5 words) summarizing what the agent will do
        - The result returned by the agent is not visible to the user. To show the user the result, send a text message with a concise summary
        - Use foreground (default) when you need the agent's results before proceeding
        - Use background when you have genuinely independent work to do in parallel
        - Clearly tell the agent whether you expect it to write code or just do research (search, file reads, web fetches, etc.)
        - Provide clear, detailed prompts so the agent can work autonomously — explain what you're trying to accomplish, what you've already learned, and give enough context for the agent to make judgment calls
        "#
    }

    fn parameters_schema(&self) -> Value {
        schema_to_tool_params::<AgentParams>()
    }

    fn execute(&self, arguments: &str, cancelled: &Arc<AtomicBool>) -> ToolResult {
        let params: AgentParams = match parse_tool_args(arguments) {
            Ok(p) => p,
            Err(e) => return e,
        };

        let prompt = params.prompt;
        let description = params
            .description
            .unwrap_or_else(|| "sub-agent task".to_string());
        let run_in_background = params.run_in_background;

        // 获取 provider 和 system prompt 的快照
        let provider = safe_lock(&self.shared.provider, "AgentTool::provider").clone();
        let system_prompt =
            safe_lock(&self.shared.system_prompt, "AgentTool::system_prompt").clone();

        // 构建子 registry（排除 "Agent" 工具防递归）
        let (sub_registry, _) = self.shared.build_sub_registry();
        let sub_registry = Arc::new(sub_registry);

        let mut disabled = self.shared.disabled_tools.as_ref().clone();
        disabled.push("Agent".to_string());
        let tools = sub_registry.to_openai_tools_filtered(&disabled);

        let jcli_config = Arc::clone(&self.shared.jcli_config);

        if run_in_background {
            // 后台模式：注册任务并 spawn 线程
            let (task_id, output_buffer) = self.shared.background_manager.spawn_command(
                &format!("Agent: {}", description),
                None,
                0,
            );

            let bg_manager = Arc::clone(&self.shared.background_manager);
            let task_id_clone = task_id.clone();
            let cancelled_clone = Arc::clone(cancelled);

            std::thread::spawn(move || {
                let result = run_headless_agent_loop(
                    provider,
                    system_prompt,
                    prompt,
                    tools,
                    sub_registry,
                    jcli_config,
                    &cancelled_clone,
                );

                // 写入输出缓冲区
                {
                    let mut buf = safe_lock(&output_buffer, "AgentTool::bg_output");
                    buf.push_str(&result);
                }

                bg_manager.complete_task(&task_id_clone, "completed", result);
            });

            ToolResult {
                output: json!({
                    "task_id": task_id,
                    "description": description,
                    "status": "running in background"
                })
                .to_string(),
                is_error: false,
                images: vec![],
            }
        } else {
            // 前台模式：阻塞执行
            let cancelled_clone = Arc::clone(cancelled);
            let result = run_headless_agent_loop(
                provider,
                system_prompt,
                prompt,
                tools,
                sub_registry,
                jcli_config,
                &cancelled_clone,
            );

            ToolResult {
                output: result,
                is_error: false,
                images: vec![],
            }
        }
    }

    fn requires_confirmation(&self) -> bool {
        false
    }
}

// ========== Headless Agent Loop ==========

/// 无 UI 的子代理循环：执行工具调用直到完成或达到限制
///
/// - 不发送 StreamMsg（无 UI 交互）
/// - 需要确认的工具通过 permission 检查：允许则执行，否则返回 "Tool denied"
/// - 返回最终的 assistant 文本
fn run_headless_agent_loop(
    provider: ModelProvider,
    system_prompt: Option<String>,
    prompt: String,
    tools: Vec<ChatCompletionTools>,
    registry: Arc<ToolRegistry>,
    jcli_config: Arc<JcliConfig>,
    cancelled: &Arc<AtomicBool>,
) -> String {
    let max_rounds = 30; // 子代理最大轮数

    let (rt, client) = match create_runtime_and_client(&provider) {
        Ok(pair) => pair,
        Err(e) => return e,
    };

    let mut messages: Vec<ChatMessage> = vec![ChatMessage {
        role: "user".to_string(),
        content: prompt,
        tool_calls: None,
        tool_call_id: None,
        images: None,
    }];

    let mut final_text = String::new();

    for round in 0..max_rounds {
        if cancelled.load(Ordering::Relaxed) {
            return format!("{}\n[Sub-agent cancelled]", final_text);
        }

        write_info_log("SubAgent", &format!("Round {}/{}", round + 1, max_rounds));

        let choice = match call_llm_non_stream(
            &rt,
            &client,
            &provider,
            &messages,
            &tools,
            system_prompt.as_deref(),
        ) {
            Ok(c) => c,
            Err(e) => return format!("{}\n{}", final_text, e),
        };

        let assistant_text = choice.message.content.clone().unwrap_or_default();
        if !assistant_text.is_empty() {
            final_text = assistant_text.clone();
            write_info_log("SubAgent", &format!("Reply: {}", &final_text));
        }

        // 检查是否有工具调用
        let is_tool_calls = matches!(
            choice.finish_reason,
            Some(async_openai::types::chat::FinishReason::ToolCalls)
        );

        if !is_tool_calls || choice.message.tool_calls.is_none() {
            break;
        }

        let tool_items = extract_tool_items(choice.message.tool_calls.as_ref().unwrap());
        if tool_items.is_empty() {
            break;
        }

        // 将 assistant 消息（含 tool_calls）加入历史
        messages.push(ChatMessage {
            role: "assistant".to_string(),
            content: assistant_text,
            tool_calls: Some(tool_items.clone()),
            tool_call_id: None,
            images: None,
        });

        // 逐个执行工具
        for item in &tool_items {
            let result_msg = execute_tool_with_permission(
                item,
                &registry,
                &jcli_config,
                cancelled,
                "SubAgent",
                true,
            );
            messages.push(result_msg);
        }
    }

    if final_text.is_empty() {
        "[Sub-agent completed with no text output]".to_string()
    } else {
        final_text
    }
}
