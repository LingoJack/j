# 项目指令

在项目根目录创建 AGENT.md 文件来定义项目级指令，这些指令会在每次对话中自动加载到 LLM 上下文中。

## 用法

- 项目级: 在项目根目录创建 `AGENT.md` 或 `.jcli/AGENT.md`
- 个人级: 创建 `AGENT.local.md` 或 `.jcli/AGENT.local.md`（不提交到 git）
- 用户级: 编辑本文件 `~/.jdata/agent/AGENT.md`

## 示例

```markdown
# 项目约定

- 使用 Rust 2024 edition
- 所有公共 API 必须有文档注释
- 错误处理使用 thiserror 而非 anyhow
- 测试使用 insta snapshot 测试
- 提交信息格式: type(scope): description
```

## 注意

- AGENT.md 中的指令会覆盖默认行为
- 每个文件上限 200 行 / 25KB
- 项目级指令优先级高于用户级
