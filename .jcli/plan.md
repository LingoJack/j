# Plan: Markdown 编辑器代码块支持改进

## 当前问题分析

### 1. 代码块检测问题
- `is_line_in_code_block` 方法将 ``` 开始行也判断为"在代码块内"，但实际上 ``` 行本身应该作为分隔线渲染
- 没有区分代码块开始行和结束行

### 2. 代码块渲染问题
- 代码块内的行使用 `render_source_line` 渲染，但没有应用代码块特殊背景色 (`theme.code_bg`)
- 没有显示代码块语言标识
- 代码块内的代码没有语法高亮（Theme 已有 `code_keyword`, `code_string` 等字段但未使用）

### 3. 代码块边框/装饰
- 没有代码块边框或左侧指示线
- 代码块开始/结束行没有特殊样式

## 改进方案

### 阶段 1: 代码块基础渲染改进
1. **修复代码块检测逻辑**
   - ``` 行不应算作"在代码块内"
   - 添加 `is_code_block_start()` 和 `is_code_block_end()` 方法

2. **代码块特殊样式**
   - 代码块内的行使用 `code_bg` 背景色
   - 代码块左侧添加竖线装饰 (`│`)
   - ``` 开始行显示语言标识

### 阶段 2: 代码块语法高亮（可选）
1. **简单语法高亮**
   - 基于语言类型应用简单高亮规则
   - 支持 Rust、Python、JavaScript、Shell 等常用语言
   - 高亮关键字、字符串、注释、数字等

2. **高亮实现方式**
   - 添加 `highlight_code_line(line, language)` 方法
   - 使用正则或简单解析器识别 token

### 阶段 3: 代码块行号
1. **相对行号**
   - 代码块内显示相对于代码块开始的行号
   - 或使用不同样式的行号

## 实施步骤

### Step 1: 修改代码块检测
- [ ] 修改 `is_line_in_code_block` 方法，``` 行返回 false
- [ ] 添加 `get_code_block_language(line_idx)` 方法获取语言标识
- [ ] 添加 `is_code_fence_line(line)` 检测 ``` 行

### Step 2: 代码块渲染方法
- [ ] 添加 `render_code_fence_line()` 渲染 ``` 行
- [ ] 修改 `render_source_line` 支持 `in_code_block` 参数应用特殊背景色
- [ ] 添加左侧装饰线

### Step 3: 语法高亮（简化版）
- [ ] 添加 `highlight_code()` 方法
- [ ] 支持基础语言关键字高亮
- [ ] 应用 Theme 中的代码颜色

## 文件修改范围
- `src/tui/editor_markdown.rs` - 主要修改文件
- 可能需要添加简单的语法高亮模块

## 预期效果
```
   1 │ # Title                    ← Markdown 渲染
   2 │ 
   3 │ ┌─ rust ───────────────┐   ← 代码块开始
   4 │ │  1 │ fn main() {     │   ← 代码块内容（带背景色）
   5 │ │  2 │     println!()  │
   6 │ │  3 │ }               │
   7 │ └──────────────────────┘   ← 代码块结束
   8 │ 
   9 │ Normal text here           ← Markdown 渲染
```
