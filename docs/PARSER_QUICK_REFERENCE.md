# Markdown Parser Quick Reference

## Key Files

| Path | Purpose | Key Types |
|------|---------|-----------|
| `src/markdown/parser.rs` | Full parser implementation | `ParseContext`, `parse_markdown()` |
| `src/markdown/ir.rs` | Intermediate representation | `Block`, `Inline`, `SourceRange` |
| `src/command/read/server.rs` | HTTP API | `/api/parse` endpoint |
| `web/src/reader/types.ts` | Frontend type definitions | `Block`, `Inline`, `ParsedDocument` |
| `web/src/reader/MarkdownLiveEditor.tsx` | Live editor UI | Typora-style WYSIWYG |
| `web/src/reader/MarkdownIR.tsx` | Block/Inline rendering | `renderBlock()`, `renderInline()` |

## Core Flow

```
Markdown Text
    ↓ parse_markdown()
ParsedDocument {
  blocks: [
    Block {
      source: SourceRange { start_line, end_line }  // ← KEY for triggers
      kind: BlockKind::Paragraph(Vec<Inline>)
    }
  ]
}
    ↓ JSON serialize
{ "blocks": [...] }
    ↓ /api/parse response
Frontend receives & renders
```

## Inline Nesting (How Formatting Works)

### The Parse Stack

```rust
Event::Start(Tag::Strong)
  → push Stack
  → push new Vec<Inline> onto children stack

Event::Text("bold")
  → push Text into innermost children

Event::End(TagEnd::Strong)
  → pop children Vec
  → wrap as Inline::Strong(children)
  → push back to parent
```

### Result

```
Source: "**bold**"
     ↓
Inline::Strong([
  Inline::Text("bold")
])
```

### Nesting Example

```
Source: "***bold italic***"
     ↓
Inline::Strong([
  Inline::Emphasis([
    Inline::Text("bold italic")
  ])
])
```

## Source Range Mapping

### How It Works

1. **Build line offsets** — map byte positions to line numbers
2. **pulldown_cmark** emits events with byte ranges
3. **Convert byte ranges → line ranges** using offset map
4. **Store in Block.source** — used for DOM mapping

### Example

```
Source:
  Line 0: "# Heading"           (bytes 0-8)
  Line 1: "**bold** text"        (bytes 10-22)

Event: Strong @ bytes 10-15
  → byte 10 = Line 1
  → byte 15 = Line 1
  → Block.source = { start_line: 1, end_line: 1 }
```

### Critical Limitation

- ✅ **Blocks** have source ranges
- ❌ **Inlines** have NO source ranges
- ❌ Can't easily find which inline is at cursor position X

**Workaround:** Use block-level ranges + manual offset reconstruction

## Supported Formatting

| Markdown | Rust Type | JSON |
|----------|-----------|------|
| `**bold**` | `Inline::Strong([...])` | `{"type":"strong","value":[...]}` |
| `*italic*` | `Inline::Emphasis([...])` | `{"type":"emphasis","value":[...]}` |
| `~~strikethrough~~` | `Inline::Strikethrough([...])` | `{"type":"strikethrough","value":[...]}` |
| `` `code` `` | `Inline::Code(String)` | `{"type":"code","value":"code"}` |
| `[link](url)` | `Inline::Link{text,url}` | `{"type":"link","value":{"text":[...],"url":"..."}}` |
| `![alt](img)` | `Inline::Image{url,alt}` | `{"type":"image","value":{"url":"...","alt":"..."}}` |

## Parsing Options

```rust
// src/markdown/parser.rs:342-344
let options = pulldown_cmark::Options::ENABLE_STRIKETHROUGH
    | pulldown_cmark::Options::ENABLE_TABLES
    | pulldown_cmark::Options::ENABLE_TASKLISTS;
```

- ✅ GFM strikethrough (`~~text~~`)
- ✅ Tables (with colspan, alignment)
- ✅ Task lists (`- [ ] item`)

## Preprocessing (Important!)

**Lines 301-334** modify source before parsing:

1. **ANSI sanitization** — strip terminal codes
2. **Chinese quote fix** — add zero-width spaces around `**`
3. **Table separator normalization** — pad `|` columns

**Impact:** `Block.source` line numbers refer to **preprocessed** text, not original

## Performance

| Operation | Time | Frequency |
|-----------|------|-----------|
| Parse full document | 1-5ms | Every keystroke (debounced 150ms) |
| JSON serialize | <1ms | Same |
| Frontend render | ~50ms | After IR received |

