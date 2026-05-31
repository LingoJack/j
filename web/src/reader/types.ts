// Rust IR JSON 类型定义 — 与 `src/markdown/ir.rs` 一一对应。
// IR 由 `parse_markdown` 生成，通过 `/api/file`、`/api/parse` 返回给前端。
//
// 序列化形式（adjacently-tagged）：
//   { "type": "<variant>", "value": <payload> }
// 单元变体（rule / soft_break / hard_break）没有 `value` 字段。

export type Inline =
  | { type: 'text'; value: string }
  | { type: 'strong'; value: Inline[] }
  | { type: 'emphasis'; value: Inline[] }
  | { type: 'strikethrough'; value: Inline[] }
  | { type: 'code'; value: string }
  | { type: 'link'; value: { text: Inline[]; url: string } }
  | { type: 'image'; value: { url: string; alt: string } }
  | { type: 'soft_break' }
  | { type: 'hard_break' }

export type Alignment = 'none' | 'left' | 'center' | 'right'

export interface ListItem {
  checked: boolean | null
  content: Inline[]
  children: Block[]
}

export interface ListData {
  ordered: boolean
  start_index: number | null
  items: ListItem[]
}

export interface TableData {
  alignments: Alignment[]
  rows: Inline[][][]
}

export type BlockKind =
  | { type: 'paragraph'; value: Inline[] }
  | { type: 'heading'; value: { level: number; content: Inline[] } }
  | { type: 'code_block'; value: { lang: string; code: string } }
  | { type: 'table'; value: TableData }
  | { type: 'list'; value: ListData }
  | { type: 'block_quote'; value: Block[] }
  | { type: 'rule' }

export interface Block {
  source: { start_line: number; end_line: number }
  kind: BlockKind
}

export interface ParsedDocument {
  blocks: Block[]
}

// `/api/file` 响应：单文件渲染产物
export type DocKind = 'markdown' | 'plain_text' | 'pptx' | 'docx' | 'xlsx'

export interface RenderedDoc {
  path: string
  filename: string
  kind: DocKind
  source: string
  payload: ParsedDocument | null | unknown
}

// `/api/list` 响应：目录列出结果
export interface DirEntry {
  name: string
  path: string
  is_dir: boolean
  size: number
}

export interface ListResp {
  dir: string
  parent: string | null
  entries: DirEntry[]
  truncated: boolean
}

// `/api/initial` 响应
export interface InitialResp {
  /** 目录入口时为 null */
  initial_path: string | null
  root_dir: string
}

// 编辑器内部使用的 Tab 状态。
// **不要**把 `source` / `doc` 放进来。
// 输入文字会以高频率（每个按键）变化，如果走 setState，整个 Reader / TabBar /
// FileTree 都会跟着 re-render，大文件下会很卡。
// 改放在 Reader.tsx 的 useRef 桶里：
//   sourcesRef.current[path]  →  最新 markdown / plain text 内容
//   docsRef.current[path]     →  最近一次 /api/parse 的 IR（仅 TOC 使用）
// 只有 dirty / saving / error 这种 UI 必须感知的字段保留在 state 里。
export interface Tab {
  path: string
  filename: string
  kind: DocKind
  /** 是否有未保存改动 */
  dirty: boolean
  saving: 'idle' | 'saving' | 'error'
  error?: string
}
