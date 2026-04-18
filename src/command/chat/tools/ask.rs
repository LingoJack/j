use crate::command::chat::app::{AskOption, AskQuestion, AskRequest};
use crate::command::chat::tools::{
    PlanDecision, Tool, ToolResult, parse_tool_args, schema_to_tool_params,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::sync::{Arc, atomic::AtomicBool, mpsc};

/// 选项参数
#[derive(Deserialize, JsonSchema)]
struct AskOptionParam {
    /// Option display label shown to user (1-5 words). REQUIRED. Example: "确认无误"
    label: String,
    /// Short explanation of what this option means. Example: "继续进入下一阶段"
    #[serde(default)]
    description: String,
}

/// 问题参数
#[derive(Deserialize, JsonSchema)]
struct AskQuestionParam {
    /// Full question text shown to user
    question: String,
    /// Short tag (max 12 chars) as column header, e.g. '需求确认'
    header: String,
    /// List of 2-4 option OBJECTS. Each element MUST be {"label": "...", "description": "..."}, NOT a plain string.
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

#[derive(Debug)]
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

        IMPORTANT — Input format:
        - `questions` is an array of question OBJECTS (NOT strings).
        - Each question object has: `question` (full text), `header` (short tag ≤12 chars), `options` (array of OPTION OBJECTS), `multi_select` (bool, optional).
        - Each option object has: `label` (1-5 words shown to user) and `description` (short explanation). DO NOT pass options as plain strings.

        Concrete example (copy this structure exactly):
        ```json
        {
          "questions": [
            {
              "question": "需求文档是否符合预期？",
              "header": "需求确认",
              "options": [
                {"label": "确认无误", "description": "继续进入 API 设计和前端开发阶段"},
                {"label": "需要调整", "description": "我会指出需要修改的部分"}
              ],
              "multi_select": false
            }
          ]
        }
        ```

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
                plan_decision: PlanDecision::None,
            };
        }

        let mut questions: Vec<AskQuestion> = Vec::new();
        for q in &params.questions {
            if q.options.len() < 2 || q.options.len() > 4 {
                return ToolResult {
                    output: format!("问题 '{}' 的选项数量必须为 2-4 个", q.question),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
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
                plan_decision: PlanDecision::None,
            };
        }

        // 阻塞等待用户响应
        match response_rx.recv() {
            Ok(response) => ToolResult {
                output: response,
                is_error: false,
                images: vec![],
                plan_decision: PlanDecision::None,
            },
            Err(_) => ToolResult {
                output: "等待用户响应时连接断开".to_string(),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            },
        }
    }

    fn requires_confirmation(&self) -> bool {
        false
    }
}
