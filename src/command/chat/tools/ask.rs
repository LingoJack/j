use crate::command::chat::app::{AskOption, AskQuestion, AskRequest};
use crate::command::chat::tools::{Tool, ToolResult, parse_tool_args, schema_to_tool_params};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::sync::{Arc, atomic::AtomicBool, mpsc};

/// 选项参数
#[derive(Deserialize, JsonSchema)]
struct AskOptionParam {
    /// Option display text (1-5 words)
    label: String,
    /// Option description
    description: String,
}

/// 问题参数
#[derive(Deserialize, JsonSchema)]
struct AskQuestionParam {
    /// Full question text
    question: String,
    /// Short tag (max 12 chars), e.g. 'Auth method'
    header: String,
    /// List of options (2-4)
    options: Vec<AskOptionParam>,
    /// Whether to allow multiple selections (default false)
    #[serde(default)]
    multi_select: bool,
}

/// AskTool 参数
#[derive(Deserialize, JsonSchema)]
struct AskParams {
    /// List of questions to ask (1-4)
    questions: Vec<AskQuestionParam>,
}

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
        schema_to_tool_params::<AskParams>()
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let params: AskParams = match parse_tool_args(arguments) {
            Ok(p) => p,
            Err(e) => return e,
        };

        if params.questions.is_empty() || params.questions.len() > 4 {
            return ToolResult {
                output: "questions 数量必须为 1-4 个".to_string(),
                is_error: true,
                images: vec![],
            };
        }

        let mut questions: Vec<AskQuestion> = Vec::new();
        for q in &params.questions {
            if q.options.len() < 2 || q.options.len() > 4 {
                return ToolResult {
                    output: format!("问题 '{}' 的选项数量必须为 2-4 个", q.question),
                    is_error: true,
                    images: vec![],
                };
            }

            questions.push(AskQuestion {
                question: q.question.clone(),
                header: q.header.clone(),
                options: q
                    .options
                    .iter()
                    .map(|o| AskOption {
                        label: o.label.clone(),
                        description: o.description.clone(),
                    })
                    .collect(),
                multi_select: q.multi_select,
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
