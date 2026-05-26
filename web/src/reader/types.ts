// Rust IR JSON 类型定义 — 与 `src/markdown/ir.rs` 一一对应。
// IR 由 `parse_markdown` 生成，通过 `/api/doc` 返回给前端。
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

// `/api/doc` 响应体
export type DocKind = 'markdown' | 'plain_text' | 'pptx' | 'docx' | 'xlsx'

export interface RenderedDoc {
  filename: string
  kind: DocKind
  payload: ParsedDocument | { text: string } | unknown
}
