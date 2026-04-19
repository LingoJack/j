use crate::command::chat::agent::compact::InvokedSkillsMap;
use crate::command::chat::app::AskRequest;
use crate::command::chat::infra::hook::HookManager;
use crate::command::chat::infra::skill::Skill;
use crate::command::chat::permission::queue::PermissionQueue;
use async_openai::types::chat::{ChatCompletionTool, ChatCompletionTools, FunctionObject};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::sync::{Arc, Mutex, atomic::AtomicBool, mpsc};

// ========== 核心类型 ==========

pub use crate::command::chat::app::types::PlanDecision;

#[derive(Debug, Clone)]
pub struct ImageData {
    pub base64: String,
    pub media_type: String,
}

#[derive(Debug)]
pub struct ToolResult {
    pub output: String,
    pub is_error: bool,
    pub images: Vec<ImageData>,
    pub plan_decision: PlanDecision,
}

pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> Value;
    fn execute(&self, arguments: &str, cancelled: &Arc<AtomicBool>) -> ToolResult;
    fn requires_confirmation(&self) -> bool {
        false
    }
    fn confirmation_message(&self, arguments: &str) -> String {
        format!("调用工具 {} 参数: {}", self.name(), arguments)
    }
}

pub fn schema_to_tool_params<T: JsonSchema>() -> Value {
    let root = schemars::schema_for!(T);
    let mut v = serde_json::to_value(root).unwrap_or_default();

    // Extract definitions before cleanup, then inline all $ref references
    let definitions = v
        .as_object()
        .and_then(|o| o.get("definitions").cloned())
        .and_then(|d| d.as_object().cloned());

    if let Some(defs) = definitions {
        inline_refs(&mut v, &defs);
    }

    if let Some(obj) = v.as_object_mut() {
        obj.remove("$schema");
        obj.remove("title");
        obj.remove("definitions");
    }
    v
}

/// Recursively replace all `{"$ref": "#/definitions/X"}` with the inlined definition
fn inline_refs(value: &mut Value, definitions: &serde_json::Map<String, Value>) {
    match value {
        Value::Object(map) => {
            // If this object is a $ref, replace it entirely with the inlined definition
            if let Some(ref_path) = map.get("$ref").and_then(|r| r.as_str())
                && let Some(key) = ref_path.strip_prefix("#/definitions/")
                && let Some(def) = definitions.get(key)
            {
                *value = def.clone();
                // The inlined definition may itself contain $refs, so recurse
                inline_refs(value, definitions);
                return;
            }
            // Otherwise recurse into all values
            for v in map.values_mut() {
                inline_refs(v, definitions);
            }
        }
        Value::Array(arr) => {
            for v in arr.iter_mut() {
                inline_refs(v, definitions);
            }
        }
        _ => {}
    }
}

pub fn parse_tool_args<T: for<'de> Deserialize<'de>>(arguments: &str) -> Result<T, ToolResult> {
    serde_json::from_str::<T>(arguments).map_err(|e| ToolResult {
        output: format!("参数解析失败: {}", e),
        is_error: true,
        images: vec![],
        plan_decision: PlanDecision::None,
    })
}

// ========== ToolRegistry ==========

pub struct ToolRegistry {
    tools: Vec<Box<dyn Tool>>,
    pub todo_manager: Arc<super::todo::TodoManager>,
    pub plan_mode_state: Arc<super::plan::PlanModeState>,
    #[allow(dead_code)]
    pub worktree_state: Arc<super::worktree::WorktreeState>,
    pub permission_queue: Option<Arc<PermissionQueue>>,
    pub plan_approval_queue: Option<Arc<super::plan::PlanApprovalQueue>>,
}

