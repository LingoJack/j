use crate::command::chat::app::AskRequest;
use crate::command::chat::context::compact::InvokedSkillsMap;
use crate::command::chat::infra::hook::HookManager;
use crate::command::chat::infra::skill::Skill;
use crate::command::chat::permission::queue::PermissionQueue;
use crate::command::chat::tools::tool_names;
use crate::llm::{FunctionObject, ToolDefinition};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::borrow::Cow;
use std::sync::{Arc, Mutex, atomic::AtomicBool, mpsc};

// ========== 核心类型 ==========

pub use crate::command::chat::app::types::PlanDecision;

/// 图片数据，以 base64 编码存储
#[derive(Debug, Clone)]
pub struct ImageData {
    /// 图片的 base64 编码数据
    pub base64: String,
    /// 图片的 MIME 媒体类型（如 "image/png"）
    pub media_type: String,
}

/// 工具执行结果，包含输出文本、错误标记、附加图片和计划决策
#[derive(Debug)]
pub struct ToolResult {
    /// 工具执行的文本输出
    pub output: String,
    /// 是否为错误结果
    pub is_error: bool,
    /// 执行过程中产生的图片列表
    pub images: Vec<ImageData>,
    /// 计划模式下的决策结果
    pub plan_decision: PlanDecision,
}

/// 工具核心接口，定义工具的名称、描述、参数模式和执行逻辑
pub trait Tool: Send + Sync {
    /// 返回工具名称
    fn name(&self) -> &str;
    /// 返回工具功能描述，静态描述返回 `Cow::Borrowed`，动态描述返回 `Cow::Owned`
    fn description(&self) -> Cow<'_, str>;
    /// 返回工具参数的 JSON Schema
    fn parameters_schema(&self) -> Value;
    /// 执行工具，传入参数字符串和取消信号，返回执行结果
    fn execute(&self, arguments: &str, cancelled: &Arc<AtomicBool>) -> ToolResult;
    /// 该工具是否需要用户确认后才能执行
    fn requires_confirmation(&self) -> bool {
        false
    }
    /// 生成用户确认时显示的提示消息
    fn confirmation_message(&self, arguments: &str) -> String {
        format!("调用工具 {} 参数: {}", self.name(), arguments)
    }
    /// 工具是否当前可用（默认 `true`）。
    ///
    /// 返回 `false` 时，该工具不会出现在 LLM 的工具列表和工具摘要中，
    /// 且直接调用会返回错误提示。
    fn is_available(&self) -> bool {
        true
    }
}

/// 将实现了 `JsonSchema` 的类型转换为基础清理后的工具参数 JSON Schema，
/// 自动内联所有 `$ref` 引用并移除 `$schema`、`title`、`definitions` 等冗余字段
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

/// 将 JSON 参数字符串解析为指定类型 `T`，解析失败时返回包含错误信息的 `ToolResult`
pub fn parse_tool_args<T: for<'de> Deserialize<'de>>(arguments: &str) -> Result<T, ToolResult> {
    serde_json::from_str::<T>(arguments).map_err(|e| ToolResult {
        output: format!("参数解析失败: {}", e),
        is_error: true,
        images: vec![],
        plan_decision: PlanDecision::None,
    })
}

// ========== ToolRegistry ==========

/// 工具注册中心，管理所有可用工具及其相关状态
pub struct ToolRegistry {
    tools: Vec<Box<dyn Tool>>,
    /// 待办事项管理器
    pub todo_manager: Arc<super::todo::TodoManager>,
    /// 计划模式状态
    pub plan_mode_state: Arc<super::plan::PlanModeState>,
    /// 工作树状态（当前未使用）
    #[allow(dead_code)]
    pub worktree_state: Arc<super::worktree::WorktreeState>,
    /// 权限请求队列
    pub permission_queue: Option<Arc<PermissionQueue>>,
    /// 计划审批队列
    pub plan_approval_queue: Option<Arc<super::plan::PlanApprovalQueue>>,
}

impl std::fmt::Debug for ToolRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tool_names: Vec<&str> = self.tools.iter().map(|t| t.name()).collect();
        f.debug_struct("ToolRegistry")
            .field("tool_names", &tool_names)
            .finish()
    }
}

impl ToolRegistry {
    /// 创建工具注册中心，初始化所有内置工具及相关状态
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

