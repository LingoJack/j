use crate::command::chat::app::{AskOption, AskQuestion, AskRequest};
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
        r#"
        向用户提出结构化的选择题，支持单选和多选。每次可提 1-4 个问题，每个问题 2-4 个选项

        何时使用\n\n在任何需要用户介入的场景下都应使用此工具，包括但不限于：
        - 需要用户做出选择或确认操作
        - 需要用户提供偏好或配置
        - 遇到多种方案需要用户决定
        - 需要向用户展示阶段性结果并获取反馈

        格式说明：
        每个问题包含 header（短标签）、question（完整问题）、options（选项列表）和 multi_select（是否多选）。
        用户可以选择预设选项，也可以自由输入

        响应格式：
        返回 JSON：
        ```json
        {
            "answers": {
                "问题文本": "选中的label或自由输入"
            }
        }
        ```
        多选时用逗号分隔多个 label。
        "#
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "questions": {
                    "type": "array",
                    "description": "要提问的问题列表（1-4 个）",
                    "items": {
                        "type": "object",
                        "properties": {
                            "question": {
                                "type": "string",
                                "description": "完整的问题文本"
                            },
                            "header": {
                                "type": "string",
                                "description": "短标签（最多12字符），如 'Auth method'"
                            },
                            "options": {
                                "type": "array",
                                "description": "选项列表（2-4 个）",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "label": {
                                            "type": "string",
                                            "description": "选项显示文本（1-5 个词）"
                                        },
                                        "description": {
                                            "type": "string",
                                            "description": "选项说明"
                                        }
                                    },
                                    "required": ["label", "description"]
                                }
                            },
                            "multi_select": {
                                "type": "boolean",
                                "description": "是否允许多选（默认 false）"
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
                };
            }
        };

        let questions_val = match parsed.get("questions").and_then(|v| v.as_array()) {
            Some(arr) => arr,
            None => {
                return ToolResult {
                    output: "缺少 questions 参数".to_string(),
                    is_error: true,
                };
            }
        };

        if questions_val.is_empty() || questions_val.len() > 4 {
            return ToolResult {
                output: "questions 数量必须为 1-4 个".to_string(),
                is_error: true,
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