**No incremental parsing** — always re-parses entire document

## API Endpoint

### POST /api/parse

**Request:**
```json
{ "source": "# Title\n**bold**" }
```

**Response:**
```json
{
  "blocks": [
    {
      "source": { "start_line": 0, "end_line": 0 },
      "kind": {
        "type": "heading",
        "value": { "level": 1, "content": [...] }
      }
    },
    {
      "source": { "start_line": 1, "end_line": 1 },
      "kind": {
        "type": "paragraph",
        "value": [...]
      }
    }
  ]
}
```

## For Syntax Triggers

### What You Have

```typescript
block.source = { start_line: 1, end_line: 1 }
// Can find which block contains cursor
```

### What You Need

```typescript
inline.source = { start_line: 1, end_line: 1 }  // ← Doesn't exist yet
// Would let you find which inline contains cursor
```

### Three Paths Forward

| Path | Difficulty | Backend Changes | Frontend Only |
|------|------------|-----------------|---------------|
| **A: Client-side detection** | Easy | None | ✅ Yes |
| **B: Server validation** | Medium | Add 1 endpoint | Minimal |
| **C: Inline sources** | Hard | Refactor parser | No |

**Recommended:** Start with A+B, defer C

## Key Data Structures

### ParseContext (the workhorse)

```rust
struct ParseContext {
    blocks: Vec<Block>,              // ← output
    
    current_inlines: Vec<Inline>,    // ← accumulator
    inline_stack: Vec<InlineContainer>,
    inline_children_stack: Vec<Vec<Inline>>,
    
    list_stack: Vec<ListFrame>,      // ← nested lists
    item_stack: Vec<ItemFrame>,
    
    // ... table/code/blockquote state ...
}
```

### Event Processing

```rust
for (event, range) in parser.into_offset_iter() {
    let source = SourceRange {
        start_line: byte_to_line(range.start, &line_offsets),
        end_line: byte_to_line(range.end - 1, &line_offsets),
    };
    ctx.current_source = source;
    
    match event {
        Event::Start(Tag::Strong) => { /* push nesting */ },
        Event::Text(text) => { /* collect text */ },
        Event::End(TagEnd::Strong) => { /* pop nesting */ },
        // ... handle all events
    }
}
```

## Common Mistakes

### ❌ Don't

1. Use byte positions directly — they're multibyte-unsafe
2. Assume Inlines have source ranges — they don't (yet)
3. Modify source and expect old line ranges to work — they won't
4. Forget preprocessing happens — it changes the text

### ✅ Do

1. Use `byte_to_line()` helper
2. Use `Block.source.start_line`/`end_line` for DOM mapping
3. Call parse_markdown on preprocessed source
4. Track preprocessing for offset mapping

## Testing

### Quick Test

```bash
cd /Users/jacklingo/dev_custom/jcli

# Parse a test file
j read /path/to/test.md

# Or use API directly
curl -X POST http://127.0.0.1:PORT/api/parse \
  -H 'Content-Type: application/json' \
  -d '{"source":"**test**"}'
```

### Unit Tests

```bash
cargo test markdown::parser::tests
```

## Debug Tips

### Print IR Structure

```rust
println!("{:#?}", parse_markdown(source, 120));
```

### Track Event Processing

```rust
for (event, range) in parser.into_offset_iter() {
    eprintln!("Event: {:?}, bytes: {:?}", event, range);
    // ...
}
```

### Verify Source Ranges

```typescript
// Frontend: check block.source
console.log('Block at lines', block.source.start_line, '-', block.source.end_line)
const lines = source.split('\n')
console.log('Content:', lines.slice(block.source.start_line, block.source.end_line + 1))
```

## Code Navigation

```
Parse entry → src/markdown/parser.rs:299 parse_markdown()
            ↓
Preprocessor → src/markdown/parser.rs:301-334
            ↓
Line offsets → src/markdown/parser.rs:273-283 build_line_offsets()
            ↓
Event loop → src/markdown/parser.rs:349-630
            ↓
IR output → src/markdown/ir.rs (Block, Inline, SourceRange)
            ↓
JSON API → src/command/read/server.rs:252 api_parse()
            ↓
Frontend → web/src/reader/MarkdownIR.tsx renderBlock()
```

## Related Docs

- **MARKDOWN_PARSER_ANALYSIS.md** — Deep dive into architecture
- **ARCHITECTURE_DIAGRAM.txt** — Visual flow diagram
- **SYNTAX_TRIGGERS_IMPLEMENTATION.md** — Implementation strategies
