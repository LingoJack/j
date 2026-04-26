# 拆分 `src/assets.rs` 为模块目录

## 现状

`src/assets.rs` 是一个 336 行的单文件，包含三大块职责：
1. **核心定义** — `Assets` (RustEmbed)、`HelpTab`/`HelpFrontmatter` 类型、`load_help_tabs()`
2. **模板加载** — 8 个便捷函数，从嵌入资源读取模板内容（`version_template`, `default_system_prompt` 等）
3. **安装逻辑** — 3 个安装函数（`install_default_skills`, `install_default_commands`, `install_default_scripts`），负责将预设资源写入用户数据目录

## 目标结构

采用 `assets.rs` + `assets/` 子目录的 Rust 惯用模块组织：

```
src/assets.rs              ← 模块入口，保留 Assets struct、pub use 重导出
src/assets/
├── help.rs                ← HelpTab, HelpFrontmatter, load_help_tabs(), split_help_frontmatter()
├── template.rs            ← 8 个模板便捷访问函数 + quotes_text()
└── install.rs             ← install_default_skills(), install_default_commands(), install_default_scripts()
```

## 具体步骤

### Step 1: 创建 `src/assets/help.rs`
- 迁移 `HelpTab` struct
- 迁移 `HelpFrontmatter` struct
- 迁移 `load_help_tabs()` 函数
- 迁移 `split_help_frontmatter()` 私有函数

### Step 2: 创建 `src/assets/template.rs`
- 迁移 `version_template()`
- 迁移 `default_system_prompt()`
- 迁移 `default_memory()`
- 迁移 `default_soul()`
- 迁移 `default_agent_md()`
- 迁移 `teammate_system_prompt_template()`
- 迁移 `sub_agent_system_prompt_template()`
- 迁移 `quotes_text()`

### Step 3: 创建 `src/assets/install.rs`
- 迁移 `install_default_skills()`
- 迁移 `install_default_commands()`
- 迁移 `install_default_scripts()`
- 迁移 `PresetEntry` struct
- `install.rs` 内部需要 `use super::Assets;`

### Step 4: 改写 `src/assets.rs` 为模块入口
- 保留 `Assets` struct 定义（因为它是 RustEmbed 核心类型，被多处直接引用）
- 保留模块级 doc comment
- 声明子模块：`mod help; mod template; mod install;`
- 通过 `pub use` 重导出所有公开类型和函数，保持外部调用路径不变

### Step 5: 清理依赖
- 子模块中需要用到 `Assets` 的，通过 `use super::Assets;` 引入
- 外部调用路径（`crate::assets::xxx`）全部保持不变，无需修改任何调用方

## 影响范围

- **无需修改任何调用方代码**，因为 `assets.rs` 入口通过 `pub use` 重导出所有公开 API
- `src/lib.rs` 和 `src/main.rs` 中的 `mod assets` 声明不变
- 所有 `use crate::assets::xxx` 和 `crate::assets::xxx()` 调用路径不变
