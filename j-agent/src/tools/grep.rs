use super::{
    PlanDecision, Tool, ToolResult, effective_cwd, parse_tool_args, resolve_path,
    schema_to_tool_params,
};
use ignore::WalkBuilder;
use regex::Regex;
use regex::RegexBuilder;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::borrow::Cow;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// GrepTool 参数
#[derive(Deserialize, JsonSchema)]
struct GrepParams {
    /// Regex pattern to search for (e.g. "log.*Error", "function\\s+\\w+")
    pattern: String,
    /// File or directory path to search. Defaults to current working directory if not specified. Important: omit this field if not needed
    #[serde(default)]
    path: Option<String>,
    /// Glob pattern to filter files (e.g. "*.js", "*.{ts,tsx}", "src/**/*.py")
    #[serde(default)]
    glob: Option<String>,
    /// File type to search (e.g. "js", "py", "rust", "go", "java"). More efficient than glob
    #[serde(default, rename = "type")]
    file_type: Option<String>,
    /// Output mode: "content" shows matching lines with line numbers (default), "files_with_matches" returns file paths only, "count" returns match counts
    #[serde(default = "default_output_mode")]
    output_mode: String,
    /// Limit the number of output results
    #[serde(default)]
    head_limit: Option<usize>,
    /// Skip the first N results, for pagination
    #[serde(default)]
    offset: usize,
    /// Show N lines of context around each match (before and after)
    #[serde(default)]
    context: usize,
    /// Case-insensitive search
    #[serde(default)]
    ignore_case: bool,
}

fn default_output_mode() -> String {
    "content".to_string()
}

/// 正则搜索工具，用于在文件内容中搜索匹配的文本
#[derive(Debug)]
pub struct GrepTool;

impl GrepTool {
    pub const NAME: &'static str = "Grep";
}

impl Tool for GrepTool {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn description(&self) -> Cow<'_, str> {
        r###"
        A powerful regex-based search tool for searching within file contents.

        Usage:
        - ALWAYS use Grep for content search tasks. NEVER invoke `grep` or `rg` as a Bash command
        - Supports full regex syntax, e.g. "log.*Error", "function\s+\w+"
        - Filter files with the glob parameter (e.g. "*.js", "**/*.tsx") or the type parameter (e.g. "js", "py", "rust")
        - Output modes:
          - "content": show matching lines with line numbers (default)
          - "files_with_matches": return file paths only
          - "count": return match counts
        - Supports pagination: head_limit limits output count, offset skips the first N results
        - Use the context parameter to show N lines of context around each match
        - For finding files by name, use the Glob tool; Grep is for searching file contents
        - Use Agent tool for open-ended searches requiring multiple rounds
        - Multiple tools can be called in a single response. For independent patterns, run searches in parallel
        - Important: if no path is needed, omit the field entirely — do not enter "undefined", "null", or empty string
        "###
        .into()
    }

    fn parameters_schema(&self) -> Value {
        schema_to_tool_params::<GrepParams>()
    }

    fn execute(&self, arguments: &str, cancelled: &Arc<AtomicBool>) -> ToolResult {
        let params: GrepParams = match parse_tool_args(arguments) {
            Ok(p) => p,
            Err(e) => return e,
        };

        let re = match RegexBuilder::new(&params.pattern)
            .case_insensitive(params.ignore_case)
            .build()
        {
            Ok(re) => re,
            Err(e) => {
                return ToolResult {
                    output: format!("正则表达式无效: {}", e),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                };
            }
        };

        let search_path_str = params
            .path
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(resolve_path)
            .unwrap_or_else(effective_cwd);
        let search_path = Path::new(&search_path_str);

        let type_extensions: Vec<&str> = params
            .file_type
            .as_deref()
            .map(get_extensions_for_type)
            .unwrap_or_default();

        let walker = build_file_walker(search_path, params.glob.as_deref());

        let mut results = SearchResults::default();

        for entry in walker.build() {
            if cancelled.load(Ordering::Relaxed) {
                return ToolResult {
                    output: "[已取消]".to_string(),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                };
            }

            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            if !type_extensions.is_empty() && !matches_file_type(path, &type_extensions) {
                continue;
            }

            // 提前终止：files_with_matches 模式已收集够
            if params.output_mode == "files_with_matches"
                && params
                    .head_limit
                    .is_some_and(|l| results.file_entries.len() >= l)
            {
                break;
            }

            search_single_file(
                path,
                &re,
                &params.output_mode,
                params.context,
                params.head_limit,
                &mut results,
            );
        }

        format_grep_output(&params, &results)
    }

    fn requires_confirmation(&self) -> bool {
        false
    }
}

