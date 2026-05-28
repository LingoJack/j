# Implementing Typora-Style Syntax Triggers

**Status:** Planning phase  
**Target:** Add real-time syntax trigger UI for markdown formatting  
**Challenge Level:** Medium  
**Timeline:** Depends on approach chosen

## Quick Start: Understanding the Flow

### 1. How Parsing Works Today

```
Keystroke in editor
    ↓
onChange fires (every keystroke)
    ↓
Debounced (150ms) POST to /api/parse
    ↓
Backend calls parse_markdown(source, 120)
    ↓
Returns JSON with Blocks + Inlines + source.start_line/end_line
    ↓
Frontend re-renders blocks based on editingBlockIdx
```

### 2. Why Syntax Triggers are Hard

The current architecture has a **critical gap**:

- ✅ **Blocks** have source ranges: `{ start_line: 0, end_line: 3 }`
- ❌ **Inlines** have NO source ranges: just `{ type: "strong", value: [...] }`

**Impact:** You can't precisely map a cursor position to a `Strong` or `Emphasis` inline without reconstructing the text offsets manually.

### 3. Three Implementation Paths

#### Path A: Client-Side Pattern Detection (Simplest)

**Pros:** No backend changes, fast, client-controlled  
**Cons:** Can't validate against actual parser

```typescript
// In BlockSourceEditor.tsx
const handleKeyDown = (e: KeyboardEvent) => {
  const text = (e.target as HTMLTextAreaElement).value
  const pos = (e.target as HTMLTextAreaElement).selectionStart
  
  // Detect `**` trigger
  if (pos >= 2 && text.slice(pos - 2, pos) === '**') {
    showFormattingMenu('bold', pos - 2)
  }
  
  // Detect `~~` trigger
  if (pos >= 2 && text.slice(pos - 2, pos) === '~~') {
    showFormattingMenu('strikethrough', pos - 2)
  }
}
```

**Next Step:** User confirms → apply formatting → rely on debounced parse for validation

---

#### Path B: Server-Side Validation (Recommended)

**Pros:** Can validate against actual parser, precise error detection  
**Cons:** Adds API endpoint, slightly higher latency

**New API:** `POST /api/validate-format`

```rust
// src/command/read/server.rs

#[derive(Deserialize)]
struct ValidateFormatReq {
    source: String,
    start: usize,      // cursor start (bytes)
    end: usize,        // cursor end (bytes)
    trigger: String,   // "bold", "italic", "strikethrough"
}

#[derive(Serialize)]
struct ValidateFormatResp {
    valid: bool,
    message: String,
    applied_source: Option<String>,  // if valid, show what it becomes
}

async fn api_validate_format(Json(req): Json<ValidateFormatReq>) -> Json<ValidateFormatResp> {
    let (applied, valid) = apply_formatting(&req.source, req.start, req.end, &req.trigger);
    
    // Parse both to compare IR
    let original_doc = parse_markdown(&req.source, 120);
    let applied_doc = parse_markdown(&applied, 120);
    
    Json(ValidateFormatResp {
        valid,
        message: format!("Formatting {} valid", if valid { "is" } else { "would be invalid" }),
        applied_source: if valid { Some(applied) } else { None },
    })
}
```

**Frontend Usage:**

```typescript
const handleTrigger = async (text: string, start: number, end: number) => {
  const res = await fetch('./api/validate-format', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ source: text, start, end, trigger: 'bold' })
  })
  const result = (await res.json()) as ValidateFormatResp
  
  if (result.valid) {
    onChange(result.applied_source!)  // triggers debounced parse
  } else {
    showError(result.message)
  }
}
```

---

#### Path C: Inline Source Ranges (Most Comprehensive)

**Pros:** Precise character-level mapping, matches Typora exactly  
**Cons:** Significant backend refactoring needed

**Changes Required:**

1. **Modify `ir.rs`** to add source tracking to Inline:

```rust
#[derive(Debug, Clone, Serialize)]
pub struct Inline {
    #[serde(flatten)]
    pub kind: InlineKind,
    pub source: Option<SourceRange>,  // ← new field
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum InlineKind {
    Text(String),
    Strong(Vec<Inline>),
    // ... etc
}
```

2. **Modify `parser.rs`** to track inline byte offsets:

