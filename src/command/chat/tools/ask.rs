use crate::command::chat::app::AskRequest;
use crate::command::chat::tools::{Tool, ToolResult};
use serde_json::{Value, json};
use std::sync::{Arc, atomic::AtomicBool, mpsc};

// ========== AskTool ==========

pub struct AskTool {
    /// 发送 ask 请求到主线程
    pub ask_tx: mpsc::Sender<AskRequest>,
}

impl Tool for AskTool {
    fn name(&self) -> &str {
        "ask"
    }

    fn description(&self) -> &str {
        "向用户提出问题并等待回答。当你需要用户提供更多信息、确认操作或做出选择时使用此工具。问题内容支持 Markdown 格式。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "question": {
                    "type": "string",
                    "description": "要向用户提出的问题（支持 Markdown 格式）"
                }
            },
            "required": ["question"]
        })
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let parsed = serde_json::from_str::<Value>(arguments).ok();

        let question = parsed
            .as_ref()
            .and_then(|v| v.get("question").and_then(|q| q.as_str()))
            .unwrap_or("请回答");

        // 创建响应 channel
        let (response_tx, response_rx) = mpsc::channel::<String>();

        // 发送 ask 请求到主线程
        let ask_request = AskRequest {
            question: question.to_string(),
            response_tx,
        };

        if self.ask_tx.send(ask_request).is_err() {
            return ToolResult {
                output: "无法发送提问请求（主线程可能已退出）".to_string(),
                is_error: true,
            };
        }

        // 阻塞等待用户响应
        match response_rx.recv() {
            Ok(response) => ToolResult {
                output: response,
                is_error: false,
            },
            Err(_) => ToolResult {
                output: "等待用户响应时连接断开".to_string(),
                is_error: true,
            },
        }
    }

    fn requires_confirmation(&self) -> bool {
        false
    }
}
