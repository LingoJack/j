# 计划：编辑器初始光标策略（CursorPolicy）

## 问题
report 进入 TUI 编辑器后，光标始终在文件开头 `(0, 0)`，但用户实际需要在文件末尾编辑。  
其他场景（notebook、chat system prompt 等）则默认在开头更合理。

## 方案：在 `MarkdownEditorOpts` 中增加 `cursor_policy` 字段

### 核心思路
新增 `CursorPolicy` 枚举，放入 `MarkdownEditorOpts`，让调用方按需选择初始光标位置。  
默认 `StartOfFile` 保持向后兼容。

### 改动清单

#### 1. `src/tui/editor_core/editor.rs`
- 新增枚举：
  ```rust
  /// 编辑器初始光标策略
  #[derive(Debug, Clone, PartialEq, Eq, Default)]
  pub enum CursorPolicy {
      /// 光标在文件开头（默认，向后兼容）
      #[default]
      StartOfFile,
      /// 光标在文件末尾
      EndOfFile,
  }
  ```
- `MarkdownEditorOpts` 增加 `pub cursor_policy: CursorPolicy` 字段
- `MarkdownEditor::new()` 中，构造完 buffer 后根据 `cursor_policy` 调用 `buffer.move_cursor_bottom()`

#### 2. `src/tui/editor_core.rs`
- re-export `CursorPolicy`

#### 3. `src/tui/editor_markdown.rs`
- `build_editor_opts()` 增加 `cursor_policy` 参数（或签名改为接收 `CursorPolicy`）
- 所有公共 API 函数不变，内部 `build_editor_opts` 用默认值 `StartOfFile`（保持向后兼容）
- 可选：为 report 场景提供便捷包装，或直接让调用方自行构建 opts

#### 4. `src/command/report/write.rs`
- `handle_report_tui()` 和 `handle_open_report()`：构建 opts 时指定 `CursorPolicy::EndOfFile`

### 不改动的调用方（自动兼容）
- `tui_loop.rs` 中的 3 处调用 —— 使用 `open_markdown_editor_on_terminal(title, content, theme)`，走默认 `StartOfFile`
- `notebook/app/io.rs` 中的 2 处调用 —— 同上
- `notebook/handler.rs` 中的 1 处调用 —— 同上
- `open_script_editor` —— 同上

### 影响范围
- 最小化：仅 editor_core 内部 + report 调用方
- 向后兼容：所有现有调用方无需修改
- 可复用：未来任何需要"打开即跳到末尾"的场景都能使用
