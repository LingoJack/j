use crate::command::chat::app::{AskOption, AskQuestion, AskRequest};
use crate::command::chat::tools::{Tool, ToolResult};
use serde_json::{Value, json};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};

// ========== Plan Mode State ==========

/// Plan Mode 全局状态（跨工具共享）
pub struct PlanModeState {
    /// 是否处于 plan mode
    pub active: AtomicBool,
    /// plan 文件路径（EnterPlanMode 创建，ExitPlanMode 读取）
    pub plan_file_path: Mutex<Option<String>>,
}

impl PlanModeState {
    pub fn new() -> Self {
        Self {
            active: AtomicBool::new(false),
            plan_file_path: Mutex::new(None),
        }
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }

    pub fn enter(&self, path: String) {
        self.active.store(true, Ordering::Relaxed);
        if let Ok(mut p) = self.plan_file_path.lock() {
            *p = Some(path);
        }
    }

    pub fn exit(&self) {
        self.active.store(false, Ordering::Relaxed);
    }

    pub fn get_plan_file_path(&self) -> Option<String> {
        self.plan_file_path.lock().ok()?.clone()
    }
}

/// plan mode 下允许执行的工具白名单
pub const PLAN_MODE_WHITELIST: &[&str] = &[
    "Read",
    "Glob",
    "Grep",
    "WebFetch",
    "WebSearch",
    "Ask",
    "Compact",
    "TodoRead",
    "TodoWrite",
    "TaskOutput",
    "Task",
    "EnterPlanMode",
    "ExitPlanMode",
];

/// 检查工具是否在 plan mode 白名单中
pub fn is_allowed_in_plan_mode(tool_name: &str) -> bool {
    PLAN_MODE_WHITELIST.contains(&tool_name)
}

// ========== EnterPlanModeTool ==========

pub struct EnterPlanModeTool {
    pub plan_state: Arc<PlanModeState>,
}

impl EnterPlanModeTool {
    pub const NAME: &'static str = "EnterPlanMode";
}

impl Tool for EnterPlanModeTool {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn description(&self) -> &str {
        r#"
        Enter plan mode to explore the codebase and design an implementation approach before writing code.
        In plan mode, only read-only tools (Read, Glob, Grep, WebFetch, WebSearch, Ask, etc.) are available.
        Write tools (Bash, Write, Edit, etc.) will be blocked until plan mode is exited.

        Use this proactively before starting non-trivial implementation tasks. Prefer using EnterPlanMode when ANY of these apply:
        - New feature implementation with architectural decisions
        - Multiple valid approaches exist and user should choose
        - Code modifications that affect existing behavior
        - Multi-file changes (touching more than 2-3 files)
        - Unclear requirements that need exploration first

        Do NOT use for: single-line fixes, typos, or purely research/exploration tasks.
        "#
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "description": {
                    "type": "string",
                    "description": "Optional short description of what you plan to investigate"
                }
            }
        })
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        if self.plan_state.is_active() {
            return ToolResult {
                output: "Already in plan mode. Use ExitPlanMode to exit.".to_string(),
                is_error: false,
                images: vec![],
            };
        }

        let parsed: Value = serde_json::from_str(arguments).unwrap_or(Value::Null);
        let description = parsed
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("implementation plan");

        // 创建 plan 文件
        let plan_dir = crate::command::chat::permission::JcliConfig::ensure_config_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default().join(".jcli"));
        let _ = std::fs::create_dir_all(&plan_dir);
        let plan_file = plan_dir.join("plan.md");
        let plan_path = plan_file.display().to_string();

        // 写入初始模板
        let template = format!("# Plan: {}\n\n## Steps\n\n1. \n\n## Notes\n\n", description);
        let _ = std::fs::write(&plan_file, &template);

        self.plan_state.enter(plan_path.clone());

        ToolResult {
            output: format!(
                "Entered plan mode. Plan file created at: {}\n\
                 In plan mode, only read-only tools are available.\n\
                 Write your plan to the plan file, then use ExitPlanMode when ready for user approval.",
                plan_path
            ),
            is_error: false,
            images: vec![],
        }
    }

    fn requires_confirmation(&self) -> bool {
        false
    }
}