```rust
impl ParseContext {
    fn push_inline(&mut self, kind: InlineKind, byte_range: Option<std::ops::Range<usize>>) {
        let source = byte_range.map(|r| {
            SourceRange {
                start_line: byte_to_line(r.start, &line_offsets),
                end_line: byte_to_line(r.end.saturating_sub(1), &line_offsets),
            }
        });
        let inline = Inline { kind, source };
        self.current_inline_target().push(inline);
    }
}
```

3. **Update event loop** to pass byte ranges:

```rust
Event::Start(Tag::Strong) => {
    let byte_range = Some(range.clone());
    ctx.inline_stack.push(InlineContainer::Strong(byte_range));
    // ...
}
```

**Frontend Benefit:**

```typescript
// Can now find exact inline at cursor position
function findInlineAtPos(block: Block, pos: number): Inline | null {
  const walk = (inlines: Inline[]): Inline | null => {
    for (const inline of inlines) {
      if (inline.source && inline.source.start_line <= pos && pos <= inline.source.end_line) {
        if (inline.type === 'strong' || inline.type === 'emphasis') {
          return walk(inline.value)  // recurse into children
        }
        return inline
      }
    }
    return null
  }
  
  if (block.kind.type === 'paragraph') {
    return walk(block.kind.value)
  }
  if (block.kind.type === 'heading') {
    return walk(block.kind.value.content)
  }
  return null
}
```

---

## Implementation Recommendation

**Start with Path A + B Hybrid:**

