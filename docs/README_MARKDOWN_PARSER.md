# Markdown Parser Documentation

This directory contains comprehensive documentation about the jcli markdown parser and how to implement syntax triggers (Typora-style auto-formatting).

## Documentation Files

### 1. **PARSER_QUICK_REFERENCE.md** ⭐ START HERE
- **Best for:** Quick lookup, common tasks, code navigation
- **Length:** ~400 lines
- **Contains:**
  - Key file locations and purposes
  - Core parsing flow diagram
  - Supported formatting types
  - API endpoint reference
  - Performance metrics
  - Common mistakes and debug tips

**Use this when:** You need a quick answer about how something works

---

### 2. **MARKDOWN_PARSER_ANALYSIS.md** — Deep Technical Analysis
- **Best for:** Understanding architecture in detail
- **Length:** ~600 lines
- **Contains:**
  - Inline formatting handler details (how `**bold**` is parsed)
  - Parsed IR structure and source mapping algorithm
  - Frontend integration and JSON serialization
  - Current rendering flow (Typora-style WYSIWYG)
  - Parsing features and limitations
  - Text preprocessing steps
  - Key data structure summaries
  - API reference with examples

**Use this when:** You're implementing a backend feature or need deep understanding

---

### 3. **SYNTAX_TRIGGERS_IMPLEMENTATION.md** — Implementation Guide
- **Best for:** Planning and implementing syntax triggers
- **Length:** ~500 lines
- **Contains:**
  - Three implementation paths (A, B, C) with tradeoffs
  - Path A: Client-side pattern detection (simplest)
  - Path B: Server-side validation (recommended)
  - Path C: Inline source ranges (most comprehensive)
  - Code examples for each path
  - Testing strategies
  - Potential pitfalls
  - Performance considerations
  - Next immediate steps

**Use this when:** You're ready to implement syntax triggers

---

### 4. **ARCHITECTURE_DIAGRAM.txt** — Visual Flow Diagram
- **Best for:** Visual learners, presentations
- **Length:** ~200 lines
- **Contains:**
  - Complete parsing pipeline diagram (ASCII art)
  - Event processing flow
  - Data structure transformations
  - Syntax trigger integration points
  - Challenge explanation (why inline sources are hard)

**Use this when:** You want to understand the overall flow visually

---

## Key Concepts Summary

### The Main Gap for Syntax Triggers

The parser has a critical limitation:

```
✅ BLOCKS have source ranges:     { start_line: 0, end_line: 3 }
❌ INLINES have NO source ranges: { type: "strong", value: [...] }
```

This means:
- You can map a line number to a block
- You **cannot** easily map a character position to an inline (bold, italic, code, etc.)

**Workaround:** Implement one of the three paths from SYNTAX_TRIGGERS_IMPLEMENTATION.md

### Three Paths to Syntax Triggers

| Path | Difficulty | Backend Work | When To Use |
|------|-----------|--------------|-------------|
| **A: Client-side detection** | Easy | None | Prototype/MVP |
| **B: Server validation** | Medium | 1 endpoint | Production + Path A |
| **C: Inline sources** | Hard | Parser refactor | Long-term enhancement |

**Recommended:** Start with A+B hybrid, plan C for future

---

## Quick Navigation by Use Case

### "I want to understand how the parser works"
1. Read: **PARSER_QUICK_REFERENCE.md** (sections: Core Flow, Inline Nesting, Source Range Mapping)
2. Deep dive: **MARKDOWN_PARSER_ANALYSIS.md** (Part 1-2)
3. Visualize: **ARCHITECTURE_DIAGRAM.txt**

### "I want to implement syntax triggers"
1. Read: **SYNTAX_TRIGGERS_IMPLEMENTATION.md** (sections 1-3: Paths A, B, C)
2. Reference: **PARSER_QUICK_REFERENCE.md** (API Endpoint section)
3. Deep dive: **MARKDOWN_PARSER_ANALYSIS.md** (Part 3: Frontend Integration)

### "I need to modify the parser"
1. Reference: **PARSER_QUICK_REFERENCE.md** (Code Navigation, Key Data Structures)
2. Deep dive: **MARKDOWN_PARSER_ANALYSIS.md** (Part 1-2, Part 5)
3. Implementation: **SYNTAX_TRIGGERS_IMPLEMENTATION.md** (Path C)

### "I need to add a new API endpoint"
1. Reference: **PARSER_QUICK_REFERENCE.md** (API Endpoint section)
2. Deep dive: **MARKDOWN_PARSER_ANALYSIS.md** (Part 3: Frontend Integration)
3. Example: **SYNTAX_TRIGGERS_IMPLEMENTATION.md** (Path B: Server-Side Validation)

