use crate::tools::{PlanDecision, Tool, ToolResult, schema_to_tool_params};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::borrow::Cow;
use std::sync::{Arc, atomic::AtomicBool};

/// CompactTool 参数
#[derive(Deserialize, JsonSchema)]
#[allow(dead_code)]
struct CompactParams {
    /// What to preserve in the summary (optional)
    #[serde(default)]
    focus: Option<String>,
}

/// CompactTool: 让模型可以主动触发对话压缩（Layer 3）
///
#[derive(Debug)]
pub struct CompactTool;

impl CompactTool {
    pub const NAME: &'static str = "Compact";
}

impl Tool for CompactTool {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn description(&self) -> Cow<'_, str> {
        r#"
        Trigger conversation compression to free up context window.
        Use when 
        - the conversation is getting long and you want to summarize and compress the history to continue working efficiently.
        - when you try to solve a issue but fail too many time, this tool help you free down your mind and help you solve clearly.
        "#.into()
    }

    fn parameters_schema(&self) -> Value {
        schema_to_tool_params::<CompactParams>()
    }

    fn execute(&self, _arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        ToolResult {
            output: "Compression requested.".to_string(),
            is_error: false,
            images: vec![],
            plan_decision: PlanDecision::None,
        }
    }

    fn requires_confirmation(&self) -> bool {
        false
    }
}
