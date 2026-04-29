# Notebook 嵌入 Markdown 编辑器方案

## 1. 目标

将 notebook 右侧预览区替换为**嵌入式 Markdown 编辑器**，实现：
- 选中笔记后直接编辑（无需双击进入外部编辑器）
- 左侧列表 + 右侧编辑器同屏显示
- 焦点可在列表区和编辑区之间切换
- 移除 Preview 模式（全屏预览）、移除外部编辑模式

## 2. 架构变更

### 2.1 核心数据结构（types.rs）

```rust
use crate::tui::editor_core::{MarkdownEditor, EditorTheme, HighlightFn, ...};

pub enum Focus {
    /// 焦点在左侧列表
    List,
    /// 焦点在右侧编辑器
    Editor,
}

pub struct NotebookApp {
    // ... 现有字段 ...
    
    /// 当前焦点
    pub focus: Focus,
    /// 嵌入式 Markdown 编辑器
    pub editor: Option<MarkdownEditor>,
    /// 当前编辑的笔记路径
    pub editing_path: Option<String>,
    /// 编辑器是否需要保存
    pub editor_dirty: bool,
}
```

### 2.2 模式简化

移除以下模式：
- `Preview`（全屏预览）— 编辑器自带渲染
- 不再需要 `e` 键进入外部编辑（编辑器已嵌入）

保留模式：
- `Normal`（浏览+编辑）
- `Adding`（新建笔记）
- `Renaming`（重命名）
- `Search`（搜索）
- `ConfirmDelete`（确认删除）
- `Help`（帮助页）
- `CommandPopup`（命令面板）
- `RatioInput`（比例调整）
- `Mkdir`（新建目录）
- `Mv`（移动笔记）

### 2.3 事件分发（handler.rs）

```rust
match app.focus {
    Focus::List => {
        // 列表区按键处理（现有逻辑）
        // 特殊键切换焦点：
        // - Tab / Right: 切换到编辑器
        // - Enter: 双击行为 → 切换到编辑器并开始编辑
    }
    Focus::Editor => {
        // 编辑器按键处理
        // 特殊键切换焦点：
        // - Esc (Normal mode): 切换回列表
        // - Tab: 切换回列表
        // - Ctrl+S: 保存并切换回列表
    }
}
```

### 2.4 渲染（ui.rs）

```rust
fn render_main_area(app: &NotebookApp, f: &mut Frame, main_area: Rect) {
    let chunks = split_horizontal(main_area, app.panel_ratio);
    
    // 左侧：笔记列表（现有逻辑）
    render_list_panel(app, f, chunks[0]);
    
    // 右侧：编辑器
    if let Some(editor) = &app.editor {
        editor.render(f, chunks[1]);
    } else {
        // 无内容时显示提示
        render_empty_hint(f, chunks[1]);
    }
}
```

## 3. 详细实现步骤

### Step 0: 修复 editor_core 坐标问题（前置依赖）

`MarkdownEditor::render` 中状态栏和命令栏的坐标使用了 `Rect::new(0, ...)`，在非全屏嵌入时会定位错误。

需修改：
- `status_area`: `Rect::new(0, area.height - 1, ...)` → `Rect::new(area.x, area.y + area.height - 1, ...)`
- `cmd_area`: `Rect::new(0, area.height - 2, ...)` → `Rect::new(area.x, area.y + area.height - 2, ...)`

### Step 1: types.rs 数据结构

1. 添加 `Focus` 枚举
2. 添加 `editor: Option<MarkdownEditor>` 字段
3. 添加 `editing_path: Option<String>` 字段
4. 移除 Preview 模式相关字段（`preview_scroll`, `preview_content`, `preview_lines`, `preview_width`）
5. 初始化编辑器需要的 `EditorTheme` 和 `highlight_fn`

### Step 2: handler.rs 事件分发

1. 修改 `run_notebook_tui_internal` 的事件循环
2. 添加 `Focus::List` 和 `Focus::Editor` 的分支
3. 实现焦点切换逻辑
4. 实现编辑器保存逻辑

### Step 3: ui.rs 渲染

1. 修改 `render_main_area` 以渲染编辑器
2. 移除 Preview 模式的渲染逻辑
3. 添加焦点高亮（当前焦点区域的边框颜色）

### Step 4: io.rs 文件操作

1. 修改 `update_preview` → `load_note_for_editor`
2. 添加 `save_note_from_editor` 保存编辑器内容

## 4. 焦点切换快捷键

| 按键 | 当前焦点 | 动作 |
|------|----------|------|
| Tab | List | 切换到编辑器 |
| Right | List | 切换到编辑器 |
| Enter | List | 切换到编辑器（并进入 Insert 模式） |
| Esc | Editor (Normal) | 切换回列表 |
| Tab | Editor | 切换回列表 |
| Ctrl+S | Editor | 保存并切换回列表 |
| Ctrl+Q | Editor | 取消编辑（恢复原内容）并切换回列表 |

## 5. 鼠标支持扩展

- 点击列表区 → 切换焦点到列表
- 点击编辑区 → 切换焦点到编辑器
- 拖拽分割线 → 保持现有逻辑

## 6. 文件修改清单

| 文件 | 修改内容 |
|------|----------|
| `types.rs` | 添加 Focus、editor 字段；移除 Preview 相关字段；简化 AppMode |
| `handler.rs` | 事件分发重构；焦点切换；编辑器集成 |
| `ui.rs` | 编辑器渲染；移除 Preview 模式渲染 |
| `input.rs` | 简化按键处理（移除 Preview 模式相关） |
| `io.rs` | 添加编辑器文件加载/保存 |

## 7. 需导入的 editor_core 类型

```rust
use crate::tui::editor_core::{
    MarkdownEditor,
    EditorTheme,
    HighlightFn,
    CursorPolicy,
    ThemeGalleryItem,
};
use crate::markdown::highlight::highlight_code_line;
use crate::theme::ThemeName;
```

## 8. 测试要点

1. 选中笔记后右侧显示编辑器
2. Tab 切换焦点
3. 在编辑器中编辑并 Ctrl+S 保存
4. 切换笔记时编辑器内容更新
5. 列表区的 j/k 导航正常
6. 鼠标点击切换焦点
7. 拖拽分割线调整比例