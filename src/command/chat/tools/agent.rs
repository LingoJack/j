use crate::command::chat::api::{build_request_with_tools, create_openai_client};
use crate::command::chat::compact::CompactConfig;
use crate::command::chat::hook::HookManager;
use crate::command::chat::permission::JcliConfig;
use crate::command::chat::storage::{ChatMessage, ModelProvider, ToolCallItem};
use crate::command::chat::tools::background::BackgroundManager;
use crate::command::chat::tools::task::TaskManager;
use crate::command::chat::tools::todo::TodoManager;
use crate::command::chat::tools::{Tool, ToolRegistry, ToolResult};
use crate::util::log::{write_error_log, write_info_log};
use crate::util::safe_lock;
use async_openai::types::chat::ChatCompletionTools;
use serde_json::{Value, json};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};

// ========== AgentTool ==========

/// Agent 工具：启动子代理执行复杂多步任务
#[allow(dead_code)]
pub struct AgentTool {
    pub background_manager: Arc<BackgroundManager>,
    pub provider: Arc<Mutex<ModelProvider>>,
    pub system_prompt: Arc<Mutex<Option<String>>>,
    pub jcli_config: Arc<JcliConfig>,
    pub compact_config: CompactConfig,
    /// 构建子 registry 所需的共享组件
    pub hook_manager: Arc<Mutex<HookManager>>,
    pub task_manager: Arc<TaskManager>,
    pub todo_manager: Arc<TodoManager>,
    /// 禁用的工具列表
    pub disabled_tools: Arc<Vec<String>>,
}

impl Tool for AgentTool {
    fn name(&self) -> &str {
        "Agent"
    }

