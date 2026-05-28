//! 文件渲染器：把单个文件转换为前端可消费的 JSON payload。
//!
//! 当前实现：
//! - `MarkdownRenderer` — 复用 `crate::markdown::parser::parse_markdown` 产出 IR
//! - `PlainTextRenderer` — 其它格式的兜底
//!
//! 未来扩展（接口已预留，实现待补）：
//! - `PptxRenderer` / `DocxRenderer` / `XlsxRenderer`
//!
//! 选型策略由 [`pick_renderer`] 根据文件扩展名决定。

use serde::Serialize;
use std::path::Path;

/// 文档类型 — 用于前端按类型分发组件。
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DocKind {
    Markdown,
    PlainText,
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
    /// 编辑器初始内容（textarea 的 value）
    pub source: String,
    /// 类型特定的载荷（首次解析的快照）：
    /// - Markdown → `ParsedDocument` JSON
    /// - PlainText → `null`（前端不需要）
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

/// 根据文件扩展名选择渲染器。
pub fn pick_renderer(path: &Path) -> Box<dyn Renderer> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "md" | "markdown" => Box::new(MarkdownRenderer),
        _ => Box::new(PlainTextRenderer),
    }
}

/// 读取并渲染单文件，构建 `RenderedDoc`。
///
/// 调用方需提前完成路径校验（存在性、是普通文件、大小上限）。
pub fn render_file(path: &Path) -> Result<RenderedDoc, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("读取文件失败：{e}"))?;
    let renderer = pick_renderer(path);

    // Markdown 必须 UTF-8；其它类型走 lossy 转换以兼容偶发 BOM / 非 UTF-8 文件
    let source = match renderer.kind() {
        DocKind::Markdown => std::str::from_utf8(&bytes)
            .map_err(|e| format!("文件不是合法的 UTF-8 编码：{e}"))?
            .to_string(),
        _ => String::from_utf8_lossy(&bytes).into_owned(),
    };

    let payload = renderer.render(&source)?;
    let filename = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("untitled")
        .to_string();

    Ok(RenderedDoc {
        path: path.display().to_string(),
        filename,
        kind: renderer.kind(),
        source,
        payload,
    })
}
