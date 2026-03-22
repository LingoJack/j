use crate::command::chat::tools::{Tool, ToolResult};
use serde_json::{Value, json};
use std::sync::{Arc, atomic::AtomicBool};

/// CompactTool: 让模型可以主动触发对话压缩（Layer 3）
///
/// 实际压缩不在 tool execute 中发生，而是 agent loop 检测到
/// compact tool 被调用后触发 auto_compact。
pub struct CompactTool;

impl Tool for CompactTool {
    fn name(&self) -> &str {
        "Compact"
    }

    fn description(&self) -> &str {
        "Trigger conversation compression to free up context window. \
         Use this when the conversation is getting long and you want to \
         summarize and compress the history to continue working efficiently."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "focus": {
                    "type": "string",
                    "description": "What to preserve in the summary (optional)"
                }
            }
        })
    }

    fn execute(&self, _arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        ToolResult {
            output: "Compression requested.".to_string(),
            is_error: false,
        }
    }

    fn requires_confirmation(&self) -> bool {
        false
    }
}
