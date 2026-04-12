use crate::command::chat::app::{AskOption, AskQuestion, AskRequest};
use crate::command::chat::tools::{Tool, ToolResult, schema_to_tool_params};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::sync::{Arc, Mutex, atomic::AtomicBool, mpsc};

// ========== Plan Mode State ==========

/// Plan mode 内部状态（受 Mutex 保护，保证原子性）
struct PlanModeInner {
    active: bool,
    plan_file_path: Option<String>,
}

/// Plan Mode 全局状态（跨工具共享）
///
/// 使用单一 Mutex 保护 active + plan_file_path，避免以下并发问题：
/// - enter() 的 TOCTOU 竞态（先检查 is_active 再进入）
/// - exit() 不清理 plan_file_path
/// - is_active() 与 get_plan_file_path() 之间状态不一致
pub struct PlanModeState {
    inner: Mutex<PlanModeInner>,
}

impl Default for PlanModeState {
    fn default() -> Self {
        Self::new()
    }
}

impl PlanModeState {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(PlanModeInner {
                active: false,
                plan_file_path: None,
            }),
        }
    }

    /// 检查是否处于 plan mode
    pub fn is_active(&self) -> bool {
        self.inner.lock().map(|g| g.active).unwrap_or(false)
    }

    /// 进入 plan mode，同时设置 plan 文件路径
    /// 返回 Ok(()) 表示成功进入，Err(msg) 表示已在 plan mode
    pub fn enter(&self, path: String) -> Result<(), String> {
        match self.inner.lock() {
            Ok(mut guard) => {
                if guard.active {
                    return Err("Already in plan mode. Use ExitPlanMode to exit.".to_string());
                }
                guard.active = true;
                guard.plan_file_path = Some(path);
                Ok(())
            }
            Err(e) => Err(format!("Lock poisoned: {}", e)),
        }
    }

    /// 退出 plan mode，保留 plan 文件（不删除）
    pub fn exit(&self) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.active = false;
            guard.plan_file_path = None;
        }
    }

    /// 原子地检查是否 active 并获取 plan 文件路径
    /// 返回 (is_active, plan_file_path)
    pub fn get_state(&self) -> (bool, Option<String>) {
        match self.inner.lock() {
            Ok(guard) => (guard.active, guard.plan_file_path.clone()),
            Err(_) => (false, None),
        }
    }

    /// 获取 plan 文件路径（仅在 active 时有意义）
    pub fn get_plan_file_path(&self) -> Option<String> {
        self.inner.lock().ok()?.plan_file_path.clone()
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
    "EnterWorktree",
    "ExitWorktree",
];

/// 检查工具是否在 plan mode 白名单中
pub fn is_allowed_in_plan_mode(tool_name: &str) -> bool {
    PLAN_MODE_WHITELIST.contains(&tool_name)
}

// ========== EnterPlanModeTool ==========

/// 将描述文本转为安全的文件名（只保留字母数字、中文、下划线、短横线）
fn sanitize_filename(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-' || *c > '\u{4e00}')
        .collect::<String>()
        .trim()
        .to_string()
}

