# Rust Markdown Parser Analysis for Syntax Triggers
## jcli Project `/Users/jacklingo/dev_custom/jcli`

### Executive Summary

The jcli markdown parser is built on `pulldown_cmark` and produces a Serialize-friendly **Intermediate Representation (IR)** that can be exported to JSON for the web frontend. The architecture supports **full-document re-parsing on every keystroke** (debounced at 150ms) with **no incremental parsing capability** currently implemented.

For **Typora-style syntax triggers**, you'll need to:
1. Detect trigger patterns client-side (e.g., `**text**` → bold)
2. Use the IR output to validate/apply formatting
3. Leverage `source.start_line`/`source.end_line` in Blocks for precise DOM mapping

---

## Part 1: Inline Formatting Handler

**File:** `src/markdown/parser.rs` (lines 380-407)

### How Inline Formatting is Handled

The parser uses **nested stacks** to handle inline formatting:

```rust
// Inline container types (line 16-23)
enum InlineContainer {
    Strong,           // **bold**
    Emphasis,         // *italic*
    Strikethrough,    // ~~strikethrough~~
    Link { url: String },
}
```

#### Event Flow for `**bold text**`:

1. **pulldown_cmark** emits `Event::Start(Tag::Strong)`
   ```rust
   Event::Start(Tag::Strong) => {
       ctx.inline_stack.push(InlineContainer::Strong);
       ctx.inline_children_stack.push(Vec::new());
   }
   ```
   - Opens a new nesting level
   - Starts accumulating child inlines

2. **Text events** are collected into `inline_children_stack.last_mut()`
   ```rust
   Event::Text(text) => {
       ctx.push_inline(Inline::Text(text.to_string()));
   }
   ```

3. **pulldown_cmark** emits `Event::End(TagEnd::Strong)`
   ```rust
   Event::End(TagEnd::Strong) => {
       ctx.inline_stack.pop();
       let children = ctx.inline_children_stack.pop().unwrap_or_default();
       ctx.push_inline(Inline::Strong(children));
   }
   ```
   - Wraps accumulated inlines in `Inline::Strong`
   - Pushes back to parent inline target

### Inline Types (src/markdown/ir.rs, lines 105-129)

```rust
pub enum Inline {
    Text(String),                          // Plain text
    Strong(Vec<Inline>),                   // **bold**
    Emphasis(Vec<Inline>),                 // *italic* or _italic_
    Strikethrough(Vec<Inline>),            // ~~strikethrough~~
    Code(String),                          // `code`
    Link { text: Vec<Inline>, url: String },
    Image { url: String, alt: String },
    SoftBreak,                             // Space or \n within paragraph
    HardBreak,                             // <br> or line-ending \
}
```

**JSON Serialization** (adjacently-tagged):
```json
{ "type": "strong", "value": [{ "type": "text", "value": "bold text" }] }
```

---

## Part 2: Parsed IR Structure & Source Mapping

**File:** `src/markdown/ir.rs`

### Block Structure

```rust
pub struct Block {
    pub source: SourceRange,     // ← KEY for syntax triggers!
    pub kind: BlockKind,
}

pub struct SourceRange {
    pub start_line: usize,       // 0-based line number
    pub end_line: usize,         // 0-based, inclusive
}
```

**This is the key for frontend mapping!** Each block knows exactly which lines it came from:

```rust
pub enum BlockKind {
    Paragraph(Vec<Inline>),
    Heading { level: u8, content: Vec<Inline> },
    CodeBlock { lang: String, code: String, fenced: bool },
    Table(TableData),
    List(ListData),
    BlockQuote(Vec<Block>),
    Rule,
}
```

### How Source Ranges are Computed

**File:** `src/markdown/parser.rs` (lines 273-291)

The parser builds a **byte-offset-to-line mapping**:

```rust
fn build_line_offsets(text: &str) -> Vec<usize> {
    let mut offsets = Vec::with_capacity(64);
    offsets.push(0);
    for (i, ch) in text.char_indices() {
        if ch == '\n' {
            offsets.push(i + 1);
        }
    }
    offsets
}

fn byte_to_line(byte: usize, line_offsets: &[usize]) -> usize {
    match line_offsets.binary_search(&(byte + 1)) {
        Ok(idx) => idx,
        Err(idx) => idx.saturating_sub(1),
    }
}
```

**In the event loop** (lines 349-359):
```rust
for (event, range) in parser.into_offset_iter() {
    let source = if !range.is_empty() {
        SourceRange {
            start_line: byte_to_line(range.start, &line_offsets),
            end_line: byte_to_line(range.end.saturating_sub(1), &line_offsets),
        }
    } else {
        SourceRange::default()
    };
    ctx.current_source = source;
    // ... handle event
}
```

