use crate::command::chat::agent::thread_identity::current_agent_name;
use crate::command::chat::teammate::acquire_global_file_lock;
use crate::command::chat::tools::{
    PlanDecision, Tool, ToolResult, parse_tool_args, resolve_path, schema_to_tool_params,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, atomic::AtomicBool};

/// 计算批量替换的确认 token。
/// 绑定 (path, old_string, new_string, 当前文件内容)：AI 必须先调用预览才能拿到它；
/// 且文件在两次调用之间若发生变化，token 会失配，强制重新预览。
fn compute_confirm_token(path: &str, old: &str, new: &str, content: &str) -> String {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    old.hash(&mut hasher);
    new.hash(&mut hasher);
    content.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

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

/// 返回文件中 old_string 所有匹配的起始行号（0-indexed）
fn find_match_line_numbers(content: &str, old_string: &str) -> Vec<usize> {
    let mut positions = Vec::new();
    if old_string.is_empty() {
        return positions;
    }
    let mut search_start = 0;
    while let Some(pos) = content[search_start..].find(old_string) {
        let abs_pos = search_start + pos;
        let line_num = content[..abs_pos].matches('\n').count();
        positions.push(line_num);
        search_start = abs_pos + old_string.len();
    }
    positions
}

/// 生成带上下文的匹配预览（用于失败反馈，帮助模型定位差异）
fn format_match_context(content: &str, old_string: &str, max_matches: usize) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let positions = find_match_line_numbers(content, old_string);
    let match_span = old_string.lines().count().max(1);
    const CONTEXT_LINES: usize = 2;

    let mut output = String::new();
    for (i, &line_num) in positions.iter().enumerate().take(max_matches) {
        let ctx_start = line_num.saturating_sub(CONTEXT_LINES);
        let ctx_end = (line_num + match_span + CONTEXT_LINES).min(lines.len());
        output.push_str(&format!(
            "\n── 匹配 #{} (行 {}-{}) ──\n",
            i + 1,
            line_num + 1,
            line_num + match_span
        ));
        for (l, text) in lines.iter().enumerate().take(ctx_end).skip(ctx_start) {
            let in_match = l >= line_num && l < line_num + match_span;
            let marker = if in_match { ">>" } else { "  " };
            output.push_str(&format!("{} {:>5}│ {}\n", marker, l + 1, text));
        }
    }
    if positions.len() > max_matches {
        output.push_str(&format!(
            "\n…还有 {} 处匹配未在上方列出\n",
            positions.len() - max_matches
        ));
    }
    output
}

/// EditFileTool 参数
#[derive(Deserialize, JsonSchema)]
struct EditFileParams {
    /// File path to edit
    path: String,
    /// Original string to replace (must be unique unless replace_all=true)
    old_string: String,
    /// Replacement string; empty string means delete
    #[serde(default)]
    new_string: String,
    /// 批量替换：为 true 时替换文件中所有匹配项，跳过唯一性检查
    /// 首次调用仅返回匹配预览与一个 confirm_token，必须再次调用并回传该 token 才会真正执行
    #[serde(default)]
    replace_all: bool,
    /// 批量替换的一次性确认 token。由预览响应生成，AI 无法预先猜出；
    /// 原样回传即可完成二次确认。文件变化会使 token 失效
    #[serde(default)]
    confirm_token: Option<String>,
}

/// 编辑文件的工具（基于字符串替换）
#[derive(Debug)]
pub struct EditFileTool;

impl EditFileTool {
    pub const NAME: &'static str = "Edit";
}

impl Tool for EditFileTool {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn description(&self) -> &str {
        r#"
        Performs exact string replacements in files.

        Usage:
        - You must use your Read tool at least once in the conversation before editing. Read the file first to understand its content
        - When editing text from Read tool output, ensure you preserve the exact indentation (tabs/spaces). The line number prefix format is: number + │. Everything after │ is the actual file content to match. Never include any part of the line number prefix in old_string or new_string
        - ALWAYS prefer editing existing files in the codebase. NEVER write new files unless explicitly required
        - Only use emojis if the user explicitly requests it
        - The edit will FAIL if old_string is not unique in the file (unless replace_all=true). Either provide a larger string with more surrounding context to make it unique, break the edit into smaller unique chunks, or set replace_all=true to replace every occurrence
        - If new_string is empty, the matched content is deleted
        - Bulk replacement is a two-step protocol enforced by a one-time token:
          (1) Call with replace_all=true (no confirm_token) — the tool returns a preview of every match AND a confirm_token. The file is NOT modified.
          (2) After inspecting the preview, call again with replace_all=true and confirm_token set to the exact token value from step 1. The token cannot be guessed without seeing the preview, and becomes invalid if the file changes between steps (forcing a fresh preview)
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

        let path = resolve_path(&params.path);

        // 文件编辑互斥锁（多 agent 模式下防止同时编辑同一文件）
        let agent_name = current_agent_name();
        let file_path = std::path::Path::new(&path);
        let _lock_guard = match acquire_global_file_lock(file_path, &agent_name) {
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

        // 读取文件
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                return ToolResult {
                    output: format!("读取文件失败: {}", e),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                };
            }
        };

        // 空 old_string 保护（replacen("", ..) 行为不符合直觉）
        if params.old_string.is_empty() {
            return ToolResult {
                output: "old_string 不能为空".to_string(),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            };
        }