### "I'm debugging parsing issues"
1. Reference: **PARSER_QUICK_REFERENCE.md** (Debug Tips)
2. Learn: **MARKDOWN_PARSER_ANALYSIS.md** (Part 5: Limitations & Preprocessing)
3. Visualize: **ARCHITECTURE_DIAGRAM.txt**

---

## Core Files (In The Codebase)

### Backend Rust

| File | Purpose |
|------|---------|
| `src/markdown/parser.rs` | Main parser (764 lines) |
| `src/markdown/ir.rs` | IR types (150 lines) |
| `src/command/read/server.rs` | HTTP API (300+ lines) |

### Frontend TypeScript/React

| File | Purpose |
|------|---------|
| `web/src/reader/types.ts` | Type definitions (100 lines) |
| `web/src/reader/MarkdownLiveEditor.tsx` | Live editor (150 lines) |
| `web/src/reader/MarkdownIR.tsx` | Rendering (380 lines) |
| `web/src/reader/BlockSourceEditor.tsx` | Source editing |

---

## Architecture at a Glance

```
Markdown Source Text
    ↓
[Parser: src/markdown/parser.rs]
  - Preprocess (sanitize, fix quotes, normalize tables)
  - Build line offset map
  - Pull down_cmark event loop
  - Accumulate Blocks/Inlines
    ↓
[IR: src/markdown/ir.rs]
  - ParsedDocument { blocks: Vec<Block> }
  - Each Block knows: source.start_line/end_line
  - Blocks contain: Vec<Inline> (nested)
    ↓
[API: src/command/read/server.rs]
  - POST /api/parse → returns JSON
    ↓
[Frontend: web/src/reader/]
  - Receive JSON IR
  - Render blocks with Typora-style WYSIWYG
  - Show textarea for editing one block at a time
```

---

## Implementation Roadmap

### Phase 1: Client-Side Detection (Path A)
- [ ] Add trigger pattern detection in `BlockSourceEditor.tsx`
- [ ] Show formatting hint/suggestion UI
- [ ] Apply formatting on user confirm

### Phase 2: Server Validation (Path B)
- [ ] Add `/api/validate-format` endpoint
- [ ] Validate against actual parser
- [ ] Return applied source for frontend to use

### Phase 3: Enhanced Features
- [ ] Add inline source ranges (Path C) — complex refactor
- [ ] Support more trigger types (tables, lists, etc.)
- [ ] Add undo/redo

---

## Key Takeaways

### What the Parser Does Well
- ✅ Full Markdown support (GFM extensions)
- ✅ Block-level source mapping (which lines)
- ✅ Nested inline formatting
- ✅ Fast re-parsing (1-5ms)
- ✅ Clean IR for JSON serialization

### What Needs Enhancement for Syntax Triggers
- ❌ Inline source mapping (which characters)
- ❌ Incremental/partial parsing
- ❌ Character-level trigger detection

### How to Work Around These
- Use block-level ranges + manual reconstruction
- Detect triggers client-side (simple pattern matching)
- Validate triggers server-side (call `/api/parse`)

---

## How to Update This Documentation

When you modify the parser or implement syntax triggers:

1. Update the relevant .md file
2. Regenerate diagrams if needed
3. Add examples to SYNTAX_TRIGGERS_IMPLEMENTATION.md
4. Update Performance table in PARSER_QUICK_REFERENCE.md

---

## Questions?

### "How does the parser handle nested formatting?"
→ See PARSER_QUICK_REFERENCE.md: "Inline Nesting"

### "What's the actual file path mapping?"
→ See MARKDOWN_PARSER_ANALYSIS.md: Part 2

### "How do I add a new formatting type (e.g., superscript)?"
→ See SYNTAX_TRIGGERS_IMPLEMENTATION.md: "Path C: Inline Source Ranges"

### "Why can't I find which inline is at the cursor?"
→ See PARSER_QUICK_REFERENCE.md: "For Syntax Triggers" section

---

## Document Statistics

| Document | Lines | Focus | Audience |
|----------|-------|-------|----------|
| PARSER_QUICK_REFERENCE.md | ~400 | Lookup | Everyone |
| MARKDOWN_PARSER_ANALYSIS.md | ~600 | Deep dive | Backend devs |
| SYNTAX_TRIGGERS_IMPLEMENTATION.md | ~500 | Implementation | Implementation devs |
| ARCHITECTURE_DIAGRAM.txt | ~200 | Visual | Visual learners |
| **Total** | **~1700** | **Complete coverage** | **All developers** |

---

**Last Updated:** May 28, 2026  
**Project:** jcli  
**Location:** `/Users/jacklingo/dev_custom/jcli/docs/`
