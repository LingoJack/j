use super::{Tool, ToolResult, expand_tilde};
use ignore::WalkBuilder;
use regex::RegexBuilder;
use serde_json::{Value, json};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct GrepTool;

impl Tool for GrepTool {
    fn name(&self) -> &str {
        "Grep"
    }

    fn description(&self) -> &str {
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
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regex pattern to search for (e.g. \"log.*Error\", \"function\\\\s+\\\\w+\")"
                },
                "path": {
                    "type": "string",
                    "description": "File or directory path to search. Defaults to current working directory if not specified. Important: omit this field if not needed"
                },
                "glob": {
                    "type": "string",
                    "description": "Glob pattern to filter files (e.g. \"*.js\", \"*.{ts,tsx}\", \"src/**/*.py\")"
                },
                "type": {
                    "type": "string",
                    "description": "File type to search (e.g. \"js\", \"py\", \"rust\", \"go\", \"java\"). More efficient than glob"
                },
                "output_mode": {
                    "type": "string",
                    "enum": ["content", "files_with_matches", "count"],
                    "description": "Output mode: \"content\" shows matching lines with line numbers (default), \"files_with_matches\" returns file paths only, \"count\" returns match counts"
                },
                "head_limit": {
                    "type": "integer",
                    "description": "Limit the number of output results"
                },
                "offset": {
                    "type": "integer",
                    "description": "Skip the first N results, for pagination"
                },
                "context": {
                    "type": "integer",
                    "description": "Show N lines of context around each match (before and after)"
                },
                "ignore_case": {
                    "type": "boolean",
                    "description": "Case-insensitive search"
                }
            },
            "required": ["pattern"]
        })
    }

    fn execute(&self, arguments: &str, cancelled: &Arc<AtomicBool>) -> ToolResult {
        let v = match serde_json::from_str::<Value>(arguments) {
            Ok(v) => v,
            Err(e) => {
                return ToolResult {
                    output: format!("参数解析失败: {}", e),
                    is_error: true,
                    images: vec![],
                };
            }
        };

        let pattern = match v.get("pattern").and_then(|p| p.as_str()) {
            Some(p) => p,
            None => {
                return ToolResult {
                    output: "参数缺少 pattern 字段".to_string(),
                    is_error: true,
                    images: vec![],
                };
            }
        };

        // 构建正则表达式
        let ignore_case = v.get("ignore_case").and_then(|i| i.as_bool()) == Some(true);
        let re = match RegexBuilder::new(pattern)
            .case_insensitive(ignore_case)
            .build()
        {
            Ok(re) => re,
            Err(e) => {
                return ToolResult {
                    output: format!("正则表达式无效: {}", e),
                    is_error: true,
                    images: vec![],
                };
            }
        };

        // 搜索路径
        let search_path = v
            .get("path")
            .and_then(|p| p.as_str())
            .filter(|s| !s.is_empty())
            .map(expand_tilde)
            .unwrap_or_else(|| ".".to_string());
        let search_path = Path::new(&search_path);

        // 输出模式
        let output_mode = v
            .get("output_mode")
            .and_then(|m| m.as_str())
            .unwrap_or("content");

        // glob 过滤
        let glob_pattern = v.get("glob").and_then(|g| g.as_str());

        // 文件类型过滤
        let file_type = v.get("type").and_then(|t| t.as_str());
        let type_extensions: Vec<&str> = file_type.map(get_extensions_for_type).unwrap_or_default();

        // head_limit
        let head_limit = v
            .get("head_limit")
            .and_then(|l| l.as_u64())
            .map(|l| l as usize);

        // offset
        let offset = v.get("offset").and_then(|o| o.as_u64()).unwrap_or(0) as usize;

        // context
        let context = v.get("context").and_then(|c| c.as_u64()).unwrap_or(0) as usize;

        // 构建文件遍历器（自动处理 .gitignore）
        let mut walker = WalkBuilder::new(search_path);
        walker
            .hidden(false) // 搜索隐藏文件
            .git_ignore(true) // 尊重 .gitignore
            .git_global(true)
            .git_exclude(true);

        // 应用 glob 过滤
        if let Some(glob) = glob_pattern.and_then(|g| glob::Pattern::new(g).ok()) {
            let globber = std::sync::Arc::new(glob);
            walker.filter_entry(move |entry| {
                let path = entry.path();
                if path.is_dir() {
                    return true;
                }
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    return globber.matches(name);
                }
                false
            });
        }

        // 收集结果
        let mut matches: Vec<String> = Vec::new();
        let mut file_matches: Vec<String> = Vec::new();
        let mut total_count: usize = 0;

        for entry in walker.build() {
            if cancelled.load(Ordering::Relaxed) {
                return ToolResult {
                    output: "[已取消]".to_string(),
                    is_error: true,
                    images: vec![],
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

            // 文件类型过滤
            if !type_extensions.is_empty() {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !type_extensions.iter().any(|&e| e == ext || e == filename) {
                    continue;
                }
            }

            // 检查 head_limit（对于 files_with_matches 模式）
            if output_mode == "files_with_matches"
                && head_limit.map(|l| file_matches.len() >= l).unwrap_or(false)
            {
                break;
            }

            // 读取文件并搜索
            let file = match File::open(path) {
                Ok(f) => f,
                Err(_) => continue,
            };

            let reader = BufReader::new(file);
            let lines: Vec<String> = reader.lines().map_while(Result::ok).collect();
            let path_str = path.display().to_string();

            let mut file_has_match = false;
            let mut file_count = 0;

            for (line_num, line) in lines.iter().enumerate() {
                if re.is_match(line) {
                    file_has_match = true;
                    file_count += 1;
                    total_count += 1;

                    if output_mode == "content" {
                        // 检查 head_limit
                        if head_limit.map(|l| matches.len() >= l).unwrap_or(false) {
                            break;
                        }

                        let mut result_line = format!("{}:{}:{}", path_str, line_num + 1, line);

                        // 添加上下文
                        if context > 0 {
                            let start = line_num.saturating_sub(context);
                            let end = (line_num + context + 1).min(lines.len());
                            let mut context_lines = Vec::new();
                            for (i, ctx_line) in lines.iter().enumerate().take(end).skip(start) {
                                if i != line_num {
                                    context_lines.push(format!(
                                        "{}-{}:{}",
                                        path_str,
                                        i + 1,
                                        ctx_line
                                    ));
                                }
                            }
                            if !context_lines.is_empty() {
                                result_line =
                                    format!("{}\n{}", result_line, context_lines.join("\n"));
                            }
                        }

                        matches.push(result_line);
                    }
                }
            }

            if output_mode == "files_with_matches" && file_has_match {
                file_matches.push(path_str);
            } else if output_mode == "count" && file_count > 0 {
                file_matches.push(format!("{}:{}", path_str, file_count));
            }
        }

        // 构建输出
        if output_mode == "files_with_matches" {
            if file_matches.is_empty() {
                return ToolResult {
                    output: format!("未找到匹配 '{}' 的文件", pattern),
                    is_error: false,
                    images: vec![],
                };
            }
            let total = file_matches.len();
            let results: Vec<&str> = file_matches
                .iter()
                .skip(offset)
                .take(head_limit.unwrap_or(usize::MAX))
                .map(String::as_str)
                .collect();
            let mut output = format!("找到 {} 个匹配文件", total);
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
            ToolResult {
                output,
                is_error: false,
                images: vec![],
            }
        } else if output_mode == "count" {
            if file_matches.is_empty() {
                return ToolResult {
                    output: format!("未找到匹配 '{}' 的内容", pattern),
                    is_error: false,
                    images: vec![],
                };
            }
            let mut output = format!("共 {} 处匹配:\n\n", total_count);
            output.push_str(&file_matches.join("\n"));
            ToolResult {
                output,
                is_error: false,
                images: vec![],
            }
        } else {
            if matches.is_empty() {
                return ToolResult {
                    output: format!("未找到匹配 '{}' 的内容", pattern),
                    is_error: false,
                    images: vec![],
                };
            }
            let total = matches.len();
            let results: Vec<&str> = matches
                .iter()
                .skip(offset)
                .take(head_limit.unwrap_or(usize::MAX))
                .map(String::as_str)
                .collect();
            let mut output = format!("找到 {} 个匹配", total);
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
            ToolResult {
                output,
                is_error: false,
                images: vec![],
            }
        }
    }

    fn requires_confirmation(&self) -> bool {
        false
    }
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
