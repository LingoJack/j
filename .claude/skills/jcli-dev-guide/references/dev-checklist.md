# 开发任务 Checklist

## 添加新 CLI 子命令

- [ ] **`src/cli.rs`** — 在 `SubCmd` 枚举添加新变体（含参数、doc comment、alias）
- [ ] **`src/command/handler.rs`** — 用 `command_handlers!` 宏注册对应的 `XxxCmd` struct
- [ ] **`src/command/handler.rs`** — 在 `SubCmd::into_handler()` 的 `match` 中添加分支
- [ ] **`src/command/xxx.rs`** — 实现实际的 `handle_xxx()` 函数
- [ ] **`src/command/mod.rs`** — 如需要 `mod xxx;` 声明
- [ ] **`src/interactive/completer.rs`** — 更新 REPL tab 补全列表（如需要）
- [ ] **`src/command/help/`** — 更新帮助文字（如需要）
- [ ] 运行 `cargo build` + `cargo clippy -- -D warnings` 验证

## 添加新 AI 工具（Chat 模块）

- [ ] **`src/command/chat/tools/my_tool.rs`** — 创建文件，实现 `Tool` trait
  - `name()` 返回工具名（驼峰，如 `"MyTool"`）
  - `description()` 写清楚用途，LLM 依赖此选择工具
  - `parameters_schema()` 用 `schema_to_tool_params::<MyArgs>()`
  - `execute()` 实现业务逻辑，返回 `ToolResult`
  - 需要确认时重写 `requires_confirmation()` 返回 `true`
  - 重写 `confirmation_message()` 提供友好的确认提示
- [ ] **`src/command/chat/tools/mod.rs`** — 添加 `pub mod my_tool;`
- [ ] **`src/command/chat/tools/mod.rs`** — 在 `ToolRegistry::new()` 的 `tools: vec![...]` 中添加 `Box::new(my_tool::MyTool)`（或带参数的实例）
- [ ] 运行 `cargo build` + `cargo clippy -- -D warnings` 验证

## 修改 Hook 系统

- [ ] 新增事件类型：在 `src/command/chat/hook.rs` 的 `HookEvent` 枚举中添加变体
- [ ] 更新 `as_str()` / `from_str()` 实现
- [ ] 在 `agent.rs` 的 agent 循环中调用 `hook_manager.fire(HookEvent::MyEvent, ctx)`
- [ ] 更新 hook 文档注释中的触发时机表格

## 修改 Permission 系统

- [ ] 新增规则类型：在 `src/command/chat/permission.rs` 的 `PermissionConfig` 中添加字段
- [ ] 在 `JcliConfig::load()` 中处理新字段的反序列化
- [ ] 在工具执行前调用 `permission.check()` 相关逻辑
- [ ] 更新 `.jcli/permissions.yaml` 格式文档

## 修改配置系统

- [ ] **`src/config/yaml_config.rs`** — 在 `YamlConfig` struct 中添加字段（带默认值）
- [ ] 确保字段有 `#[serde(default)]` 以向后兼容
- [ ] **`src/command/system.rs`** — 如需 `j change` 支持新字段，更新 `handle_change()`
- [ ] **`src/constants.rs`** — 如有新的路径/常量，在此添加

## 添加新主题（Chat TUI）

- [ ] **`src/command/chat/theme.rs`** — 在主题枚举和 `theme()` 函数中添加新主题
- [ ] 按现有主题格式定义颜色 palette

## 发布前检查

- [ ] `make pre-commit`（format + lint + test 全部通过）
- [ ] `cargo test --all-features` 通过
- [ ] 更新 `CLAUDE.md` 如有新的架构变化
- [ ] `make bump-version` 或 `make set-version V=x.y.z`
- [ ] `make publish`（bump + build + tag + push + publish）

## 常用路径速查

| 用途 | 路径 |
|------|------|
| 主配置 | `~/.jdata/config.yaml` |
| Agent 配置 | `~/.jdata/agent/data/agent_config.json` |
| 聊天历史 | `~/.jdata/agent/sessions/` |
| Agent 日志 | `~/.jdata/agent/logs/` |
| 用户 skills | `~/.jdata/agent/skills/` |
| 用户 hooks | `~/.jdata/agent/hooks/` |
| 项目权限配置 | `.jcli/permissions.yaml`（从 cwd 向上查找）|
| 项目 hooks | `.jcli/hooks/`（从 cwd 向上查找）|
| patched 依赖 | `patches/tui-textarea-0.7.0/` |

## 调试技巧

```bash
# 启用 verbose 日志
j log mode verbose

# 查看 agent 日志
tail -f ~/.jdata/agent/logs/*.log

# 带 browser CDP 功能构建
cargo build --features browser_cdp

# 运行单个测试
cargo test test_name_here

# 本地安装测试
make install
```
