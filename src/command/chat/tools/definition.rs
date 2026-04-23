use crate::command::chat::app::AskRequest;
use crate::command::chat::context::compact::InvokedSkillsMap;
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
    /// 返回工具功能描述
    fn description(&self) -> &str;
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
                Box::new(super::compact_tool::CompactTool),
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
            Some(tool) => tool.execute(arguments, cancelled),
            None => ToolResult {
                output: format!("未知工具: {}", name),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            },
        }
    }

    /// 构建工具摘要，以 XML 格式展示所有未禁用工具的名称、描述和参数
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

    /// 将未禁用的工具转换为 OpenAI 函数调用格式的工具列表
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

#[cfg(test)]
mod tests {
    use super::*;
    use schemars::JsonSchema;
    use serde::Deserialize;

    /// 模拟 TodoWriteParams 结构（与 todo_write_tool.rs 中定义相同）
    #[derive(Deserialize, JsonSchema)]
    struct TodoItemParam {
        #[serde(default)]
        id: Option<String>,
        #[serde(default)]
        content: String,
        #[serde(default = "default_status")]
        status: String,
    }

    fn default_status() -> String {
        "pending".to_string()
    }

    #[derive(Deserialize, JsonSchema)]
    struct TodoWriteParams {
        todos: Vec<TodoItemParam>,
        #[serde(default)]
        merge: bool,
    }

    /// 测试 schemars 生成的 schema 中 content 不再出现在 required 里
    #[test]
    fn test_schema_content_not_required() {
        let schema = schema_to_tool_params::<TodoWriteParams>();

        // 顶层 required 应包含 "todos"，不包含 "merge"（有 serde default）
        let required: Vec<&str> = schema
            .get("required")
            .and_then(|r| r.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default();

        assert!(
            required.contains(&"todos"),
            "todos should be required, got: {:?}",
            required
        );
        assert!(
            !required.contains(&"merge"),
            "merge should NOT be required (has serde default), got: {:?}",
            required
        );

        // todos items 里的 required：id 和 content 都有 serde(default)，不应出现
        let todos_schema = schema
            .get("properties")
            .and_then(|p| p.get("todos"))
            .and_then(|t| t.get("items"));

        assert!(todos_schema.is_some(), "todos.items should exist in schema");

        let item_required: Vec<&str> = todos_schema
            .and_then(|s| s.get("required"))
            .and_then(|r| r.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default();

        // 关键断言：content 不应在 required 里（因为 #[serde(default)]）
        assert!(
            !item_required.contains(&"content"),
            "content should NOT be required in todo items (has serde default), got required: {:?}",
            item_required
        );
        assert!(
            !item_required.contains(&"id"),
            "id should NOT be required in todo items (has serde default), got required: {:?}",
            item_required
        );
    }

    /// 测试 schema 内联后没有残留的 $ref
    #[test]
    fn test_schema_no_dangling_refs() {
        let schema = schema_to_tool_params::<TodoWriteParams>();
        let schema_str = serde_json::to_string(&schema).unwrap();
        assert!(
            !schema_str.contains("\"$ref\""),
            "Schema should not contain any $ref after inlining, got: {}",
            schema_str
        );
    }

    /// 测试 merge=true 时只传 id+status（不传 content）能正确反序列化
    #[test]
    fn test_merge_without_content_deserializes() {
        let json = r#"{"todos": [{"id": "1", "status": "completed"}], "merge": true}"#;
        let params: TodoWriteParams = serde_json::from_str(json).unwrap();
        assert!(params.merge);
        assert_eq!(params.todos.len(), 1);
        assert_eq!(params.todos[0].id, Some("1".to_string()));
        assert_eq!(params.todos[0].content, ""); // default empty string
        assert_eq!(params.todos[0].status, "completed");
    }

    /// 测试完整参数能正确反序列化
    #[test]
    fn test_full_params_deserialize() {
        let json = r#"{"todos": [{"id": "1", "content": "implement feature", "status": "in_progress"}, {"content": "write tests"}], "merge": false}"#;
        let params: TodoWriteParams = serde_json::from_str(json).unwrap();
        assert!(!params.merge);
        assert_eq!(params.todos.len(), 2);
        assert_eq!(params.todos[0].content, "implement feature");
        assert_eq!(params.todos[1].id, None);
        assert_eq!(params.todos[1].content, "write tests");
        assert_eq!(params.todos[1].status, "pending"); // default
    }
}