        // 检查匹配次数
        let count = content.matches(&params.old_string).count();
        if count == 0 {
            // 诊断提示：帮助模型快速判断差异来源
            let total_lines = content.lines().count();
            let first_line = params.old_string.lines().next().unwrap_or("");
            let trimmed_hits = if !first_line.trim().is_empty() {
                content.matches(first_line.trim()).count()
            } else {
                0
            };
            let mut hint = String::from("未找到匹配的字符串。常见原因：\n");
            hint.push_str("- 缩进或空白字符不一致（tab vs 空格、行尾空格、CRLF/LF）\n");
            hint.push_str("- 遗漏或多出换行符\n");
            hint.push_str("- 大小写或标点差异\n");
            hint.push_str(&format!(
                "- 文件共 {} 行，请先用 Read 确认当前内容\n",
                total_lines
            ));
            if trimmed_hits > 0 {
                hint.push_str(&format!(
                    "提示：去除首行首尾空白后可匹配到 {} 处，可能是缩进不一致导致\n",
                    trimmed_hits
                ));
            }
            return ToolResult {
                output: hint,
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            };
        }
        if count > 1 && !params.replace_all {
            // 提供带上下文的匹配预览（最多 3 处），帮助模型选择更具区分度的上下文
            let preview = format_match_context(&content, &params.old_string, 3);
            return ToolResult {
                output: format!(
                    "old_string 在文件中匹配了 {} 次，必须唯一匹配。可选方案：\n\
                     1) 在 old_string 中加入更多上下文使其唯一\n\
                     2) 使用两步批量替换：先传 replace_all=true 获取预览与 confirm_token，再带上该 token 执行\n\
                     \n以下是前若干处匹配的上下文（>> 标记命中行）：\n{}",
                    count, preview
                ),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            };
        }

        // 批量替换的二次确认握手（基于一次性 token，AI 无法跳过预览）
        if params.replace_all {
            let expected_token =
                compute_confirm_token(&path, &params.old_string, &params.new_string, &content);

            match params.confirm_token.as_deref() {
                // 首次调用：不带 token，仅返回预览 + 本次预览对应的 token
                None => {
                    let preview = format_match_context(&content, &params.old_string, count);
                    return ToolResult {
                        output: format!(
                            "【批量替换预览 — 未执行】\n\
                             共将替换 {} 处匹配，文件尚未修改。\n\
                             请审查下列全部匹配位置，确认无误后重新调用本工具：\n  \
                             replace_all=true，confirm_token=\"{}\"\n\
                             （该 token 与当前文件内容绑定，文件变化后会失效）\n\
                             \n匹配详情（>> 标记命中行，共 {} 处）：\n{}",
                            count, expected_token, count, preview
                        ),
                        is_error: true,
                        images: vec![],
                        plan_decision: PlanDecision::None,
                    };
                }
                // 二次调用：校验 token；不匹配即视为未看过预览或文件已变化
                Some(provided) if provided == expected_token => {
                    // 通过校验，继续往下执行
                }
                Some(_) => {
                    return ToolResult {
                        output: format!(
                            "confirm_token 无效或已过期。可能原因：\n\
                             - 未先调用预览步骤（不带 confirm_token）就直接传入了 token\n\
                             - 自上次预览以来文件、old_string 或 new_string 发生了变化\n\
                             请重新调用 replace_all=true 获取新的预览与 token（当前文件共 {} 处匹配）",
                            count
                        ),
                        is_error: true,
                        images: vec![],
                        plan_decision: PlanDecision::None,
                    };
                }
            }
        } else if params.confirm_token.is_some() {
            return ToolResult {
                output: "confirm_token 仅在 replace_all=true 时生效，单次编辑无需此参数"
                    .to_string(),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            };
        }

        // 执行替换
        let new_content = if params.replace_all {
            content.replace(&params.old_string, &params.new_string)
        } else {
            content.replacen(&params.old_string, &params.new_string, 1)
        };
        match std::fs::write(&path, &new_content) {
            Ok(_) => {
                let diff_output = build_line_diff(&params.old_string, &params.new_string);
                let summary = if params.replace_all {
                    format!("已编辑文件（批量替换 {} 处）: {}", count, path)
                } else {
                    format!("已编辑文件: {}", path)
                };
                let output = format!("{}\n{}", summary, diff_output);
                ToolResult {
                    output,
                    is_error: false,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                }
            }
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
        if let Ok(params) = serde_json::from_str::<EditFileParams>(arguments) {
            let path = resolve_path(&params.path);
            let first_line = params.old_string.lines().next().unwrap_or("");
            let has_more = params.old_string.lines().count() > 1;
            let preview = if has_more {
                format!("{}...", first_line)
            } else {
                first_line.to_string()
            };
            if params.replace_all && params.confirm_token.is_some() {
                let count = std::fs::read_to_string(&path)
                    .map(|c| c.matches(&params.old_string).count())
                    .unwrap_or(0);
                format!(
                    "【批量替换】即将在文件 {} 中替换全部 {} 处匹配 (替换: \"{}\")",
                    path, count, preview
                )
            } else {
                format!("即将编辑文件 {} (替换: \"{}\")", path, preview)
            }
        } else {
            "即将编辑文件".to_string()
        }
    }
}