1. **Client-side detection** (Path A)
   - Cheap, instant feedback
   - Simple regex/string matching for `**`, `*`, `~~`, `` ` ``, `[`, `!`

2. **Add `/api/validate-format`** (Path B)
   - Validate against actual parser
   - Only call on user confirm (not on every keystroke)

3. **Defer Path C** (Inline sources)
   - Do this in a separate PR after Path A+B works
   - More refactoring needed, but gives you full Typora parity

---

## Code Locations to Modify

### Backend Files

| File | Change | Priority |
|------|--------|----------|
| `src/command/read/server.rs` | Add `/api/validate-format` | High (Path B) |
| `src/markdown/parser.rs` | Add inline source tracking | Low (Path C) |
| `src/markdown/ir.rs` | Add `source` field to `Inline` | Low (Path C) |

### Frontend Files

| File | Change | Priority |
|------|--------|----------|
| `web/src/reader/BlockSourceEditor.tsx` | Add trigger detection in `onKeyDown` | High (Path A) |
| `web/src/reader/MarkdownLiveEditor.tsx` | Show trigger menu UI | High (Path A) |
| `web/src/reader/types.ts` | Extend `Inline` type (Path C only) | Low |

---

## Quick Code Examples

### Example 1: Detect Bold Trigger in BlockSourceEditor

```typescript
// web/src/reader/BlockSourceEditor.tsx

const handleKeyDown = (e: KeyboardEvent) => {
  if (e.key !== '*') return
  
  const ta = e.currentTarget as HTMLTextAreaElement
  const pos = ta.selectionStart
  const text = ta.value
  
  // Detect opening ** 
  if (pos >= 2 && text[pos - 2] === '*' && text[pos - 1] === '*') {
    // Could be closing **text**
    // Or trigger
    e.preventDefault()
    
    // Show suggestion: "Press Enter to wrap as bold"
    setTriggerHint({ type: 'bold', startPos: pos - 2 })
    return
  }
  
  // Let normal keydown continue
}

const handleKeyPress = (e: KeyboardEvent) => {
  if (e.key === 'Enter' && triggerHint) {
    e.preventDefault()
    
    // Get selected text or word
    const ta = e.currentTarget as HTMLTextAreaElement
    const text = ta.value
    const start = Math.max(0, ta.selectionStart - triggerHint.startPos)
    
    // Would call /api/validate-format here
    applyFormatting(triggerHint.type, start)
    setTriggerHint(null)
  }
}
```

### Example 2: Inline Source Mapping

```typescript
// Utility to reconstruct inline positions
function computeInlineOffsets(
  blockSource: SourceRange,
  blockContent: string,
  inlines: Inline[]
): Map<Inline, [start: number, end: number]> {
  const offsets = new Map<Inline, [number, number]>()
  let pos = 0
  
  const walk = (inlines: Inline[]) => {
    for (const inline of inlines) {
      const start = pos
      
      if (inline.type === 'text') {
        pos += inline.value.length
      } else if (inline.type === 'strong' || inline.type === 'emphasis') {
        walk(inline.value)
      } else if (inline.type === 'code') {
        pos += 2 + inline.value.length + 2  // `code`
      }
      
      offsets.set(inline, [start, pos])
    }
  }
  
  walk(inlines)
  return offsets
}
```

### Example 3: Server-Side Formatting Application

```rust
// src/markdown/parser.rs

pub fn apply_inline_formatting(
    source: &str,
    start_byte: usize,
    end_byte: usize,
    format_type: &str,
) -> Result<String, String> {
    let (before, middle, after) = (
        &source[..start_byte],
        &source[start_byte..end_byte],
        &source[end_byte..],
    );
    
    let formatted = match format_type {
        "bold" => format!("**{}**", middle),
        "italic" => format!("*{}*", middle),
        "strikethrough" => format!("~~{}~~", middle),
        "code" => format!("`{}`", middle),
        _ => return Err("Unknown format".to_string()),
    };
    
    Ok(format!("{}{}{}", before, formatted, after))
}
```

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_apply_formatting() {
        let source = "hello world";
        let result = apply_inline_formatting(source, 0, 5, "bold").unwrap();
        assert_eq!(result, "**hello** world");
    }
    
    #[test]
    fn test_validate_formatting() {
        let source = "**already bold**";
        // Should detect nested bold as invalid?
        // Or allow nested?
        // Depends on markdown spec
    }
}
```

### Frontend Tests

```typescript
// web/src/reader/BlockSourceEditor.test.tsx

describe('Syntax Triggers', () => {
  it('detects ** as bold trigger', () => {
    const { getByRole } = render(<BlockSourceEditor ... />)
    const textarea = getByRole('textbox') as HTMLTextAreaElement
    
    fireEvent.change(textarea, { target: { value: 'hello **world' } })
    fireEvent.keyDown(textarea, { key: '*' })
    
    expect(screen.getByText(/bold/i)).toBeInTheDocument()
  })
})
```

---

## Potential Pitfalls

### 1. Multi-byte Characters (Chinese, Emoji)

```rust
// ❌ Don't use byte indices directly with char indices
let pos = 5;  // Is this a byte or char position?

// ✅ Use char_indices to be safe
let char_pos = source.chars().take(5).map(|c| c.len_utf8()).sum();
```

### 2. Nested Formatting

```
**bold *italic***  ← nested
↓
Strong([
  Text("bold "),
  Emphasis([Text("italic")])
])
```

Parser handles this correctly, but your trigger detection needs to account for it.

### 3. Line Mapping After Preprocessing

Source ranges map to **preprocessed** text, not original:
- ANSI codes stripped
- Chinese quotes fixed
- Table separators normalized

For syntax triggers, work with the **preprocessed** source.

### 4. Selection Spanning Multiple Lines

```
**line 1
line 2**

↑ multiline selection
```

Need to handle this in your formatting logic.

---

## Performance Considerations

| Operation | Cost | When |
|-----------|------|------|
| Full re-parse | ~1-5ms | On every keystroke (debounced 150ms) |
| `/api/validate-format` | ~1-2ms | On trigger confirm (optional) |
| Inline offset reconstruction | ~0.1ms | Per render (frontend) |

**Recommendation:** Debounce trigger validation to avoid overwhelming the API.

---

## Next Immediate Steps

1. **Pick an implementation path** (recommend A+B hybrid)
2. **Create a feature branch** for syntax triggers
3. **Add BlockSourceEditor trigger detection** (Path A - frontend only)
4. **Prototype the trigger UI** (visual feedback)
5. **Add `/api/validate-format`** endpoint (Path B - backend)
6. **Plan Path C** (inline source ranges) as future enhancement

---

## References

- Parser: `src/markdown/parser.rs:299` (entry point)
- IR Types: `src/markdown/ir.rs`
- API: `src/command/read/server.rs:252`
- Frontend Editor: `web/src/reader/BlockSourceEditor.tsx`
- Analysis Docs:
  - `docs/MARKDOWN_PARSER_ANALYSIS.md` (detailed breakdown)
  - `docs/ARCHITECTURE_DIAGRAM.txt` (visual flow)
