use super::{
    PlanDecision, Tool, ToolResult, effective_cwd, parse_tool_args, resolve_path,
    schema_to_tool_params,
};
use grep_matcher::Matcher;
use grep_regex::RegexMatcherBuilder;
use grep_searcher::{SearcherBuilder, Sink, SinkContext, SinkContextKind, SinkFinish, SinkMatch};
use ignore::WalkBuilder;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::borrow::Cow;
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
        - ALWAYS use Grep for content search tasks. NEVER invoke `grep` or `rg` as a Shell command
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

        let matcher = match RegexMatcherBuilder::new()
            .case_insensitive(params.ignore_case)
            .build(&params.pattern)
        {
            Ok(m) => m,
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
        let mut searcher = SearcherBuilder::new()
            .line_number(true)
            .before_context(params.context)
            .after_context(params.context)
            .build();

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
                &matcher,
                &mut searcher,
                &params.output_mode,
                params.head_limit,
                cancelled,
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
fn search_single_file<M: Matcher>(
    path: &Path,
    matcher: &M,
    searcher: &mut grep_searcher::Searcher,
    output_mode: &str,
    head_limit: Option<usize>,
    cancelled: &Arc<AtomicBool>,
    results: &mut SearchResults,
) {
    let path_str = path.display().to_string();
    let mut sink = GrepSink {
        path_str: &path_str,
        output_mode,
        head_limit,
        cancelled,
        results,
        file_has_match: false,
        file_count: 0,
        // 缓冲当前匹配行的上下文行（匹配行之前收集）
        pending_context: Vec::new(),
    };

    // search_path 可能因二进制文件等跳过，忽略错误
    let _ = searcher.search_path(matcher, path, &mut sink);
}

/// 自定义 Sink，收集 ripgrep 搜索结果
struct GrepSink<'a> {
    path_str: &'a str,
    output_mode: &'a str,
    head_limit: Option<usize>,
    cancelled: &'a Arc<AtomicBool>,
    results: &'a mut SearchResults,
    file_has_match: bool,
    file_count: usize,
    /// 匹配行之前的上下文行缓冲
    pending_context: Vec<String>,
}

impl GrepSink<'_> {
    /// 检查是否应该终止搜索
    fn should_stop(&self) -> bool {
        if self.cancelled.load(Ordering::Relaxed) {
            return true;
        }
        // content 模式下 head_limit 已满足
        if self.output_mode == "content"
            && self
                .head_limit
                .is_some_and(|l| self.results.line_matches.len() >= l)
        {
            return true;
        }
        false
    }
}

impl Sink for GrepSink<'_> {
    type Error = std::io::Error;

    fn matched(
        &mut self,
        _searcher: &grep_searcher::Searcher,
        mat: &SinkMatch<'_>,
    ) -> Result<bool, Self::Error> {
        if self.should_stop() {
            return Ok(false);
        }

        self.file_has_match = true;
        self.file_count += 1;
        self.results.total_count += 1;

        if self.output_mode == "content" {
            if self
                .head_limit
                .is_some_and(|l| self.results.line_matches.len() >= l)
            {
                return Ok(false);
            }

            // 先刷新之前缓冲的上下文行
            let context_lines: Vec<String> = self.pending_context.drain(..).collect();

            let line_num = mat.line_number().unwrap_or(0);
            let line_text = std::str::from_utf8(mat.bytes())
                .unwrap_or("")
                .trim_end_matches('\n')
                .trim_end_matches('\r');
            let matched_line = format!("{}:{}:{}", self.path_str, line_num, line_text);

            // 上下文行 + 匹配行 组合
            if context_lines.is_empty() {
                self.results.line_matches.push(matched_line);
            } else {
                let mut block = context_lines;
                block.push(matched_line);
                self.results.line_matches.push(
                    block
                        .iter()
                        .map(String::as_str)
                        .collect::<Vec<_>>()
                        .join("\n"),
                );
            }
        } else if self.output_mode == "files_with_matches" {
            // 只需知道有匹配即可
            return Ok(false);
        }
        // count 模式只计数

        Ok(true)
    }

    fn context(
        &mut self,
        _searcher: &grep_searcher::Searcher,
        context: &SinkContext<'_>,
    ) -> Result<bool, Self::Error> {
        if self.should_stop() {
            return Ok(false);
        }

        if self.output_mode != "content" {
            return Ok(true);
        }

        let line_num = context.line_number().unwrap_or(0);
        let line_text = std::str::from_utf8(context.bytes())
            .unwrap_or("")
            .trim_end_matches('\n')
            .trim_end_matches('\r');

        // 使用 `-` 分隔符标识上下文行（与原实现一致）
        let ctx_line = format!("{}-{}:{}", self.path_str, line_num, line_text);

        match context.kind() {
            SinkContextKind::Before => {
                // Before context 缓冲，等匹配行时一起输出
                self.pending_context.push(ctx_line);
            }
            SinkContextKind::After => {
                // After context 直接追加到最近的匹配行
                if let Some(last) = self.results.line_matches.last_mut() {
                    last.push_str(&format!("\n{}", ctx_line));
                }
            }
            SinkContextKind::Other => {}
        }

        Ok(true)
    }

    fn context_break(&mut self, _searcher: &grep_searcher::Searcher) -> Result<bool, Self::Error> {
        // 文件内不连续匹配之间的分隔，清空 pending context
        self.pending_context.clear();
        Ok(true)
    }

    fn finish(
        &mut self,
        _searcher: &grep_searcher::Searcher,
        _: &SinkFinish,
    ) -> Result<(), Self::Error> {
        if self.output_mode == "files_with_matches" && self.file_has_match {
            self.results.file_entries.push(self.path_str.to_string());
        } else if self.output_mode == "count" && self.file_count > 0 {
            self.results
                .file_entries
                .push(format!("{}:{}", self.path_str, self.file_count));
        }
        Ok(())
    }
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
