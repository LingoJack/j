# 计划：Teammate System Prompt 模板化

## 背景

当前 `teammate_loop.rs` 中 `build_teammate_system_prompt()` 函数将 teammate 的 system prompt 内容硬编码在 Rust 代码中（第 436-475 行），包含：

- Your Identity（名字、角色）
- Teammates（团队列表）
- Communication（SendMessage 通信规则）
- Message Wake Semantics（唤醒语义）
- Completing Your Work（WorkDone 机制）
- Rules（行为规则）

项目已有成熟的模板+占位符体系：
- 模板文件放在 `assets/` 目录
- 通过 `rust-embed` 编译时嵌入（`src/assets.rs`）
- 使用 `{{.placeholder}}` 格式的占位符
- 运行时通过 `.replace()` 替换为实际值（如 `system_prompt_default.md` 的处理方式）

## 改动方案

### 1. 新建模板文件 `assets/teammate_system_prompt.md`

将 `build_teammate_system_prompt()` 中的硬编码文本提取为模板，使用占位符：

| 占位符 | 含义 | 来源 |
|--------|------|------|
| `{{.base_prompt}}` | 主 agent 的 base system prompt | `base_system_prompt` 参数 |
| `{{.name}}` | teammate 名字 | `name` 参数 |
| `{{.role}}` | teammate 角色 | `role` 参数 |
| `{{.team_summary}}` | 团队成员列表摘要 | `teammate_manager.team_summary()` |

模板内容直接从现有 `format!()` 宏的字符串体提取，保持原样。

### 2. 在 `src/assets.rs` 中添加加载函数

```rust
pub fn teammate_system_prompt_template() -> Cow<'static, str> {
    Assets::get("teammate_system_prompt.md")
        .map(|f| String::from_utf8_lossy(&f.data).into_owned().into())
        .unwrap_or_else(|| Cow::Borrowed(""))
}
```

并在模块顶部注释的资源清单中新增条目。

### 3. 修改 `teammate_loop.rs` 中的 `build_teammate_system_prompt()`

从：
```rust
fn build_teammate_system_prompt(...) -> String {
    // 硬编码 format!(...) 
}
```

改为：
```rust
fn build_teammate_system_prompt(...) -> String {
    let template = crate::assets::teammate_system_prompt_template();
    let base = base_prompt.unwrap_or("You are a helpful assistant.");
    let team_summary = teammate_manager.lock().map(|m| m.team_summary()).unwrap_or_default();
    template.as_ref()
        .replace("{{.base_prompt}}", base)
        .replace("{{.name}}", name)
        .replace("{{.role}}", role)
        .replace("{{.team_summary}}", &team_summary)
}
```

## 涉及文件

| 文件 | 操作 |
|------|------|
| `assets/teammate_system_prompt.md` | **新建** — teammate system prompt 模板 |
| `src/assets.rs` | **修改** — 新增 `teammate_system_prompt_template()` 函数 |
| `src/command/chat/teammate/teammate_loop.rs` | **修改** — `build_teammate_system_prompt()` 改为模板替换 |

## 验证

- `cargo build` 编译通过
- `cargo clippy` 无警告
- `cargo fmt` 格式化
