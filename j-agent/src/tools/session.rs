//! Session 工具：操作交互式进程的标准输入输出
//!
//! 当 Shell 工具使用 interactive: true 启动时，返回 sid，
//! Agent 可用 Session 工具与该进程持续交互。

use super::background::BackgroundManager;
use super::{PlanDecision, Tool, ToolResult, parse_tool_args, schema_to_tool_params};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use std::borrow::Cow;
use std::sync::{Arc, atomic::AtomicBool};

/// Session 工具参数
#[derive(Deserialize, JsonSchema)]
struct SessionParams {
    /// 操作类型：stdin 向进程写入内容；stdout 读取当前输出；quit 终止会话
    action: String,
    /// 会话 ID，即 Shell 返回的 sid 字段，与 task_id 相同
    sid: String,
    /// action=stdin 时必填，写入进程的内容，换行用 \n
    #[serde(default)]
    text: Option<String>,
    /// action=stdout 时有效，等待输出的最大毫秒数，默认 500
    #[serde(default)]
    timeout_ms: Option<u64>,
}

// ========== SessionTool ==========

/// 交互式进程会话工具
#[derive(Debug)]
pub struct SessionTool {
    pub manager: Arc<BackgroundManager>,
}

impl SessionTool {
    pub const NAME: &'static str = "Session";
}

impl Tool for SessionTool {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn description(&self) -> Cow<'_, str> {
        r#"
        Operate on an interactive process session's stdin/stdout. Use this tool when Shell returns a sid (session ID) with interactive: true.

        Actions:
        - stdin: Write text to the process. Use \n for newlines. For example, to answer a prompt, send the text followed by \n.
        - stdout: Read the process's output since your last read. Blocks up to timeout_ms (default 500) waiting for new output.
        - quit: Terminate the session. Drops the PTY handle, the process receives SIGHUP and exits naturally.

        Example workflow:
        1. Shell with interactive: true → returns { "sid": "bg_5", ... }
        2. Session action=stdin, sid=bg_5, text="hello\n" → sends "hello" + Enter
        3. Session action=stdout, sid=bg_5 → reads process output
        4. Session action=quit, sid=bg_5 → terminates the session
        "#.into()
    }

    fn parameters_schema(&self) -> Value {
        schema_to_tool_params::<SessionParams>()
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let params: SessionParams = match parse_tool_args(arguments) {
            Ok(p) => p,
            Err(e) => return e,
        };

        let result = match params.action.as_str() {
            "stdin" => {
                let text = match params.text {
                    Some(t) => t,
                    None => {
                        return ToolResult {
                            output: json!({ "ok": false, "error": "action=stdin requires 'text' parameter" }).to_string(),
                            is_error: true,
                            images: vec![],
                            plan_decision: PlanDecision::None,
                        };
                    }
                };
                // 将 \n 转义序列转为实际换行符
                let text = text.replace("\\n", "\n");
                match self.manager.session_stdin(&params.sid, &text) {
                    Ok(()) => json!({ "ok": true }),
                    Err(e) => json!({ "ok": false, "error": e }),
                }
            }
            "stdout" => {
                let timeout_ms = params.timeout_ms.unwrap_or(500);
                match self.manager.session_stdout(&params.sid, timeout_ms) {
                    Ok(output) => json!({ "ok": true, "output": output }),
                    Err(e) => json!({ "ok": false, "error": e }),
                }
            }
            "quit" => match self.manager.session_quit(&params.sid) {
                Ok(()) => json!({ "ok": true }),
                Err(e) => json!({ "ok": false, "error": e }),
            },
            other => {
                json!({ "ok": false, "error": format!("unknown action: {}", other) })
            }
        };

        ToolResult {
            output: result.to_string(),
            is_error: false,
            images: vec![],
            plan_decision: PlanDecision::None,
        }
    }

    fn requires_confirmation(&self) -> bool {
        false
    }

    fn confirmation_message(&self, _arguments: &str) -> String {
        String::new()
    }
}
