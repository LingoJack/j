use crate::command::chat::tools::{
    Tool, ToolResult, expand_tilde, parse_tool_args, schema_to_tool_params,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

/// GlobTool 参数
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct GlobParams {
    /// File glob pattern to match (e.g. **/*.js, *.{ts,tsx}, src/**/*.py)
    pattern: String,
    /// Directory path to search. Defaults to current working directory if not specified. Important: omit this field if not needed — do not enter undefined, null, or empty string
    #[serde(default)]
    path: Option<String>,
    /// Maximum number of results to return, default 100
    #[serde(default = "default_limit")]
    limit: usize,
    /// Skip the first N results, for pagination with limit
    #[serde(default)]
    offset: usize,
    /// File glob pattern to exclude (e.g. **/node_modules/**, **/.git/**)
    #[serde(default)]
    exclude_pattern: Option<String>,
}

fn default_limit() -> usize {
    100
}

pub struct GlobTool;

impl Tool for GlobTool {
    fn name(&self) -> &str {
        "Glob"
    }

    fn description(&self) -> &str {
        r###"
        - Fast file pattern matching tool, works with any codebase size
        - Supports glob patterns like "**/*.js" or "src/**/*.tsx"
        - Returns matching file paths sorted by modification time
        - Use this tool when you need to find files by name pattern, e.g. "src/components/**/*.tsx"
        - When you are doing an open-ended search that may require multiple rounds of glob and grep, use the Agent tool instead
        - You can call multiple tools in a single response. If multiple file patterns may be useful, run glob searches in parallel
        - Important: if no path is needed, omit the field entirely — do not enter "undefined", "null", or empty string
        "###
    }

    fn parameters_schema(&self) -> Value {
        schema_to_tool_params::<GlobParams>()
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let params: GlobParams = match parse_tool_args(arguments) {
            Ok(p) => p,
            Err(e) => return e,
        };

        // 获取搜索路径，默认为当前目录
        let base_path = params
            .path
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(expand_tilde)
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .map(|d| d.to_string_lossy().to_string())
                    .unwrap_or_else(|_| ".".to_string())
            });

        let limit = params.limit.clamp(1, 1000);

        // 解析排除模式
        let exclude_pattern = params
            .exclude_pattern
            .as_deref()
            .filter(|s| !s.is_empty())
            .and_then(|p| glob::Pattern::new(p).ok());

        // 构建完整的 glob 模式
        let full_pattern = if params.pattern.starts_with('/') {
            params.pattern.clone()
        } else {
            format!("{}/{}", base_path.trim_end_matches('/'), params.pattern)
        };

        // 执行 glob 搜索
        let mut matches: Vec<std::path::PathBuf> = match glob::glob(&full_pattern) {
            Ok(paths) => paths
                .filter_map(Result::ok)
                .filter(|path| {
                    // 应用排除模式过滤
                    if let Some(ref exclude) = exclude_pattern {
                        let path_str = path.to_string_lossy();
                        !exclude.matches(&path_str)
                    } else {
                        true
                    }
                })
                .collect(),
            Err(e) => {
                return ToolResult {
                    output: format!("glob 模式无效: {}", e),
                    is_error: true,
                    images: vec![],
                };
            }
        };

        // 按修改时间排序（最新的在前）
        matches.sort_by(|a, b| {
            let meta_a = std::fs::metadata(a);
            let meta_b = std::fs::metadata(b);
            let time_a = meta_a.ok().and_then(|m| m.modified().ok());
            let time_b = meta_b.ok().and_then(|m| m.modified().ok());
            time_b.cmp(&time_a)
        });

        let total = matches.len();

        if total == 0 {
            return ToolResult {
                output: format!("未找到匹配 '{}' 的文件", params.pattern),
                is_error: false,
                images: vec![],
            };
        }

        // 应用分页
        let paginated: Vec<_> = matches
            .into_iter()
            .skip(params.offset)
            .take(limit)
            .collect();

        let displayed = paginated.len();

        // 格式化输出
        let mut result = String::new();
        result.push_str(&format!("找到 {} 个匹配文件", total));
        if params.offset > 0 || params.offset + displayed < total {
            result.push_str(&format!(
                "（显示 {}-{} 项，共 {} 项）",
                params.offset + 1,
                params.offset + displayed,
                total
            ));
        }
        result.push_str(":\n\n");

        for path in paginated {
            result.push_str(&format!("{}\n", path.display()));
        }

        if params.offset + displayed < total {
            result.push_str(&format!(
                "\n... 还有 {} 个结果未显示（使用 offset={} 继续查看）",
                total - params.offset - displayed,
                params.offset + displayed
            ));
        }

        ToolResult {
            output: result,
            is_error: false,
            images: vec![],
        }
    }

    fn requires_confirmation(&self) -> bool {
        false
    }
}
