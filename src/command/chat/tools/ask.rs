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
        "Ask"
    }

    fn description(&self) -> &str {
        "请求用户介入并等待响应。这是你在当前工具调用循环中与用户交互的唯一方式。\n\n## 何时使用\n\n在任何需要用户介入的场景下都应使用此工具，包括但不限于：\n- 需要用户提供信息、做出选择或确认操作\n- 需要用户在终端中执行某些你无法完成的操作（如登录、授权、手动安装等）\n- 遇到权限不足、环境问题等需要用户协助解决的障碍\n- 需要向用户展示阶段性结果并获取反馈后再继续\n- 任何你无法独立完成、需要人类介入的情况\n\n## 关键：保持上下文\n\n工具调用的结果只在当前循环中可见。当你结束回复后，新的循环将无法看到之前的工具调用结果。因此：\n- 如果你调用了其他工具并需要基于结果与用户讨论，必须在同一循环中使用 ask，不要直接结束回复\n- 如果你需要用户做某事后再继续处理，使用 ask 等待用户完成，这样你仍然保有当前上下文\n- 简而言之：任何需要用户参与才能继续的流程，都通过 ask 来衔接，避免上下文丢失\n\n## 格式\n\n问题内容支持 Markdown 格式，包括图片语法 ![alt](url) 可在终端中渲染图片。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "question": {
                    "type": "string",
                    "description": "要传达给用户的内容：可以是问题、操作指引、需要确认的信息等（支持 Markdown 格式，可使用 ![alt](url) 语法展示图片）"
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
