//! 文件渲染器：把单个文件转换为前端可消费的 JSON payload。
//!
//! 当前实现：
//! - `MarkdownRenderer` — 复用 `crate::markdown::parser::parse_markdown` 产出 IR
//! - `ImageRenderer`    — 图片直通；source 为空，payload 携带 mime / size，前端经
//!   `/api/asset` 拉原始字节
//! - `PlainTextRenderer` — 其它格式的兜底
//!
//! 未来扩展（接口已预留，实现待补）：
//! - `PptxRenderer` / `DocxRenderer` / `XlsxRenderer`
//!
//! 选型策略由 [`pick_renderer`] 根据文件扩展名决定。

use serde::Serialize;
use std::path::Path;

/// 文档类型 — 用于前端按类型分发组件。
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DocKind {
    Markdown,
    PlainText,
    Image,
    // 占位，本期不实现
    #[allow(dead_code)]
    Pptx,
    #[allow(dead_code)]
    Docx,
    #[allow(dead_code)]
    Xlsx,
}

/// `/api/file` 的响应：包含原始 source（编辑器初始内容）+ 解析后的 payload。
#[derive(Debug, Serialize)]
pub struct RenderedDoc {
    /// 文件绝对路径（用于前端 Tab 标识 + /api/save 回传）
    pub path: String,
    /// 文件名（不含路径，仅用于 UI 展示）
    pub filename: String,
    /// 文档类型
    pub kind: DocKind,
    /// 编辑器初始内容（textarea 的 value）；图片为空串
    pub source: String,
    /// 类型特定的载荷（首次解析的快照）：
    /// - Markdown → `ParsedDocument` JSON
    /// - PlainText → `null`（前端不需要）
    /// - Image    → `{ "mime": ..., "size": ... }`，前端可用于 status bar
    pub payload: serde_json::Value,
}

/// 文件渲染器抽象。
pub trait Renderer {
    fn render(&self, source: &str) -> Result<serde_json::Value, String>;
    fn kind(&self) -> DocKind;
}

/// Markdown 渲染器：复用项目内 `parse_markdown`，输出 IR JSON。
pub struct MarkdownRenderer;

impl Renderer for MarkdownRenderer {
    fn render(&self, source: &str) -> Result<serde_json::Value, String> {
        // max_width 仅影响表格分隔行预处理逻辑，传一个足够大的值即可
        let doc = crate::markdown::parser::parse_markdown(source, 120);
        serde_json::to_value(&doc).map_err(|e| format!("Markdown IR 序列化失败：{e}"))
    }

    fn kind(&self) -> DocKind {
        DocKind::Markdown
    }
}

/// 纯文本兜底渲染器：payload 为 null，前端直接用 source 渲染 textarea。
pub struct PlainTextRenderer;

impl Renderer for PlainTextRenderer {
    fn render(&self, _source: &str) -> Result<serde_json::Value, String> {
        Ok(serde_json::Value::Null)
    }

    fn kind(&self) -> DocKind {
        DocKind::PlainText
    }
}

/// 图片渲染器：不读取文件内容到 source（前端走 `/api/asset` 直接拉字节）；
/// payload 仅包含若干元数据。
pub struct ImageRenderer;

impl Renderer for ImageRenderer {
    fn render(&self, _source: &str) -> Result<serde_json::Value, String> {
        // payload 在 render_file 里组装（需要文件元数据），这里返回 null 占位。
        Ok(serde_json::Value::Null)
    }

    fn kind(&self) -> DocKind {
        DocKind::Image
    }
}

/// 根据文件扩展名选择渲染器。
pub fn pick_renderer(path: &Path) -> Box<dyn Renderer> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "md" | "markdown" => Box::new(MarkdownRenderer),
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" | "ico" | "avif" | "apng"
        | "tif" | "tiff" => Box::new(ImageRenderer),
        _ => Box::new(PlainTextRenderer),
    }
}

/// 仅按扩展名判断是否图片。给调用方在「读文件之前」选定大小上限。
pub fn is_image_path(path: &Path) -> bool {
    pick_renderer(path).kind() == DocKind::Image
}

/// 读取并渲染单文件，构建 `RenderedDoc`。
///
/// 调用方需提前完成路径校验（存在性、是普通文件、大小上限）。
pub fn render_file(path: &Path) -> Result<RenderedDoc, String> {
    let renderer = pick_renderer(path);
    let kind = renderer.kind();

    let filename = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("untitled")
        .to_string();

    // 图片：不把字节塞进 source，前端通过 /api/asset 取原始数据。
    if kind == DocKind::Image {
        let metadata = std::fs::metadata(path).map_err(|e| format!("读取图片元数据失败：{e}"))?;
        let mime = mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string();
        let payload = serde_json::json!({
            "mime": mime,
            "size": metadata.len(),
        });
        return Ok(RenderedDoc {
            path: path.display().to_string(),
            filename,
            kind,
            source: String::new(),
            payload,
        });
    }

    let bytes = std::fs::read(path).map_err(|e| format!("读取文件失败：{e}"))?;

    // Markdown 必须 UTF-8；其它类型走 lossy 转换以兼容偶发 BOM / 非 UTF-8 文件
    let source = match kind {
        DocKind::Markdown => std::str::from_utf8(&bytes)
            .map_err(|e| format!("文件不是合法的 UTF-8 编码：{e}"))?
            .to_string(),
        _ => String::from_utf8_lossy(&bytes).into_owned(),
    };

    let payload = renderer.render(&source)?;

    Ok(RenderedDoc {
        path: path.display().to_string(),
        filename,
        kind,
        source,
        payload,
    })
}
