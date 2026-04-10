use crate::command::chat::tools::{
    Tool, ToolResult, expand_tilde, parse_tool_args, schema_to_tool_params,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::sync::{Arc, atomic::AtomicBool};

/// 计算两个字符串切片的最长公共子序列（LCS）表
/// 返回 `(i, j)` 表示 `a[..i]` 和 `b[..j]` 是 LCS
fn lcs(a: &[&str], b: &[&str]) -> Vec<Vec<usize>> {
    let m = a.len();
    let n = b.len();
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 1..=m {
        for j in 1..=n {
            if a[i - 1] == b[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }
    dp
}

/// 根据 LCS 表回溯生成 diff 操作序列
fn backtrack(dp: &[Vec<usize>], a: &[&str], b: &[&str]) -> Vec<(char, String)> {
    let mut result = Vec::new();
    let mut i = a.len();
    let mut j = b.len();
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && a[i - 1] == b[j - 1] && dp[i][j] == dp[i - 1][j - 1] + 1 {
            result.push((' ', a[i - 1].to_owned()));
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || dp[i][j] == dp[i][j - 1]) {
            result.push(('+', b[j - 1].to_owned()));
            j -= 1;
        } else if i > 0 && (j == 0 || dp[i][j] == dp[i - 1][j]) {
            result.push(('-', a[i - 1].to_owned()));
            i -= 1;
        } else {
            // fallback: shouldn't happen, but handle gracefully
            result.push(('-', a[i - 1].to_owned()));
            i -= 1;
        }
    }
    result.reverse();
    result
}

/// 生成行级 diff 字符串（包含上下文，`-` 和 `+` 交错而非全减后全加）
fn build_line_diff(old: &str, new: &str) -> String {
    let has_any = !old.is_empty() || !new.is_empty();
    if !has_any {
        return "".to_string();
    }

    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    let dp = lcs(&old_lines, &new_lines);
    let ops = backtrack(&dp, &old_lines, &new_lines);

    let mut output = String::from("```diff\n");
    for (op, line) in &ops {
        output.push_str(&format!("{} {}\n", op, line));
    }
    output.push_str("```");
    output
}

/// EditFileTool 参数
#[derive(Deserialize, JsonSchema)]
struct EditFileParams {
    /// File path to edit
    path: String,
    /// Original string to replace (must be unique in the file)
    old_string: String,
    /// Replacement string; empty string means delete
    #[serde(default)]
    new_string: String,
}

/// 编辑文件的工具（基于字符串替换）
pub struct EditFileTool;

impl Tool for EditFileTool {
    fn name(&self) -> &str {
        "Edit"
    }

    fn description(&self) -> &str {
        r#"
        Performs exact string replacements in files.

        Usage:
        - You must use your Read tool at least once in the conversation before editing. Read the file first to understand its content
        - When editing text from Read tool output, ensure you preserve the exact indentation (tabs/spaces). The line number prefix format is: number + │. Everything after │ is the actual file content to match. Never include any part of the line number prefix in old_string or new_string
        - ALWAYS prefer editing existing files in the codebase. NEVER write new files unless explicitly required
        - Only use emojis if the user explicitly requests it
        - The edit will FAIL if old_string is not unique in the file. Either provide a larger string with more surrounding context to make it unique, or break the edit into smaller unique chunks
        - If new_string is empty, the matched content is deleted
        "#
    }

    fn parameters_schema(&self) -> Value {
        schema_to_tool_params::<EditFileParams>()
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let params: EditFileParams = match parse_tool_args(arguments) {
            Ok(p) => p,
            Err(e) => return e,
        };

        let path = expand_tilde(&params.path);

        // 文件编辑互斥锁（多 agent 模式下防止同时编辑同一文件）
        let agent_name = crate::command::chat::teammate::current_agent_name();
        let file_path = std::path::Path::new(&path);
        let _lock_guard = match crate::command::chat::teammate::acquire_global_file_lock(
            file_path,
            &agent_name,
        ) {
            Ok(guard) => guard,
            Err(holder) => {
                return ToolResult {
                    output: format!("文件 {} 正被 {} 编辑，请稍后重试", path, holder),
                    is_error: true,
                    images: vec![],
                };
            }
        };

        // 读取文件
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                return ToolResult {
                    output: format!("读取文件失败: {}", e),
                    is_error: true,
                    images: vec![],
                };
            }
        };

        // 检查匹配次数
        let count = content.matches(&params.old_string).count();
        if count == 0 {
            return ToolResult {
                output: "未找到匹配的字符串".to_string(),
                is_error: true,
                images: vec![],
            };
        }
        if count > 1 {
            return ToolResult {
                output: format!(
                    "old_string 在文件中匹配了 {} 次，必须唯一匹配。请提供更多上下文使其唯一",
                    count
                ),
                is_error: true,
                images: vec![],
            };
        }

        // 执行替换
        let new_content = content.replacen(&params.old_string, &params.new_string, 1);
        match std::fs::write(&path, &new_content) {
            Ok(_) => {
                let diff_output = build_line_diff(&params.old_string, &params.new_string);
                let output = format!("已编辑文件: {}\n{}", path, diff_output);
                ToolResult {
                    output,
                    is_error: false,
                    images: vec![],
                }
            }
            Err(e) => ToolResult {
                output: format!("写入文件失败: {}", e),
                is_error: true,
                images: vec![],
            },
        }
    }

    fn requires_confirmation(&self) -> bool {
        true
    }

    fn confirmation_message(&self, arguments: &str) -> String {
        if let Ok(params) = serde_json::from_str::<EditFileParams>(arguments) {
            let path = expand_tilde(&params.path);
            let first_line = params.old_string.lines().next().unwrap_or("");
            let has_more = params.old_string.lines().count() > 1;
            let preview = if has_more {
                format!("{}...", first_line)
            } else {
                first_line.to_string()
            };
            format!("即将编辑文件 {} (替换: \"{}\")", path, preview)
        } else {
            "即将编辑文件".to_string()
        }
    }
}