// ========== Search Result Types ==========

/// 搜索过程中收集的原始结果
#[derive(Default)]
struct SearchResults {
    /// content 模式下每个匹配行（含行号、上下文等）
    line_matches: Vec<String>,
    /// files_with_matches 模式下匹配的文件路径；count 模式下为 "path:count"
    file_entries: Vec<String>,
    /// count 模式下的总匹配数
    total_count: usize,
}

// ========== Search Helpers ==========

/// 构建文件遍历器（自动处理 .gitignore）
fn build_file_walker(root: &Path, glob_pattern: Option<&str>) -> WalkBuilder {
    let mut walker = WalkBuilder::new(root);
    walker
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true);

    if let Some(glob) = glob_pattern.and_then(|g| glob::Pattern::new(g).ok()) {
        let globber = Arc::new(glob);
        walker.filter_entry(move |entry| {
            let path = entry.path();
            if path.is_dir() {
                return true;
            }
            path.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|name| globber.matches(name))
        });
    }

    walker
}

/// 判断文件扩展名或文件名是否匹配给定的类型列表
fn matches_file_type(path: &Path, type_extensions: &[&str]) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    type_extensions.iter().any(|&e| e == ext || e == filename)
}

/// 在单个文件中搜索正则匹配，将结果写入 `results`
fn search_single_file(
    path: &Path,
    re: &Regex,
    output_mode: &str,
    context: usize,
    head_limit: Option<usize>,
    results: &mut SearchResults,
) {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return,
    };

    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().map_while(Result::ok).collect();
    let path_str = path.display().to_string();

    let mut file_has_match = false;
    let mut file_count = 0;

    for (line_num, line) in lines.iter().enumerate() {
        if !re.is_match(line) {
            continue;
        }

        file_has_match = true;
        file_count += 1;
        results.total_count += 1;

        if output_mode == "content" {
            if head_limit.is_some_and(|l| results.line_matches.len() >= l) {
                break;
            }

            let result_line = build_content_line(&path_str, line_num, line, &lines, context);
            results.line_matches.push(result_line);
        }
    }

    if output_mode == "files_with_matches" && file_has_match {
        results.file_entries.push(path_str);
    } else if output_mode == "count" && file_count > 0 {
        results
            .file_entries
            .push(format!("{}:{}", path_str, file_count));
    }
}

/// 构建单行 content 匹配结果（可选上下文行）
fn build_content_line(
    path_str: &str,
    line_num: usize,
    line: &str,
    all_lines: &[String],
    context: usize,
) -> String {
    let mut result_line = format!("{}:{}:{}", path_str, line_num + 1, line);

    if context > 0 {
        let start = line_num.saturating_sub(context);
        let end = (line_num + context + 1).min(all_lines.len());
        let ctx_lines: Vec<String> = all_lines
            .iter()
            .enumerate()
            .take(end)
            .skip(start)
            .filter(|(i, _)| *i != line_num)
            .map(|(i, l)| format!("{}-{}:{}", path_str, i + 1, l))
            .collect();

        if !ctx_lines.is_empty() {
            result_line = format!("{}\n{}", result_line, ctx_lines.join("\n"));
        }
    }

    result_line
}

// ========== Output Formatting ==========

/// 根据输出模式格式化搜索结果
fn format_grep_output(params: &GrepParams, results: &SearchResults) -> ToolResult {
    match params.output_mode.as_str() {
        "files_with_matches" => format_file_matches(params, &results.file_entries),
        "count" => format_count_output(params, &results.file_entries, results.total_count),
        _ => format_content_output(params, &results.line_matches),
    }
}

