use crate::command::chat::tools::{PlanDecision, Tool, ToolResult, schema_to_tool_params};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::borrow::Cow;
use std::sync::{Arc, Mutex, atomic::AtomicBool};

/// LoadTool 参数
#[derive(Deserialize, JsonSchema)]
#[allow(dead_code)]
struct LoadToolParams {
    /// 要加载的工具名称
    name: String,
}

/// LoadTool: 让模型可以动态加载 deferred 的工具
///
/// 当工具被设置为 defer 时，它不会出现在初始的工具列表中。
/// 模型可以通过调用 LoadTool 来加载这些工具，加载后该工具在后续轮次中可用。
#[derive(Debug)]
pub struct LoadTool {
    /// 延迟加载的工具列表（加载后从中移除）
    deferred_tools: Arc<Mutex<Vec<String>>>,
}

impl LoadTool {
    pub const NAME: &'static str = "LoadTool";

    /// 静态描述。**不要**在这里读 `self.deferred_tools` —— 一旦读，就违反了
    /// "Tool::description 是廉价静态查询" 的 trait 契约，并会导致与外层持锁
    /// 调用栈的自死锁（参见 notes/锁与不变量.md）。
    ///
    /// "当前可加载的工具列表" 这个动态信息由 `ToolRegistry` 在拼装
    /// system prompt / LLM tool 列表时，由调用方从外部 `deferred: &[String]`
    /// 拼到本工具描述末尾。
    pub const STATIC_DESCRIPTION: &'static str = "Load a deferred tool so it becomes available in subsequent turns. \
         Use this when you need a tool that is not currently available in your tool list. \
         The tool name must match exactly. After loading, the tool will be available in the next turn.";

    pub fn new(deferred_tools: Arc<Mutex<Vec<String>>>) -> Self {
        Self { deferred_tools }
    }
}

impl Tool for LoadTool {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn description(&self) -> Cow<'_, str> {
        Cow::Borrowed(Self::STATIC_DESCRIPTION)
    }

    fn parameters_schema(&self) -> Value {
        schema_to_tool_params::<LoadToolParams>()
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let params: LoadToolParams = match serde_json::from_str(arguments) {
            Ok(p) => p,
            Err(e) => {
                return ToolResult {
                    output: format!("Failed to parse arguments: {e}"),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                };
            }
        };

        let tool_name = params.name.trim().to_string();
        if tool_name.is_empty() {
            return ToolResult {
                output: "Tool name cannot be empty.".to_string(),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            };
        }

        let mut deferred = match self.deferred_tools.lock() {
            Ok(guard) => guard,
            Err(e) => {
                // Mutex poison 不应发生在正常使用中，获取已恢复的数据
                e.into_inner()
            }
        };
        let idx = deferred.iter().position(|n| n == &tool_name);
        match idx {
            Some(i) => {
                deferred.remove(i);
                ToolResult {
                    output: format!(
                        "Tool '{}' has been loaded successfully. It will be available in the next turn.",
                        tool_name
                    ),
                    is_error: false,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                }
            }
            None => {
                // 工具可能已经加载过，或者从未被标记为 deferred
                ToolResult {
                    output: format!(
                        "Tool '{}' is not in the deferred list. It may already be loaded or does not exist.",
                        tool_name
                    ),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                }
            }
        }
    }

    fn requires_confirmation(&self) -> bool {
        false
    }
}
