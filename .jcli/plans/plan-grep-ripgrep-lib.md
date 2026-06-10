# 计划：用 ripgrep 库替换 GrepTool 的手动搜索实现

## 目标

将 `j-agent/src/tools/grep.rs` 中的手写 `BufReader` + `regex` 搜索逻辑，替换为 ripgrep 的官方库（`grep-searcher` + `grep-regex` + `grep-matcher`），获得：

- ripgrep 的内存映射（mmap）优化策略
- 更高效的缓冲区管理和上下文行处理
- 消除手动 `collect()` 所有行到 `Vec<String>` 的内存浪费
- 保持现有功能和输出格式完全不变

## 变更范围

### 1. `j-agent/Cargo.toml` — 添加依赖

```toml
grep-matcher = "0.1"
grep-regex = "0.1"
grep-searcher = "0.1"
```

**注意**：`regex` crate 在 `j-agent/src/util/text.rs`、`j-agent/src/permission/rules.rs`、`src/util/text.rs` 等处也有使用，**必须保留**。只需移除 grep.rs 中对 `regex` 的 `use` 导入。

### 2. `j-agent/src/tools/grep.rs` — 核心改造

#### 需要修改的部分

1. **正则构建**：`RegexBuilder::new() → RegexMatcherBuilder::new().build()`
   - `grep_regex::RegexMatcherBuilder` 支持 `case_insensitive()`，和当前行为一致
   - 错误处理保持相同格式

2. **文件搜索**：`search_single_file()` → 用 `SearcherBuilder` + 自定义 `Sink` 实现
   - `SearcherBuilder` 配置 `before_context(context)` / `after_context(context)` 和 `line_number(true)`
   - 自定义 `Sink` 实现收集匹配结果到 `SearchResults`
   - 利用 `Sink::matched()` 处理匹配行，`Sink::context()` 处理上下文行
   - 利用 `Sink::matched()` 返回 `false` 来实现 `head_limit` 提前终止
   - `Searcher::search_path()` 让 ripgrep 自行决定是否使用 mmap

3. **取消支持**：`cancelled: Arc<AtomicBool>` 的检查
   - 在 `Sink::matched()` 中检查 cancelled 标志，返回 `false` 终止搜索
   - 文件遍历层面的 cancelled 检查保留在 walker 循环中

#### 不变的部分

- `GrepParams` 结构体和 JSON Schema — 完全不变
- `build_file_walker()` — 仍用 `ignore::WalkBuilder`，ripgrep 库不负责文件遍历
- `get_extensions_for_type()` / `matches_file_type()` — 不变
- 输出格式化函数 (`format_grep_output` 等) — 不变
- `Tool` trait 实现（`name`, `description`, `parameters_schema`）— 不变

#### 新增的类型

```rust
/// 自定义 Sink，收集搜索结果
struct GrepSink<'a> {
    path_str: String,
    output_mode: &'a str,
    head_limit: Option<usize>,
    cancelled: &'a Arc<AtomicBool>,
    results: &'a mut SearchResults,
}
```

实现 `grep_searcher::Sink` trait，在 `matched()` 和 `context()` 中收集结果。

### 3. 移除的代码

- `search_single_file()` 函数 — 被 `Searcher::search_path()` + `GrepSink` 替代
- `build_content_line()` 函数 — 上下文行由 `Sink::context()` 直接处理
- `use regex::{Regex, RegexBuilder}` 导入 — 改为 `grep_regex::RegexMatcherBuilder`
- `use std::fs::File` / `use std::io::{BufRead, BufReader}` — 不再需要手动读文件

## 实施步骤

1. 检查 `regex` crate 在项目其他地方的使用情况，确认是否可以移除
2. 在 `Cargo.toml` 添加 `grep-matcher`、`grep-regex`、`grep-searcher` 依赖
3. 定义 `GrepSink` 结构体并实现 `Sink` trait
4. 重写 `execute()` 方法：用 `RegexMatcherBuilder` 构建匹配器，用 `Searcher::search_path()` 搜索每个文件
5. 删除不再需要的旧函数 (`search_single_file`, `build_content_line`)
6. 运行 `make fmt` + `make lint` + `make test` 验证

## 风险与注意事项

- `Sink` trait 的 `context()` 方法中需要区分 before/after context，使用 `SinkContextKind` 枚举
- 输出格式必须与现有完全一致（`path:line_num:content` 匹配行，`path-line_num:content` 上下文行）
- `grep-searcher` 的 `search_path` 可能对二进制文件自动跳过（需确认行为一致）
- `RegexMatcherBuilder` 的 regex 语法与 `regex` crate 一致（因为底层就是 `regex` crate），所以正则兼容性没有问题