/// files_with_matches 模式输出
fn format_file_matches(params: &GrepParams, file_matches: &[String]) -> ToolResult {
    if file_matches.is_empty() {
        return empty_result(&params.pattern, "文件");
    }
    let output = paginate_and_format(
        "找到 {} 个匹配文件",
        file_matches,
        params.offset,
        params.head_limit,
    );
    ToolResult {
        output,
        is_error: false,
        images: vec![],
        plan_decision: PlanDecision::None,
    }
}

/// count 模式输出
fn format_count_output(
    params: &GrepParams,
    file_matches: &[String],
    total_count: usize,
) -> ToolResult {
    if file_matches.is_empty() {
        return empty_result(&params.pattern, "内容");
    }
    let mut output = format!("共 {} 处匹配:\n\n", total_count);
    output.push_str(&file_matches.join("\n"));
    ToolResult {
        output,
        is_error: false,
        images: vec![],
        plan_decision: PlanDecision::None,
    }
}

/// content 模式输出
fn format_content_output(params: &GrepParams, matches: &[String]) -> ToolResult {
    if matches.is_empty() {
        return empty_result(&params.pattern, "内容");
    }
    let output = paginate_and_format("找到 {} 个匹配", matches, params.offset, params.head_limit);
    ToolResult {
        output,
        is_error: false,
        images: vec![],
        plan_decision: PlanDecision::None,
    }
}

/// 无匹配时的通用结果
fn empty_result(pattern: &str, kind: &str) -> ToolResult {
    ToolResult {
        output: format!("未找到匹配 '{}' 的{}", pattern, kind),
        is_error: false,
        images: vec![],
        plan_decision: PlanDecision::None,
    }
}

/// 对列表分页并格式化输出（用于 files_with_matches / content 模式共用）
fn paginate_and_format(
    header_fmt: &str,
    items: &[String],
    offset: usize,
    head_limit: Option<usize>,
) -> String {
    let total = items.len();
    let results: Vec<&str> = items
        .iter()
        .skip(offset)
        .take(head_limit.unwrap_or(usize::MAX))
        .map(String::as_str)
        .collect();

    let mut output = header_fmt.replace("{}", &total.to_string());
    if offset > 0 || results.len() < total {
        output.push_str(&format!(
            "（显示 {}-{} 项，共 {} 项）",
            offset + 1,
            offset + results.len(),
            total
        ));
    }
    output.push_str(":\n\n");
    output.push_str(&results.join("\n"));
    output
}

/// 文件类型到扩展名的映射
fn get_extensions_for_type(file_type: &str) -> Vec<&'static str> {
    match file_type {
        "js" => vec!["js", "jsx", "mjs", "cjs"],
        "ts" => vec!["ts", "tsx"],
        "py" => vec!["py", "pyw"],
        "rust" | "rs" => vec!["rs"],
        "go" => vec!["go"],
        "java" => vec!["java"],
        "c" => vec!["c", "h"],
        "cpp" | "c++" | "cc" => vec!["cpp", "cc", "cxx", "hpp", "hh", "hxx", "h"],
        "cs" | "csharp" => vec!["cs"],
        "ruby" | "rb" => vec!["rb", "rake"],
        "php" => vec!["php"],
        "swift" => vec!["swift"],
        "kt" | "kotlin" => vec!["kt", "kts"],
        "scala" => vec!["scala", "sc"],
        "lua" => vec!["lua"],
        "perl" => vec!["pl", "pm", "t"],
        "shell" | "sh" | "bash" => vec!["sh", "bash", "zsh", "ksh"],
        "sql" => vec!["sql"],
        "html" => vec!["html", "htm", "xhtml"],
        "css" => vec!["css", "scss", "sass", "less"],
        "json" => vec!["json"],
        "yaml" | "yml" => vec!["yaml", "yml"],
        "xml" => vec!["xml", "xsl", "xslt", "svg"],
        "markdown" | "md" => vec!["md", "markdown"],
        "toml" => vec!["toml"],
        "docker" | "dockerfile" => vec!["Dockerfile", "dockerfile"],
        _ => vec![],
    }
}
