# 代码块语言标签优化 + VSCode 风格文件打开

## 一、代码块语言标签优化

### 当前问题

1. **渲染态**：语言标签是一个简单的 `<div class="md-code-lang">`，绝对定位在右上角
2. **编辑态**：替换为 `<input class="md-code-lang-input">`，是一个内联的文本输入框，插在 wrap 的第一个子元素位置
3. 两者视觉风格不统一——渲染态是右上角小标签，编辑态是顶部整行输入框

### 目标效果（对标 Typora）

Typora 的代码块：
- 渲染态：右上角显示语言标签（如 `rust`），左上角可选显示行号
- 编辑态：点击代码块时，语言标签保持原位不动，但变为可编辑状态（或点击标签后编辑）
- 整体视觉：渲染态和编辑态之间，语言标签位置、大小、样式完全一致

### 改造方案

**核心思路**：渲染态和编辑态共用同一个 DOM 位置的语言标签，编辑态直接在原标签上启用编辑。

#### 改动 1：`createCodeBlockElement()` — 渲染态语言标签保持原位不变

当前渲染态的 `label` 已经是绝对定位在右上角，不做改动。

#### 改动 2：`createCodeBlockEditor()` — 复用渲染态语言标签 DOM

不再"移除静态标签 + 创建新 input"，而是：
1. 找到渲染态的 `.md-code-lang` 标签
2. 给它设 `contenteditable="true"` 使其可编辑
3. 添加编辑态样式（focus 时显示边框等）

如果渲染态没有语言标签（`lang` 为空），则创建一个新的，同样绝对定位在右上角。

#### 改动 3：CSS — 语言标签编辑态样式

```css
.md-code-lang[contenteditable="true"] {
  cursor: text;
  outline: none;
  min-width: 30px;
}
.md-code-lang[contenteditable="true"]:focus {
  background: var(--color-seeyue-elevated);
  border-radius: 4px;
  box-shadow: 0 0 0 1px var(--color-seeyue-accent);
}
.md-code-lang[contenteditable="true"]:empty::before {
  content: 'lang';
  color: var(--color-seeyue-fg-dim);
}
```

## 二、VSCode 风格文件打开

### 当前状态

jstudio 通过内嵌的 HTTP 服务提供文件操作，Tauri 壳只在桌面端做窗口管理。文件打开有以下方式：
- **FileTree**：左侧文件树，只能打开已有的 treeRoot 下的文件
- **openRootDialog**：手动输入目录路径的弹窗（不是原生文件选择器）
- **EmptyState**：欢迎页面，提示"从 Explorer 打开文件"

没有：
- 原生文件选择对话框（Cmd/Ctrl+O 打开文件 / Cmd/Ctrl+Shift+O 打开文件夹）
- 菜单栏的 File > Open

### 目标（对标 VSCode）

VSCode 的文件打开体验：
1. **Cmd/Ctrl+O**：打开原生文件选择器，选择文件
2. **Cmd/Ctrl+Shift+O**（或无文件打开时）：打开原生文件夹选择器
3. **EmptyState** 页面有"Open File"和"Open Folder"按钮，点击弹出原生对话框
4. **EditorBar** 或菜单栏有 File 菜单

### 改造方案

由于 jstudio 是 Tauri 2 应用，需要安装 `tauri-plugin-dialog` 来使用原生文件对话框。

#### 改动 1：安装 Tauri dialog 插件

**Rust 侧** (`src-tauri/Cargo.toml`)：
```toml
tauri-plugin-dialog = "2"
```

**Rust 侧** (`src-tauri/src/lib.rs`)：
```rust
.plugin(tauri_plugin_dialog::init())
```

**前端**：
```bash
npm install @tauri-apps/plugin-dialog
```

#### 改动 2：注册 Tauri dialog 权限

需要在 `src-tauri/capabilities/` 下创建权限配置（或修改 tauri.conf.json 的 security.permissions），允许 `dialog:allow-open`。

#### 改动 3：添加前端服务函数

在 `services/index.ts` 中添加：
```typescript
import { open } from '@tauri-apps/plugin-dialog'

export async function openFileDialog(): Promise<string | null> {
  const selected = await open({ multiple: false, directory: false })
  return selected ?? null
}

export async function openFolderDialog(): Promise<string | null> {
  const selected = await open({ multiple: false, directory: true })
  return selected ?? null
}
```

#### 改动 4：Reader.tsx 中添加快捷键和按钮

1. **Cmd/Ctrl+O** → 调用 `openFileDialog()` → 打开文件
2. **Cmd/Ctrl+Shift+O** → 调用 `openFolderDialog()` → 打开文件夹设为 treeRoot
3. **EmptyState** 中添加 "Open File" / "Open Folder" 按钮
4. **FileTree** 的 "打开目录" 按钮改为调用 `openFolderDialog()`

#### 改动 5：兼容非 Tauri 环境（Web 模式）

jstudio 同时支持 Web 模式（通过 HTTP 服务），dialog 插件只在 Tauri 环境可用。需要：
- 检测是否 Tauri 环境
- Web 模式下回退到当前的手动输入路径方式

## 实施步骤

1. 改造代码块编辑器的语言标签为 contenteditable（复用渲染态 DOM）
2. 安装 `tauri-plugin-dialog`（Rust + JS）
3. 注册 dialog 权限
4. 添加 `openFileDialog()` / `openFolderDialog()` 服务函数
5. Reader.tsx 中添加 Cmd/Ctrl+O 快捷键
6. EmptyState 中添加 Open File / Open Folder 按钮
7. FileTree 的"打开目录"改为原生对话框
8. 兼容 Web 模式