        let tools: Vec<Box<dyn Tool>> = vec![
            #[cfg(unix)]
            Box::new(super::shell::ShellTool {
                manager: Arc::clone(&background_manager),
            }),
            #[cfg(windows)]
            Box::new(super::powershell::PowerShellTool {
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
            Box::new(super::compact_tool::CompactTool),
            Box::new(super::hook::RegisterHookTool { hook_manager }),
            #[cfg(target_os = "macos")]
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
        ];

        let mut registry = Self {
            todo_manager: Arc::clone(&todo_manager),
            plan_mode_state: Arc::clone(&plan_mode_state),
            worktree_state: Arc::clone(&worktree_state),
            permission_queue: None,
            plan_approval_queue: None,
            tools,
        };

        if !skills.is_empty() {
            registry.register(Box::new(super::skill::LoadSkillTool {
                skills,
                invoked_skills,
            }));
        }

        registry
    }

    /// 注册一个新工具到注册中心
    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.push(tool);
    }

    /// 根据名称获取工具的引用
    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools
            .iter()
            .find(|t| t.name() == name)
            .map(|t| t.as_ref())
    }

    /// 执行指定名称的工具，自动处理计划模式下的权限限制
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
            Some(tool) => {
                if !tool.is_available() {
                    return ToolResult {
                        output: format!("Tool '{}' is currently not available.", name),
                        is_error: true,
                        images: vec![],
                        plan_decision: PlanDecision::None,
                    };
                }
                tool.execute(arguments, cancelled)
            }
            None => ToolResult {
                output: format!("未知工具: {}", name),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            },
        }
    }

    /// 构建工具摘要（排除 disabled 和 deferred 工具），用于 system prompt
    pub fn build_tools_summary_non_deferred(
        &self,
        disabled: &[String],
        deferred: &[String],
    ) -> String {
        let mut md = String::new();
        for t in self
            .tools
            .iter()
            .filter(|t| !disabled.iter().any(|d| d == t.name()))
            .filter(|t| t.is_available())
            .filter(|t| !deferred.iter().any(|d| d == t.name()))
        {
            let name = t.name();
            md.push_str(&format!("<{}>\n", name));
            md.push_str(&format!("description:\n{}\n", t.description().trim()));
            let params = json_schema_to_xml_params(&t.parameters_schema());
            if !params.is_empty() {
                md.push('\n');
                md.push_str(&params);
            }
            md.push_str(&format!("</{}>\n\n", name));
        }

        md.trim_end().to_string()
    }

    /// 将未禁用、可用且非 deferred 的工具转换为 LLM 工具定义列表
    /// LoadTool 始终包含在列表中，其 description 动态包含当前 deferred 工具名
    pub fn to_llm_tools_non_deferred(
        &self,
        disabled: &[String],
        deferred: &[String],
    ) -> Vec<ToolDefinition> {
        let mut tools: Vec<ToolDefinition> = self
            .tools
            .iter()
            .filter(|t| !disabled.iter().any(|d| d == t.name()))
            .filter(|t| t.is_available())
            .filter(|t| !deferred.iter().any(|d| d == t.name()))
            .map(|t| ToolDefinition {
                tool_type: "function".to_string(),
                function: FunctionObject {
                    name: t.name().to_string(),
                    description: Some(t.description().trim().to_string()),
                    parameters: Some(t.parameters_schema()),
                    strict: None,
                },
            })
            .collect();

        // LoadTool 始终加入列表，description 已动态包含 deferred 工具列表。
        // 若 registry 中无 LoadTool（如子 agent registry），此 if let 静默跳过——
        // 子 agent 不支持动态加载，这是有意为之的降级行为。
        if let Some(load_tool) = self
            .tools
            .iter()
            .find(|t| t.name() == tool_names::LOAD_TOOL)
            && load_tool.is_available()
            && !disabled.iter().any(|d| d == load_tool.name())
            && !tools
                .iter()
                .any(|t| t.function.name == tool_names::LOAD_TOOL)
        {
            tools.push(ToolDefinition {
                tool_type: "function".to_string(),
                function: FunctionObject {
                    name: tool_names::LOAD_TOOL.to_string(),
                    description: Some(load_tool.description().trim().to_string()),
                    parameters: Some(load_tool.parameters_schema()),
                    strict: None,
                },
            });
        }

        tools
    }

    /// 返回所有已注册工具的名称列表
    pub fn tool_names(&self) -> Vec<&str> {
        self.tools.iter().map(|t| t.name()).collect()
    }

    /// 构建会话状态摘要，包含计划模式和工作树等当前状态信息
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

    let mut md = String::from("parameters:\n");
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

#[cfg(test)]
mod tests;
