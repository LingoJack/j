# `j md` 支持 stdin/stdout 渲染模式

## 需求

`j md` 命令支持从标准输入读取 Markdown 文本，渲染成 ANSI 彩色输出到标准输出，实现管道用法：

```bash
echo "# Hello" | j md
cat README.md | j md
curl -s https://example.com/readme.md | j md
j md README.md  # 现有行为：用编辑器打开
```

## 架构冲突分析

**结论：不冲突。** 原因如下：

1. **已有渲染基础设施**：`src/util/md_render.rs` 中的 `render_md()` 已经能将 Markdown 文本解析为带颜色的 ANSI 终端输出（`markdown_to_lines` + `print_lines_to_terminal`）。
2. **入口分发自如**：`handle_notebook(args)` 的 match 分支只需增加一个 stdin 检测分支，不影响现有的 TUI / 子命令 / 编辑器路径。
3. **无依赖新增**：`termimad`、`pulldown-cmark`、`crossterm` 已在 Cargo.toml 中，不需要引入新依赖。

## 改动方案

### 检测策略

在 `handle_notebook` 函数顶部，**在所有参数匹配之前**检测 stdin：

- 使用 `std::io::stdin()` 判断是否有管道输入
- 用 `std::io::IsTerminal`（Rust 标准库 1.70+）检测 `stdin().is_terminal()`
- 如果 stdin 不是终端（即有管道输入），读取全部内容，调用 `render_md` 输出后直接 return

### 具体改动

**文件：`src/command/notebook/handler.rs`**

```rust
pub fn handle_notebook(args: &[String]) {
    // 优先检测 stdin 管道输入
    use std::io::IsTerminal;
    if !std::io::stdin().is_terminal() {
        handle_stdin_render();
        return;
    }

    if args.is_empty() {
        run_notebook_tui();
        return;
    }
    // ... 现有逻辑不变
}
```

**新增函数 `handle_stdin_render`（同文件）：**

```rust
/// 从 stdin 读取 Markdown 文本，渲染为 ANSI 彩色输出到 stdout
fn handle_stdin_render() {
    use std::io::Read;
    let mut input = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut input) {
        eprintln!("读取 stdin 失败: {e}");
        std::process::exit(1);
    }
    if input.trim().is_empty() {
        return;
    }
    crate::util::md_render::render_md(&input);
}
```

### 改动文件清单

| 文件 | 变更类型 | 说明 |
|---|---|---|
| `src/command/notebook/handler.rs` | 修改 | 新增 stdin 检测 + `handle_stdin_render` |

### 用户用法

```bash
# 管道渲染
echo "# Hello\nWorld" | j md
cat README.md | j md

# 现有用法不受影响
j md           # TUI 浏览
j md some.md   # 编辑器打开
j md list      # 列出笔记
j md search xx # 搜索
```
