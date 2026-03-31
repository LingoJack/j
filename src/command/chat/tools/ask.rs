use crate::command::chat::app::{AskOption, AskQuestion, AskRequest};
use crate::command::chat::tools::{Tool, ToolResult};
use serde_json::{Value, json};
use std::sync::{Arc, atomic::AtomicBool, mpsc};

// ========== AskTool ==========

pub struct AskTool {
    /// 发送 ask 请求到主线程
    pub ask_tx: mpsc::Sender<AskRequest>,
}

impl AskTool {
    pub const NAME: &'static str = "Ask";
}

impl Tool for AskTool {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn description(&self) -> &str {
        r#"
        Present structured questions to the user with single-select or multi-select options. Supports 1-4 questions per call, each with 2-4 options.

        When to use: whenever user input is needed, including but not limited to:
        - Asking the user to make a choice or confirm an action
        - Gathering user preferences or configuration
        - Presenting multiple approaches for the user to decide
        - Showing intermediate results and requesting feedback

        Format:
        Each question contains header (short tag), question (full text), options (list), and multi_select (boolean).
        Users can select a preset option or provide free-text input.

        Response format:
        Returns JSON:
        ```json
        {
            "answers": {
                "question text": "selected label or free-text input"
            }
        }
        ```
        For multi-select, multiple labels are comma-separated.
        "#
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "questions": {
                    "type": "array",
                    "description": "List of questions to ask (1-4)",
                    "items": {
                        "type": "object",
                        "properties": {
                            "question": {
                                "type": "string",
                                "description": "Full question text"
                            },
                            "header": {
                                "type": "string",
                                "description": "Short tag (max 12 chars), e.g. 'Auth method'"
                            },
                            "options": {
                                "type": "array",
                                "description": "List of options (2-4)",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "label": {
                                            "type": "string",
                                            "description": "Option display text (1-5 words)"
                                        },
                                        "description": {
                                            "type": "string",
                                            "description": "Option description"
                                        }
                                    },
                                    "required": ["label", "description"]
                                }
                            },
                            "multi_select": {
                                "type": "boolean",
                                "description": "Whether to allow multiple selections (default false)"
                            }
                        },
                        "required": ["question", "header", "options"]
                    }
                }
            },
            "required": ["questions"]
        })
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let parsed: Value = match serde_json::from_str(arguments) {
            Ok(v) => v,
            Err(_) => {
                return ToolResult {
                    output: "参数解析失败".to_string(),
                    is_error: true,
                    images: vec![],
                };
            }
        };

        let questions_val = match parsed.get("questions").and_then(|v| v.as_array()) {
            Some(arr) => arr,
            None => {
                return ToolResult {
                    output: "缺少 questions 参数".to_string(),
                    is_error: true,
                    images: vec![],
                };
            }
        };

        if questions_val.is_empty() || questions_val.len() > 4 {
            return ToolResult {
                output: "questions 数量必须为 1-4 个".to_string(),
                is_error: true,
                images: vec![],
            };
        }

        let mut questions: Vec<AskQuestion> = Vec::new();
        for q_val in questions_val {
            let question = q_val
                .get("question")
                .and_then(|v| v.as_str())
                .unwrap_or("请回答")
                .to_string();
            let header = q_val
                .get("header")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let multi_select = q_val
                .get("multi_select")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let options = q_val
                .get("options")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .map(|o| AskOption {
                            label: o
                                .get("label")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            description: o
                                .get("description")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            if options.len() < 2 || options.len() > 4 {
                return ToolResult {
                    output: format!("问题 '{}' 的选项数量必须为 2-4 个", question),
                    is_error: true,
                    images: vec![],
                };
            }

            questions.push(AskQuestion {
                question,
                header,
                options,
                multi_select,
            });
        }

        // 创建响应 channel
        let (response_tx, response_rx) = mpsc::channel::<String>();

        let ask_request = AskRequest {
            questions,
            response_tx,
        };

        if self.ask_tx.send(ask_request).is_err() {
            return ToolResult {
                output: "无法发送提问请求（主线程可能已退出）".to_string(),
                is_error: true,
                images: vec![],
            };
        }

        // 阻塞等待用户响应
        match response_rx.recv() {
            Ok(response) => ToolResult {
                output: response,
                is_error: false,
                images: vec![],
            },
            Err(_) => ToolResult {
                output: "等待用户响应时连接断开".to_string(),
                is_error: true,
                images: vec![],
            },
        }
    }

    fn requires_confirmation(&self) -> bool {
        false
    }
}