**Result:** Every block knows its exact line range in the source document.

---

## Part 3: Frontend Integration & JSON API

**File:** `src/command/read/server.rs` (line 252-255)

### API Endpoint: `/api/parse`

```rust
async fn api_parse(Json(req): Json<ParseReq>) -> Json<serde_json::Value> {
    let doc = crate::markdown::parser::parse_markdown(&req.source, 120);
    Json(serde_json::to_value(&doc).unwrap_or(serde_json::Value::Null))
}
```

**Full Document Parsing** — **No incremental support**. Every keystroke sends the entire source to the backend, which:
1. Re-parses the whole document
2. Returns JSON IR with all blocks and their source ranges

### Frontend Type Definitions

**File:** `web/src/reader/types.ts`

```typescript
export interface Block {
  source: { start_line: number; end_line: number }
  kind: BlockKind
}

export type Inline =
  | { type: 'text'; value: string }
  | { type: 'strong'; value: Inline[] }
  | { type: 'emphasis'; value: Inline[] }
  | { type: 'strikethrough'; value: Inline[] }
  | { type: 'code'; value: string }
  | { type: 'link'; value: { text: Inline[]; url: string } }
  // ...
```

---

## Part 4: Current Rendering Flow (Typora-style WYSIWYG)

**File:** `web/src/reader/MarkdownLiveEditor.tsx`

### How It Works Today

```
User types in textarea → onChange fires
    ↓
Debounced (150ms) POST to /api/parse with full source
    ↓
Backend parse_markdown() returns JSON IR with source ranges
    ↓
Frontend re-renders:
  - Current block → <textarea> (source editing mode)
  - Other blocks → <MarkdownIR /> (rendered view)
    ↓
Block selection via `block.source.start_line` / `block.source.end_line`
```

**Key code** (lines 106-122):
```tsx
blocks.map((block, i) => {
  const start = block.source.start_line
  const end = block.source.end_line
  if (i === editingBlockIdx) {
    const slice = lines.slice(start, end + 1).join('\n')
    return <BlockSourceEditor sourceSlice={slice} ... />
  }
  return <div onClick={() => setEditingBlockIdx(i)}>
    {renderSingleBlock(block, `b-${i}`)}
  </div>
})
```

---

## Part 5: Parsing Features & Limitations

### ✅ Supported Inline Formatting

From `src/markdown/parser.rs` (line 342-344):
```rust
let options = pulldown_cmark::Options::ENABLE_STRIKETHROUGH
    | pulldown_cmark::Options::ENABLE_TABLES
    | pulldown_cmark::Options::ENABLE_TASKLISTS;
```

- **`**bold**`** → `Inline::Strong`
- **`*italic*`** or **`_italic_`** → `Inline::Emphasis`
- **`~~strikethrough~~`** → `Inline::Strikethrough` (GFM extension)
- **`` `code` ``** → `Inline::Code`
- **`[link](url)`** → `Inline::Link`
- **`![alt](image)`** → `Inline::Image`

### ✅ Supported Block Types

- Paragraphs (lines 507-513)
- Headings (lines 361-377)
- Code blocks (lines 426-449)
- Lists (including nested, task lists)
- Block quotes (lines 515-528)
- Tables (lines 568-611)
- Horizontal rules (lines 559-566)

### ❌ Current Limitations

1. **No incremental parsing** — Always re-parses entire document
2. **No syntax trigger detection** — All parsing done by pulldown_cmark, no character-level monitoring
3. **Source ranges are block-level only** — Inlines don't have individual source ranges
   - A `Strong([...])` inline doesn't know its start/end bytes in the source
   - Makes precise character-level trigger detection difficult

### ⚠️ Text Preprocessing

Lines 301-334: Several preprocessing steps **modify the source before parsing**:

```rust
// 1. ANSI/terminal sanitization
if needs_terminal_sanitization(md) {
    normalized_md = sanitize_terminal_text(md);
    // ...
}

// 2. Chinese quote + bold delimiter fix
if md.contains("**\u{201C}") { 
    // Insert zero-width space to prevent misparse
}

// 3. Table separator normalization
if needs_table_separator_fix(md) {
    separator_fixed = normalize_table_separators(md);
    // ...
}
```

**Impact:** The `source.start_line`/`end_line` refer to the **preprocessed** text, not the original.

---

## Part 6: Recommendations for Syntax Triggers

### Option A: Client-Side Trigger Detection (Simple)

**Frontend detects patterns, sends trigger event + position:**

