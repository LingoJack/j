## Overview

The built-in Markdown editor is a Typora-like terminal editor with line-level rendering toggle and full Vim mode support.

Core features:
- **Line-level rendering**: Current editing line shows source code, other lines show rendered output
- **Vim mode**: Full support for Normal, Insert, Visual, Command, and Search modes
- **Live preview**: Instant rendering of headings, code blocks, tables, lists, etc.

## Vim Modes

### Mode Switching

| Mode | Border Color | Description |
|------|--------------|-------------|
| Normal | Dark gray | Default browsing mode |
| Insert | Cyan | Text editing mode |
| Visual | Yellow | Visual selection mode |
| Command | Dark gray | Command mode (`:`) |
| Search | Magenta | Search mode (`/`) |

### Normal Mode Shortcuts

| Shortcut | Action |
|----------|--------|
| `h/j/k/l` | Move left/down/up/right |
| `w/b/e` | Word forward/back/end |
| `0/$` | Line start/end |
| `g/G` | File top/bottom |
| `i/a/A/I` | Enter Insert mode |
| `o/O` | Insert new line below/above |
| `x/X` | Delete character |
| `dd` | Delete line |
| `dw/d$` | Delete word/to end of line |
| `cc` | Change entire line |
| `cw/c$` | Change word/to end of line |
| `yy` | Yank (copy) line |
| `p` | Paste |
| `u` | Undo |
| `Ctrl+R` | Redo |
| `v` | Enter Visual mode |
| `:` | Enter Command mode |
| `/` | Enter Search mode |
| `n/N` | Next/previous search match |

### Insert Mode

| Shortcut | Action |
|----------|--------|
| `Esc` | Return to Normal mode |
| Others | Normal text input |

### Visual Mode

| Shortcut | Action |
|----------|--------|
| `h/j/k/l` | Extend selection |
| `y` | Yank (copy) selection |
| `Esc` | Return to Normal mode |

### Command Mode

| Command | Action |
|---------|--------|
| `:w` | Save and submit |
| `:wq` | Save and submit |
| `:x` | Save and submit |
| `:q` | Cancel editing |
| `:q!` | Cancel editing |

### Search Mode

- Press `Enter` after typing search pattern to search
- `n` jumps to next match
- `N` jumps to previous match

## Global Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+S` | Save and submit |
| `Ctrl+Q` | Cancel editing |
| `PageUp/PageDown` | Page scroll |

## Markdown Rendering

### Headings

```
# Heading 1     →  ◆ Heading 1
## Heading 2    →  ◇ Heading 2
### Heading 3   →  〈 Heading 3
#### Heading 4  →  › Heading 4
```

### Code Blocks

Code blocks render with a bordered style:

```
┌─ rust ────────────┐
│ let x = 42;       │
│ println!("{}", x);│
└───────────────────┘
```

Syntax highlighting is supported, language identifier extracted from fence line (e.g., ` ```rust`).

### Tables

Auto-aligned column widths, rendered as a formatted table:

```
│ Header1 │ Header2 │
├────────┼────────┤
│ cell1  │ cell2  │
```

### Other Elements

| Syntax | Rendered |
|--------|----------|
| `**bold**` | **bold** |
| `*italic*` | *italic* |
| `~~strikethrough~~` | ~~strikethrough~~ |
| `` `code` `` | `code` |
| `- list item` | • list item |
| `- [ ] task` | ○ task |
| `- [x] done` | ● done |
| `> quote` | │ quote |
| `[link](url)` | link ↗ |
| `![image](url)` | 🖼 alt |

## Use Cases

- Writing daily/weekly reports
- Editing Markdown documents
- Quick note-taking
- Code snippet editing

The editor returns the edited content on save, and returns empty on cancel.
