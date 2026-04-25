**1. 工具与格式 (Tools & Format)**
* 代码必须通过 `cargo fmt` 格式化。
* 代码必须通过 `cargo clippy -- -D warnings` 检查且无告警（CI/Makefile 强制）。
* 命名规范：类型/Trait 为 `PascalCase`，函数/变量/模块为 `snake_case`，常量为 `SCREAMING_SNAKE_CASE`。

**2. 内存与性能 (Memory & Performance)**
* 避免非必要的 `.clone()`，优先考虑所有权转移或借用。
* 接口参数优先使用切片（`&str`, `&[T]`）而非包装类型（`String`, `Vec`）。
* 优先使用迭代器（Iterator）处理集合，利用其特性减少手动边界检查。
* 集合类型在已知大小时使用 `with_capacity` 预分配内存，减少重分配开销。

**3. 错误处理 (Error Handling)**
* 非 test 代码应避免 `unwrap()` 和 `expect()`；仅在逻辑确信不会失败时使用（如 `Mutex::lock().unwrap()`）并辅以注释说明安全性。
* 使用 `?` 操作符进行错误传播，避免深层嵌套的 `match` 或 `if let`。
* 项目为纯 Binary，错误类型使用手写 enum + `impl std::error::Error` + `From` 转换（如 `ChatError`）；`thiserror` 为依赖但非强制。

**4. 类型设计与 Trait (Type Design)**
* 类型定义与对应的 `impl` 块应在同一文件中物理相邻。
* 优先为结构体派生常用 Trait：`Debug`, `Default`, `PartialEq`。
* 构造函数惯例命名为 `pub fn new(...) -> Self`；若无参数，应同时实现 `Default` Trait。
* 字段可见性遵循最小化原则；跨模块暴露的内部字段考虑 `pub(crate)`，模块内私有不加 `pub`。

**5. 模式匹配 (Pattern Matching)**
* 显式处理所有枚举分支，避免过度依赖 `_ => ...`。
* 利用组合子（`.map()`, `.and_then()`, `.ok_or()`）简化 `Option` 和 `Result` 的链式处理。
* 简单分支判断使用 `if let` 或 `let else`。

**6. 模块组织与路径引用 (Module & Pathing)**
* **路径简化原则**：禁止在逻辑代码中频繁出现长路径引用（如 `a::b::c::Type`）。
    * 结构体/枚举：通过 `use a::b::c::Type;` 导入，直接使用 `Type`。
    * 导入冲突：若有同名类型，使用 `use ... as ...` 别名，或仅导入至上一级（如 `use std::fmt;` 然后使用 `fmt::Result`）。
* **弃用 mod.rs**：采用 `name.rs` + `name/` 子目录的文件组织方式。
* **语义化分文件**：避免在单一文件中堆叠不相关功能，按职责拆分（如 `time.rs`、`path_utils.rs`）。

**7. 函数与逻辑 (Logic & Functions)**
* 遵循"单一职责原则"：一个函数只做一件事。
* 当函数嵌套过深或逻辑分支过多时，应提取私有辅助函数。
* 函数参数超过 4 个时，考虑封装为 `Config` 结构体或使用 Builder 模式。
* 魔法值必须提取为关联常量（`impl` 块内）或模块级 `const`。

**8. 文档与安全 (Docs & Safety)**
* 公共 API 和核心类型（ChatApp、Action、Tool trait、ChatError 等）必须有 `///` 文档注释。
* 使用 `unsafe` 块时，必须在上方标注 `// SAFETY:` 注释，解释其安全性。

**9. TUI 输出规范 (TUI Output Discipline)**
* **禁止在 TUI 模式下使用 `println!` / `eprintln!` / `crate::info!` / `crate::error!`**：这些宏输出到 stdout/stderr，会溢出到 TUI 界面（输入区、消息区等），造成显示异常。
* 后台线程、工具执行线程、agent 线程中的日志必须使用 `crate::util::log::write_info_log()` / `write_error_log()` 写入日志文件。
* `crate::debug_log!` 同理，仅在 verbose 模式下输出但也会污染 TUI；TUI 模式下的调试信息应走文件日志。