    fn description(&self) -> &str {
        r#"
        Launch a sub-agent to handle complex, multi-step tasks autonomously.
        The sub-agent runs with a fresh context (system prompt + your prompt as user message).
        It can use all tools except Agent (to prevent recursion).

        Parameters:
        - prompt (required): The task for the sub-agent to perform
        - description (optional): A short (3-5 word) description of the task
        - run_in_background (optional): Set to true to run in background, returns task_id immediately

        The sub-agent will execute tools directly without user confirmation (subject to permission rules).
        Use foreground (default) when you need results before proceeding.
        Use background when you have independent work to do in parallel.
        "#
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "The task for the sub-agent to perform"
                },
                "description": {
                    "type": "string",
                    "description": "A short (3-5 word) description of the task"
                },
                "run_in_background": {
                    "type": "boolean",
                    "description": "Set to true to run in background. Returns task_id immediately."
                }
            },
            "required": ["prompt"]
        })
    }

    fn execute(&self, arguments: &str, cancelled: &Arc<AtomicBool>) -> ToolResult {
        let parsed: Value = match serde_json::from_str(arguments) {
            Ok(v) => v,
            Err(e) => {
                return ToolResult {
                    output: format!("参数解析失败: {}", e),
                    is_error: true,
                    images: vec![],
                };
            }
        };

        let prompt = match parsed.get("prompt").and_then(|v| v.as_str()) {
            Some(p) => p.to_string(),
            None => {
                return ToolResult {
                    output: "缺少 prompt 参数".to_string(),
                    is_error: true,
                    images: vec![],
                };
            }
        };

        let description = parsed
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("sub-agent task")
            .to_string();

        let run_in_background = parsed
            .get("run_in_background")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // 获取 provider 和 system prompt 的快照
        let provider = safe_lock(&self.provider, "AgentTool::provider").clone();
        let system_prompt = safe_lock(&self.system_prompt, "AgentTool::system_prompt").clone();

        // 构建子 registry（排除 "Agent" 工具防递归）
        let (ask_tx, _ask_rx) = mpsc::channel::<crate::command::chat::app::AskRequest>();
        let sub_registry = ToolRegistry::new(
            vec![], // 不传 skills
            ask_tx,
            Arc::clone(&self.background_manager),
            Arc::clone(&self.task_manager),
            Arc::clone(&self.hook_manager),
        );
        let sub_registry = Arc::new(sub_registry);

        // 构建工具定义列表（排除 Agent）
        let mut disabled = self.disabled_tools.as_ref().clone();
        disabled.push("Agent".to_string());
        let tools = sub_registry.to_openai_tools_filtered(&disabled);

        let jcli_config = Arc::clone(&self.jcli_config);

        if run_in_background {
            // 后台模式：注册任务并 spawn 线程
            let (task_id, output_buffer) =
                self.background_manager
                    .spawn_command(&format!("Agent: {}", description), None, 0);

            let bg_manager = Arc::clone(&self.background_manager);
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

    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            return format!("Failed to create async runtime: {}", e);
        }
    };

    let client = create_openai_client(&provider);

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

        let request = match build_request_with_tools(
            &provider,
            &messages,
            tools.clone(),
            system_prompt.as_deref(),
        ) {
            Ok(req) => req,
            Err(e) => {
                return format!("Failed to build request: {}", e);
            }
        };

        // 使用非流式请求（子代理无需流式输出）
        let response = rt.block_on(async {
            let chat_client = client.chat();
            chat_client.create(request).await
        });

        let response = match response {
            Ok(r) => r,
            Err(e) => {
                let err = format!("API request failed: {}", e);
                write_error_log("SubAgent", &err);
                return format!("{}\n{}", final_text, err);
            }
        };

        let choice = match response.choices.first() {
            Some(c) => c,
            None => {
                return format!("{}\n[No response from API]", final_text);
            }
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
            // 正常结束
            break;
        }

        let tool_calls = choice.message.tool_calls.as_ref().unwrap();
        let tool_items: Vec<ToolCallItem> = tool_calls
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
            .collect();

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
            if cancelled.load(Ordering::Relaxed) {
                messages.push(ChatMessage {
                    role: "tool".to_string(),
                    content: "[Cancelled]".to_string(),
                    tool_calls: None,
                    tool_call_id: Some(item.id.clone()),
                    images: None,
                });
                continue;
            }

            // 检查 deny 规则
            if jcli_config.is_denied(&item.name, &item.arguments) {
                write_info_log(
                    "SubAgent",
                    &format!("Tool denied by deny rule: {}", item.name),
                );
                messages.push(ChatMessage {
                    role: "tool".to_string(),
                    content: format!("Tool '{}' was denied by permission rules.", item.name),
                    tool_calls: None,
                    tool_call_id: Some(item.id.clone()),
                    images: None,
                });
                continue;
            }

            // 需要确认的工具：检查 permission allow 列表
            let tool_ref = registry.get(&item.name);
            let requires_confirm = tool_ref.map(|t| t.requires_confirmation()).unwrap_or(false);

            if requires_confirm && !jcli_config.is_allowed(&item.name, &item.arguments) {
                write_info_log(
                    "SubAgent",
                    &format!(
                        "Tool '{}' requires confirmation but not auto-allowed, denying in sub-agent",
                        item.name
                    ),
                );
                messages.push(ChatMessage {
                    role: "tool".to_string(),
                    content: format!(
                        "Tool '{}' requires user confirmation which is not available in sub-agent mode. \
                         Add a permission rule to allow this tool automatically.",
                        item.name
                    ),
                    tool_calls: None,
                    tool_call_id: Some(item.id.clone()),
                    images: None,
                });
                continue;
            }

            write_info_log(
                "SubAgent",
                &format!("Executing tool: {} args: {}", item.name, item.arguments),
            );

            let result = registry.execute(&item.name, &item.arguments, cancelled);

            write_info_log(
                "SubAgent",
                &format!(
                    "Tool result: {} is_error={} len={}",
                    item.name,
                    result.is_error,
                    result.output.len()
                ),
            );

            messages.push(ChatMessage {
                role: "tool".to_string(),
                content: result.output,
                tool_calls: None,
                tool_call_id: Some(item.id.clone()),
                images: None,
            });
        }
    }

    if final_text.is_empty() {
        "[Sub-agent completed with no text output]".to_string()
    } else {
        final_text
    }
}