impl ToolRegistry {
    pub fn new(
        skills: Vec<Skill>,
        ask_tx: mpsc::Sender<AskRequest>,
        background_manager: Arc<super::background::BackgroundManager>,
        task_manager: Arc<super::task::TaskManager>,
        hook_manager: Arc<Mutex<HookManager>>,
        invoked_skills: InvokedSkillsMap,
        todos_file_path: std::path::PathBuf,
    ) -> Self {
        let todo_manager = Arc::new(super::todo::TodoManager::new_with_file_path(
            todos_file_path,
        ));
        let plan_mode_state = Arc::new(super::plan::PlanModeState::new());
        let worktree_state = Arc::new(super::worktree::WorktreeState::new());
        let plan_approval_queue = Arc::new(super::plan::PlanApprovalQueue::new());

        let mut registry = Self {
            todo_manager: Arc::clone(&todo_manager),
            plan_mode_state: Arc::clone(&plan_mode_state),
            worktree_state: Arc::clone(&worktree_state),
            permission_queue: None,
            plan_approval_queue: None,
            tools: vec![
                Box::new(super::shell::ShellTool {
                    manager: Arc::clone(&background_manager),
                }),
                Box::new(super::file::ReadFileTool),
                Box::new(super::file::WriteFileTool),
                Box::new(super::file::EditFileTool),
                Box::new(super::file::GlobTool),
                Box::new(super::grep::GrepTool),
                Box::new(super::web_fetch::WebFetchTool),
                Box::new(super::web_search::WebSearchTool),
                Box::new(super::browser::BrowserTool),
                Box::new(super::ask::AskTool {
                    ask_tx: ask_tx.clone(),
                }),
                Box::new(super::background::TaskOutputTool {
                    manager: Arc::clone(&background_manager),
                }),
                Box::new(super::task::TaskTool {
                    manager: Arc::clone(&task_manager),
                }),
                Box::new(super::todo::TodoWriteTool {
                    manager: Arc::clone(&todo_manager),
                }),
                Box::new(super::todo::TodoReadTool {
                    manager: Arc::clone(&todo_manager),
                }),
                Box::new(super::compact::CompactTool),
                Box::new(super::hook::RegisterHookTool { hook_manager }),
                Box::new(super::computer_use::ComputerUseTool::new()),
                Box::new(super::plan::EnterPlanModeTool {
                    plan_state: Arc::clone(&plan_mode_state),
                }),
                Box::new(super::plan::ExitPlanModeTool {
                    plan_state: Arc::clone(&plan_mode_state),
                    ask_tx,
                    plan_approval_queue: Some(Arc::clone(&plan_approval_queue)),
                }),
                Box::new(super::worktree::EnterWorktreeTool {
                    state: Arc::clone(&worktree_state),
                }),
                Box::new(super::worktree::ExitWorktreeTool {
                    state: Arc::clone(&worktree_state),
                }),
            ],
        };

        if !skills.is_empty() {
            registry.register(Box::new(super::skill::LoadSkillTool {
                skills,
                invoked_skills,
            }));
        }

        registry
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.push(tool);
    }

    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools
            .iter()
            .find(|t| t.name() == name)
            .map(|t| t.as_ref())
    }

    pub fn execute(&self, name: &str, arguments: &str, cancelled: &Arc<AtomicBool>) -> ToolResult {
        let (is_active, plan_file_path) = self.plan_mode_state.get_state();
        if is_active && !super::plan::is_allowed_in_plan_mode(name) {
            let is_plan_file_write = (name == "Write" || name == "Edit") && {
                if let Some(ref plan_path) = plan_file_path {
                    serde_json::from_str::<serde_json::Value>(arguments)
                        .ok()
                        .and_then(|v| {
                            v.get("path")
                                .or_else(|| v.get("file_path"))
                                .and_then(|p| p.as_str())
                                .map(|p| {
                                    let input_path = std::path::Path::new(p);
                                    let plan_path_buf = std::path::Path::new(&plan_path);

                                    if p == plan_path {
                                        return true;
                                    }

                                    if input_path.is_relative()
                                        && let Ok(cwd) = std::env::current_dir()
                                    {
                                        let absolute_path = cwd.join(input_path);
                                        if let Ok(canonical_input) = absolute_path.canonicalize()
                                            && let Ok(canonical_plan) = plan_path_buf.canonicalize()
                                        {
                                            return canonical_input == canonical_plan;
                                        }
                                    }

                                    false
                                })
                        })
                        .unwrap_or(false)
                } else {
                    false
                }
            };

            if !is_plan_file_write {
                return ToolResult {
                    output: format!(
                        "Tool '{}' is not available in plan mode. Only read-only tools are allowed. \
                         Use ExitPlanMode to exit plan mode first.",
                        name
                    ),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                };
            }
        }

        match self.get(name) {
            Some(tool) => tool.execute(arguments, cancelled),
            None => ToolResult {
                output: format!("未知工具: {}", name),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            },
        }
    }

    pub fn build_tools_summary(&self, disabled: &[String]) -> String {
        let mut md = String::new();
        for t in self
            .tools
            .iter()
            .filter(|t| !disabled.iter().any(|d| d == t.name()))
        {
            let name = t.name();
            md.push_str(&format!("<{}>\n", name));
            md.push_str(&format!("description:\n{}\n", t.description().trim()));
            md.push_str(&json_schema_to_xml_params(&t.parameters_schema()));
            md.push_str(&format!("<{}/>\n\n", name));
        }
        md.trim_end().to_string()
    }

    pub fn to_openai_tools_filtered(&self, disabled: &[String]) -> Vec<ChatCompletionTools> {
        self.tools
            .iter()
            .filter(|t| !disabled.iter().any(|d| d == t.name()))
            .map(|t| {
                ChatCompletionTools::Function(ChatCompletionTool {
                    function: FunctionObject {
                        name: t.name().to_string(),
                        description: Some(t.description().trim().to_string()),
                        parameters: Some(t.parameters_schema()),
                        strict: None,
                    },
                })
            })
            .collect()
    }

    pub fn tool_names(&self) -> Vec<&str> {
        self.tools.iter().map(|t| t.name()).collect()
    }

    pub fn build_session_state_summary(&self) -> String {
        let mut parts = Vec::new();

        let (plan_active, plan_file) = self.plan_mode_state.get_state();
        if plan_active {
            let mut s = String::from("## Session State: PLAN MODE\n\n");
            s.push_str("You are currently in **Plan Mode**. Only read-only tools are available.\n");
            s.push_str(
                "Write your plan to the plan file, then use ExitPlanMode for user approval.\n",
            );
            if let Some(ref path) = plan_file {
                s.push_str(&format!("Plan file: `{}`\n", path));
            }
            parts.push(s);
        }

        if let Some(session) = self.worktree_state.get_session() {
            let mut s = String::from("## Session State: WORKTREE\n\n");
            s.push_str("You are in an isolated git worktree.\n");
            s.push_str(&format!("Branch: `{}`\n", session.branch));
            s.push_str(&format!(
                "Worktree path: `{}`\n",
                session.worktree_path.display()
            ));
            s.push_str(&format!(
                "Original cwd: `{}`\n",
                session.original_cwd.display()
            ));
            parts.push(s);
        }

        if parts.is_empty() {
            return String::new();
        }
        parts.join("\n")
    }
}

fn json_schema_to_xml_params(schema: &Value) -> String {
    let properties = match schema.get("properties").and_then(|p| p.as_object()) {
        Some(p) => p,
        None => return String::new(),
    };
    let required: Vec<&str> = schema
        .get("required")
        .and_then(|r| r.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    let mut md = String::from("parameter schema:\n");
    for (name, prop) in properties {
        let type_str = prop
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("string");
        let desc = prop
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("");
        let req = if required.contains(&name.as_str()) {
            ", required"
        } else {
            ""
        };
        md.push_str(&format!("- `{}` ({}{}) — {}\n", name, type_str, req, desc));
    }
    md
}
