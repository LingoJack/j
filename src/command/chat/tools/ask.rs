use crate::command::chat::app::{AskQuestion, AskRequest, AskResponse};
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
        "请求用户介入并等待响应。这是你在当前工具调用循环中与用户交互的唯一方式。\n\n## 何时使用\n\n在任何需要用户介入的场景下都应使用此工具，包括但不限于：\n- 需要用户提供信息、做出选择或确认操作\n- 需要用户在终端中执行某些你无法完成的操作（如登录、授权、手动安装等）\n- 遇到权限不足、环境问题等需要用户协助解决的障碍\n- 需要向用户展示阶段性结果并获取反馈后再继续\n- 任何你无法独立完成、需要人类介入的情况\n\n## 关键：保持上下文\n\n工具调用的结果只在当前循环中可见。当你结束回复后，新的循环将无法看到之前的工具调用结果。因此：\n- 如果你调用了其他工具并需要基于结果与用户讨论，必须在同一循环中使用 ask，不要直接结束回复\n- 如果你需要用户做某事后再继续处理，使用 ask 等待用户完成，这样你仍然保有当前上下文\n- 简而言之：任何需要用户参与才能继续的流程，都通过 ask 来衔接，避免上下文丢失\n\n## 结构化问答\n\n使用 questions 参数可以提供结构化选项供用户选择，每次最多 4 个问题，每个问题最多 4 个选项。\n用户始终可以选择 \"Other\" 选项来自由输入。\n\n## 格式\n\n问题内容支持 Markdown 格式，包括图片语法 ![alt](url) 可在终端中渲染图片。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "questions": {
                    "type": "array",
                    "description": "结构化问题列表（最多 4 个问题）",
                    "maxItems": 4,
                    "items": {
                        "type": "object",
                        "properties": {
                            "question": {
                                "type": "string",
                                "description": "问题文本"
                            },
                            "header": {
                                "type": "string",
                                "maxLength": 12,
                                "description": "短标签，显示在问题上方（最多 12 字符）"
                            },
                            "multiSelect": {
                                "type": "boolean",
                                "description": "是否允许多选（默认 false）"
                            },
                            "options": {
                                "type": "array",
                                "description": "选项列表（2-4 个选项）",
                                "minItems": 2,
                                "maxItems": 4,
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "label": {
                                            "type": "string",
                                            "description": "选项显示文本（1-5 个词）"
                                        },
                                        "description": {
                                            "type": "string",
                                            "description": "选项的详细说明"
                                        }
                                    },
                                    "required": ["label", "description"]
                                }
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
        let parsed = serde_json::from_str::<Value>(arguments).ok();

        // 解析 questions 数组
        let questions: Vec<AskQuestion> = parsed
            .as_ref()
            .and_then(|v| v.get("questions").and_then(|q| q.as_array()))
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| serde_json::from_value(item.clone()).ok())
                    .collect()
            })
            .unwrap_or_default();

        if questions.is_empty() {
            return ToolResult {
                output: "questions 参数不能为空".to_string(),
                is_error: true,
            };
        }

        // 创建响应 channel
        let (response_tx, response_rx) = mpsc::channel::<AskResponse>();

        // 发送 ask 请求到主线程
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
            Ok(response) => {
                // 将答案格式化为可读文本
                let output = format_answers(&response);
                ToolResult {
                    output,
                    is_error: false,
                }
            }
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

/// 格式化答案为可读文本
fn format_answers(response: &AskResponse) -> String {
    response
        .answers
        .iter()
        .enumerate()
        .map(|(i, answer)| {
            if let Some(ref custom) = answer.custom_input {
                format!("问题 {}: {}", i + 1, custom)
            } else if !answer.selected.is_empty() {
                format!("问题 {}: {}", i + 1, answer.selected.join(", "))
            } else {
                format!("问题 {}: (未回答)", i + 1)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
