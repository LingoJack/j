use crate::command::chat::tools::{
    PlanDecision, Tool, ToolResult, parse_tool_args, resolve_path, schema_to_tool_params,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::sync::{Arc, atomic::AtomicBool};

/// WriteFileTool 参数
#[derive(Deserialize, JsonSchema)]
struct WriteFileParams {
    /// File path to write (absolute or relative to current working directory)
    path: String,
    /// Content to write to the file
    content: String,
}

/// 写入文件的工具
pub struct WriteFileTool;

impl WriteFileTool {
    pub const NAME: &'static str = "Write";
}

impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn description(&self) -> &str {
        r#"
        Writes a file to the local filesystem.

        Usage:
        - This tool will overwrite the existing file if there is one at the provided path
        - If this is an existing file, you MUST use the Read tool first to read the file's contents
        - Prefer the Edit tool for modifying existing files — it only sends the diff. Only use this tool to create new files or for complete rewrites
        - NEVER create documentation files (*.md) or README files unless explicitly requested by the User
        - Only use emojis if the user explicitly requests it
        - Auto-creates parent directories if they don't exist
        "#
    }

    fn parameters_schema(&self) -> Value {
        schema_to_tool_params::<WriteFileParams>()
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let params: WriteFileParams = match parse_tool_args(arguments) {
            Ok(p) => p,
            Err(e) => return e,
        };

        let path = resolve_path(&params.path);

        // 文件编辑互斥锁（多 agent 模式下防止同时写入同一文件）
        let agent_name = crate::command::chat::teammate::current_agent_name();
        let file_path_ref = std::path::Path::new(&path);
        let _lock_guard = match crate::command::chat::teammate::acquire_global_file_lock(
            file_path_ref,
            &agent_name,
        ) {
            Ok(guard) => guard,
            Err(holder) => {
                return ToolResult {
                    output: format!("文件 {} 正被 {} 编辑，请稍后重试", path, holder),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                };
            }
        };

        // 自动创建父目录
        let file_path = std::path::Path::new(&path);
        if let Some(parent) = file_path.parent()
            && !parent.exists()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            return ToolResult {
                output: format!("创建目录失败: {}", e),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            };
        }

        match std::fs::write(&path, &params.content) {
            Ok(_) => ToolResult {
                output: format!("已写入文件: {} ({} 字节)", path, params.content.len()),
                is_error: false,
                images: vec![],
                plan_decision: PlanDecision::None,
            },
            Err(e) => ToolResult {
                output: format!("写入文件失败: {}", e),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            },
        }
    }

    fn requires_confirmation(&self) -> bool {
        true
    }

    fn confirmation_message(&self, arguments: &str) -> String {
        let path = serde_json::from_str::<WriteFileParams>(arguments)
            .ok()
            .map(|p| resolve_path(&p.path))
            .unwrap_or_else(|| "未知路径".to_string());
        format!("即将写入文件: {}", path)
    }
}