```typescript
// Listen to onChange in textarea
const handleChange = (e) => {
  const text = e.target.value
  const pos = e.target.selectionStart
  
  // Detect trigger: "**" at pos
  if (text[pos-2:pos] === '**') {
    await fetch('./api/suggest-format', {
      method: 'POST',
      body: JSON.stringify({ source: text, pos, trigger: 'bold' })
    })
  }
}
```

### Option B: Server-Side Partial Reparse (Complex, Not Implemented)

Modify the backend to:
1. Accept a "changed_lines" range
2. Only re-parse blocks that overlap that range
3. Cache unchanged blocks

This would require:
- Tracking block boundaries across parses
- Handling edge cases (list continuation, quote nesting)
- Significant refactoring to `ParseContext`

### Option C: Leverage IR for Trigger Validation (Recommended)

**Use the full-parse + IR output** as a **validation layer**:

```typescript
// After debounced parse returns IR:
1. Look at block.source.start_line/end_line
2. Cross-reference with current cursor position
3. If cursor is in a Strong block, show formatting indicator
4. On trigger (**, //, etc.), update textarea and re-parse to validate
```

---

## Key Data Structures Summary

### Inline Nesting Stack (lines 49-56)
```rust
current_inlines: Vec<Inline>,
inline_stack: Vec<InlineContainer>,
inline_children_stack: Vec<Vec<Inline>>,
```
- Allows arbitrary nesting: `***bold italic***` → `Strong([Emphasis([Text(...)])])`

### List Stack (lines 63-67)
```rust
list_stack: Vec<ListFrame>,
item_stack: Vec<ItemFrame>,
```
- Handles nested lists: `- item 1\n  - nested`

### Table Handling (lines 79-87)
```rust
in_table: bool,
table_rows: Vec<Vec<Vec<Inline>>>,
current_row: Vec<Vec<Inline>>,
current_cell_inlines: Vec<Inline>,
```
- Separate inline stack for cell content

---

## API Reference for Frontend

### `/api/parse` Request
```json
{ "source": "# Heading\n**bold** text" }
```

### `/api/parse` Response
```json
{
  "blocks": [
    {
      "source": { "start_line": 0, "end_line": 0 },
      "kind": {
        "type": "heading",
        "value": {
          "level": 1,
          "content": [{ "type": "text", "value": "Heading" }]
        }
      }
    },
    {
      "source": { "start_line": 1, "end_line": 1 },
      "kind": {
        "type": "paragraph",
        "value": [
          { "type": "strong", "value": [{ "type": "text", "value": "bold" }] },
          { "type": "text", "value": " text" }
        ]
      }
    }
  ]
}
```

---

## Files to Modify for Syntax Triggers

| File | Purpose |
|------|---------|
| `src/markdown/parser.rs` | Add inline source tracking (if implementing Option B) |
| `src/markdown/ir.rs` | Add `SourceRange` to `Inline` struct (if needed) |
| `web/src/reader/MarkdownLiveEditor.tsx` | Add trigger detection logic |
| `web/src/reader/BlockSourceEditor.tsx` | Handle trigger → formatting conversion |
| `src/command/read/server.rs` | Optional: add new `/api/suggest-format` endpoint |

---

## Current Parse Flow Diagram

```
Markdown Source (.md file)
    ↓
[Preprocessing]
  - ANSI sanitization
  - Chinese quote fix
  - Table separator normalization
    ↓
[Line Offset Build]
  - byte_offset → line_number mapping
    ↓
[pulldown_cmark Parser with offset_iter]
  - Emits events with byte ranges
    ↓
[ParseContext Event Loop]
  - Converts byte ranges → line ranges
  - Builds nested Block/Inline tree
    ↓
[JSON Serialization]
  - Serialize ParsedDocument
    ↓
JSON response to /api/parse
```

---

## Next Steps for Your Implementation

1. **Add character-level source ranges to Inline**
   - Modify `ir.rs`: Add `SourceRange` field to `Inline` enum
   - Update parser to track byte ranges for each inline
   - Serialize in JSON

2. **Implement client-side trigger detection**
   - Listen for specific key sequences (`**`, `~~`, etc.)
   - Show formatting preview at cursor

3. **Create `/api/suggest-format` endpoint**
   - Accept cursor position + source
   - Return formatting recommendation
   - Include precise inline range from IR

4. **Update MarkdownLiveEditor**
   - Show inline formatting UI when trigger detected
   - Apply formatting on confirmation

---

## File References

- Parser entry: `src/markdown/parser.rs:299`
- IR types: `src/markdown/ir.rs`
- API endpoint: `src/command/read/server.rs:252`
- Frontend types: `web/src/reader/types.ts`
- Live editor: `web/src/reader/MarkdownLiveEditor.tsx:30`
- IR renderer: `web/src/reader/MarkdownIR.tsx`

