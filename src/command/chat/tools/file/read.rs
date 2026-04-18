use crate::command::chat::tools::{
    ImageData, PlanDecision, Tool, ToolResult, parse_tool_args, resolve_path, schema_to_tool_params,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::path::Path;
use std::sync::{Arc, atomic::AtomicBool};

/// 图片文件扩展名
const IMAGE_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "bmp", "ico", "tiff", "tif",
];

/// 检测文件是否为图片
fn is_image_file(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| IMAGE_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// 根据扩展名获取图片 MIME 类型
fn image_media_type(path: &str) -> &'static str {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "tiff" | "tif" => "image/tiff",
        _ => "application/octet-stream",
    }
}

/// ReadFileTool 参数
#[derive(Deserialize, JsonSchema)]
struct ReadFileParams {
    /// File path to read (absolute or relative to current working directory)
    path: String,
    /// Starting line number (0-based, i.e. 0 = first line). Omit to start from the beginning
    #[serde(default)]
    offset: Option<usize>,
    /// Number of lines to read. Omit to read to end of file
    #[serde(default)]
    limit: Option<usize>,
}

/// 读取文件的工具
#[derive(Debug)]
pub struct ReadFileTool;

impl ReadFileTool {
    pub const NAME: &'static str = "Read";
}

impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn description(&self) -> &str {
        r#"
        Reads a file from the local filesystem. You can access any file directly by using this tool.

        Usage:
        - The path parameter can be absolute or relative to the current working directory
        - By default, it reads the entire file with line numbers
        - You can optionally specify offset and limit for large files, but it's recommended to read the whole file first
        - Results are returned with line numbers (1-based)
        - This tool can read image files (PNG, JPG, GIF, WEBP, BMP). When reading an image, the contents are presented visually
        - This tool can only read files, not directories. To list a directory, use `ls` via the Bash tool
        - You will regularly be asked to read screenshots. If the user provides a path to a screenshot, ALWAYS use this tool to view the file
        - You can call multiple tools in a single response. It is always better to speculatively read multiple potentially useful files in parallel
        "#
    }

    fn parameters_schema(&self) -> Value {
        schema_to_tool_params::<ReadFileParams>()
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let params: ReadFileParams = match parse_tool_args(arguments) {
            Ok(p) => p,
            Err(e) => return e,
        };

        let path = resolve_path(&params.path);

        // 图片文件：读取为 base64，返回图片数据
        if is_image_file(&path) {
            return read_image_file(&path);
        }

        match std::fs::read_to_string(&path) {
            Ok(content) => {
                let lines: Vec<&str> = content.lines().collect();
                let total = lines.len();
                let start = params.offset.unwrap_or(0).min(total);
                let count = params.limit.unwrap_or(total - start).min(total - start);
                let selected: Vec<String> = lines[start..start + count]
                    .iter()
                    .enumerate()
                    .map(|(i, line)| format!("{:>4}│ {}", start + i + 1, line))
                    .collect();
                let mut result = selected.join("\n");

                if start + count < total {
                    result.push_str(&format!("\n...(还有 {} 行未显示)", total - start - count));
                }

                ToolResult {
                    output: result,
                    is_error: false,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                }
            }
            Err(e) => ToolResult {
                output: format!("读取文件失败: {}", e),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            },
        }
    }

    fn requires_confirmation(&self) -> bool {
        false
    }
}

/// 读取图片文件，返回 base64 图片数据
fn read_image_file(path: &str) -> ToolResult {
    use crate::util::log::write_info_log;
    use base64::Engine;

    match std::fs::read(path) {
        Ok(bytes) => {
            let size_kb = bytes.len() as f64 / 1024.0;
            let media_type = image_media_type(path);
            let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);

            write_info_log(
                "ReadFileTool",
                &format!(
                    "读取图片文件: {}, 大小: {:.1} KB, 类型: {}, base64长度: {}",
                    path,
                    size_kb,
                    media_type,
                    b64.len()
                ),
            );

            let output = format!(
                "图片文件: {}\n大小: {:.1} KB\n类型: {}",
                path, size_kb, media_type
            );

            ToolResult {
                output,
                is_error: false,
                images: vec![ImageData {
                    base64: b64,
                    media_type: media_type.to_string(),
                }],
                plan_decision: PlanDecision::None,
            }
        }
        Err(e) => ToolResult {
            output: format!("读取图片文件失败: {}", e),
            is_error: true,
            images: vec![],
            plan_decision: PlanDecision::None,
        },
    }
}