// ========== ExitPlanModeTool ==========

pub struct ExitPlanModeTool {
    pub plan_state: Arc<PlanModeState>,
    pub ask_tx: mpsc::Sender<AskRequest>,
}

impl ExitPlanModeTool {
    pub const NAME: &'static str = "ExitPlanMode";
}

impl Tool for ExitPlanModeTool {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn description(&self) -> &str {
        r#"
        Exit plan mode and submit the plan for user approval.
        Reads the plan file and presents it to the user for review.
        If approved, plan mode is deactivated and write tools become available again.
        If rejected, plan mode remains active so you can revise the plan.
        "#
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "allowedPrompts": {
                    "type": "array",
                    "description": "Optional list of prompt-based permissions needed to implement the plan",
                    "items": {
                        "type": "object",
                        "properties": {
                            "tool": {
                                "type": "string",
                                "description": "The tool this prompt applies to (e.g. 'Bash')"
                            },
                            "prompt": {
                                "type": "string",
                                "description": "Semantic description of the action (e.g. 'run tests')"
                            }
                        }
                    }
                }
            }
        })
    }

    fn execute(&self, _arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        if !self.plan_state.is_active() {
            return ToolResult {
                output: "Not in plan mode. Use EnterPlanMode first.".to_string(),
                is_error: true,
                images: vec![],
            };
        }

        // 读取 plan 文件内容
        let plan_content = match self.plan_state.get_plan_file_path() {
            Some(path) => match std::fs::read_to_string(&path) {
                Ok(content) => content,
                Err(e) => {
                    return ToolResult {
                        output: format!("Failed to read plan file: {}", e),
                        is_error: true,
                        images: vec![],
                    };
                }
            },
            None => {
                return ToolResult {
                    output: "No plan file path set.".to_string(),
                    is_error: true,
                    images: vec![],
                };
            }
        };

        // 通过 Ask 机制发送审批请求
        let (response_tx, response_rx) = mpsc::channel::<String>();

        let question_text = format!(
            "Please review the implementation plan:\n\n{}\n\nDo you approve this plan?",
            plan_content
        );

        let ask_request = AskRequest {
            questions: vec![AskQuestion {
                question: question_text,
                header: "Plan Review".to_string(),
                options: vec![
                    AskOption {
                        label: "Approve".to_string(),
                        description: "Approve the plan and proceed with implementation".to_string(),
                    },
                    AskOption {
                        label: "Reject".to_string(),
                        description: "Reject the plan and stay in plan mode to revise".to_string(),
                    },
                ],
                multi_select: false,
            }],
            response_tx,
        };

        if self.ask_tx.send(ask_request).is_err() {
            return ToolResult {
                output: "Failed to send approval request (main thread may have exited)".to_string(),
                is_error: true,
                images: vec![],
            };
        }

        // 阻塞等待用户审批结果
        match response_rx.recv() {
            Ok(response) => {
                // 解析用户选择
                let approved = response.contains("Approve");
                if approved {
                    self.plan_state.exit();
                    ToolResult {
                        output: "Plan approved! Exited plan mode. You can now proceed with implementation.".to_string(),
                        is_error: false,
                        images: vec![],
                    }
                } else {
                    // 保持 plan mode，让 agent 修改 plan
                    ToolResult {
                        output: format!(
                            "Plan was not approved. Still in plan mode. User response: {}\nPlease revise your plan and try ExitPlanMode again.",
                            response
                        ),
                        is_error: false,
                        images: vec![],
                    }
                }
            }
            Err(_) => ToolResult {
                output: "Connection lost while waiting for approval".to_string(),
                is_error: true,
                images: vec![],
            },
        }
    }

    fn requires_confirmation(&self) -> bool {
        false
    }
}