/// EnterPlanMode 参数
#[derive(Deserialize, JsonSchema)]
struct EnterPlanModeParams {
    /// Short description used as the plan file name (e.g. "add-auth" becomes plan-add-auth.md)
    #[serde(default)]
    description: Option<String>,
}

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

        The `description` parameter is used as the plan file name (e.g. "add-auth" → plan-add-auth.md).
        If a plan file with the same name already exists, you will be warned so you can choose a different name.
        Plan files are preserved after exiting plan mode for future reference.
        "#
    }

    fn parameters_schema(&self) -> Value {
        schema_to_tool_params::<EnterPlanModeParams>()
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let params: EnterPlanModeParams =
            serde_json::from_str(arguments).unwrap_or(EnterPlanModeParams { description: None });
        let description = params
            .description
            .as_deref()
            .unwrap_or("implementation-plan");

        // 创建 plan 目录
        let plan_dir = crate::command::chat::permission::JcliConfig::ensure_config_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default().join(".jcli"));
        let plans_dir = plan_dir.join("plans");
        let _ = std::fs::create_dir_all(&plans_dir);

        // 基于描述生成文件名（如 plan-add-auth.md）
        let safe_name = sanitize_filename(description);
        let file_name = if safe_name.is_empty() {
            format!("plan-{}.md", std::process::id())
        } else {
            format!("plan-{}.md", safe_name)
        };
        let plan_file = plans_dir.join(&file_name);
        let plan_path = plan_file.display().to_string();

        // 检查同名文件是否已存在
        let mut warning = String::new();
        if plan_file.exists() {
            match std::fs::read_to_string(&plan_file) {
                Ok(existing) => {
                    // 提取已有 plan 的第一行标题作为摘要
                    let first_line = existing.lines().next().unwrap_or("");
                    warning = format!(
                        "⚠️ Plan file already exists: {} (content starts with: {})\n\
                         The existing file will be overwritten. Consider using a different description to avoid this.\n\n",
                        plan_path, first_line
                    );
                }
                Err(_) => {
                    warning = format!(
                        "⚠️ Plan file already exists: {}\n\
                         The existing file will be overwritten. Consider using a different description to avoid this.\n\n",
                        plan_path
                    );
                }
            }
        }

        // 写入初始模板
        let template = format!("# Plan: {}\n\n## Steps\n\n1. \n\n## Notes\n\n", description);
        let _ = std::fs::write(&plan_file, &template);

        // 原子性地进入 plan mode
        match self.plan_state.enter(plan_path.clone()) {
            Ok(()) => ToolResult {
                output: format!(
                    "{}Entered plan mode. Plan file: {}\n\
                     In plan mode, only read-only tools are available.\n\
                     Write your plan to the plan file, then use ExitPlanMode when ready for user approval.\n\
                     Plan files are preserved after exit for future reference.",
                    warning, plan_path
                ),
                is_error: false,
                images: vec![],
            },
            Err(msg) => ToolResult {
                output: msg,
                is_error: false,
                images: vec![],
            },
        }
    }

    fn requires_confirmation(&self) -> bool {
        false
    }
}

// ========== ExitPlanModeTool ==========

/// ExitPlanMode 参数
#[derive(Deserialize, JsonSchema)]
#[allow(dead_code)]
struct ExitPlanModeParams {
    /// Optional list of prompt-based permissions needed to implement the plan
    #[serde(default)]
    #[serde(rename = "allowedPrompts")]
    allowed_prompts: Option<Vec<AllowedPrompt>>,
}

/// 计划实施所需的权限描述
#[derive(Deserialize, JsonSchema)]
#[allow(dead_code)]
struct AllowedPrompt {
    /// The tool this prompt applies to (e.g. 'Bash')
    #[serde(default)]
    tool: Option<String>,
    /// Semantic description of the action (e.g. 'run tests')
    #[serde(default)]
    prompt: Option<String>,
}

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
        schema_to_tool_params::<ExitPlanModeParams>()
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
            "请审阅以下实施计划：\n\n{}\n\n是否批准此计划？",
            plan_content
        );

        let ask_request = AskRequest {
            questions: vec![AskQuestion {
                question: question_text,
                header: "Plan Review".to_string(),
                options: vec![
                    AskOption {
                        label: "同意".to_string(),
                        description: "批准计划并开始实施".to_string(),
                    },
                    AskOption {
                        label: "同意并清空上下文".to_string(),
                        description: "批准计划，清空之前的探索上下文，只保留计划内容".to_string(),
                    },
                    AskOption {
                        label: "拒绝".to_string(),
                        description: "拒绝计划，继续留在 plan mode 修改".to_string(),
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
                if response.contains("同意并清空上下文") {
                    let plan_file_path = self.plan_state.get_plan_file_path();
                    self.plan_state.exit();
                    // plan 文件保留不删除，告知路径
                    let preserved_msg = plan_file_path
                        .as_deref()
                        .map(|p| format!("\nPlan file preserved at: {}", p))
                        .unwrap_or_default();
                    // 用 PLAN_CLEAR_CONTEXT: 前缀传递计划内容，agent loop 检测到此信号后清空上下文
                    ToolResult {
                        output: format!("PLAN_CLEAR_CONTEXT:{}{}", plan_content, preserved_msg),
                        is_error: false,
                        images: vec![],
                    }
                } else if response.contains("同意") {
                    let plan_file_path = self.plan_state.get_plan_file_path();
                    self.plan_state.exit();
                    let preserved_msg = plan_file_path
                        .as_deref()
                        .map(|p| format!("\nPlan file preserved at: {}", p))
                        .unwrap_or_default();
                    ToolResult {
                        output: format!(
                            "Plan approved! Exited plan mode. You can now proceed with implementation.{}",
                            preserved_msg
                        ),
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
