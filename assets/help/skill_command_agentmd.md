---
name: Skill & 命令
order: 9
---

## Skill 技能系统

Skill 支持两级目录：

- 用户级：`~/.jdata/agent/skills/`
- 项目级：`.jcli/skills/`（同名时覆盖用户级）

AI 通过 `load_skill` 工具按需加载技能。

**创建 Skill**：

```bash
mkdir -p ~/.jdata/agent/skills/my-skill
cat > ~/.jdata/agent/skills/my-skill/SKILL.md << 'EOF'
---
name: my-skill
description: 技能描述
argument-hint: "[参数说明]"
---

指令正文，$ARGUMENTS 会被替换为参数...
EOF
```

**使用方式**：

| 操作 | 说明 |
|------|------|
| 输入 `@` | 弹出混合选择列表（含 skill: 分类入口） |
| `@skill:` | 直接弹出技能选择列表（支持过滤） |
| `↑↓` 选择 + `Tab/Enter` | 补全技能名称 |
| AI 自动调用 `load_skill` | 从 skills 摘要识别后自动加载 |

> Skill 目录支持 `references/` 子目录存放参考文件，会自动附加到上下文

## Commands 自定义命令

除了 Skill，还支持自定义命令模板：

- 用户级：`~/.jdata/agent/commands/`
- 项目级：`.jcli/commands/`（同名时覆盖用户级）

**创建方式**：

```bash
mkdir -p ~/.jdata/agent/commands/review
cat > ~/.jdata/agent/commands/review/COMMAND.md << 'EOF'
---
name: review
description: 代码审查模板
---

请以 code review 模式检查当前改动，优先找 bug、回归风险和遗漏测试。
EOF
```

**使用方式**：

- 在对话中输入 `@command:` 唤起补全
- 选择后会把命令正文展开到输入内容中
- 配置界面的 `Commands` Tab 可启用/禁用命令

## AGENT.md 项目指令

AGENT.md 是项目级指令系统，让 AI 在每次对话中自动加载项目约定，确保长上下文中始终遵循项目规范。

**创建方式**：

```bash
# 项目级（提交到 git，团队共享）
cat > AGENT.md << 'EOF'
# 项目约定

- 使用 Rust 2024 edition
- 所有公共 API 必须有文档注释
- 错误处理使用 thiserror 而非 anyhow
- 提交信息格式: type(scope): description
EOF

# 项目级（.jcli 目录下，与权限配置同目录）
cat > .jcli/AGENT.md << 'EOF'
# 额外约定
...
EOF

# 个人级（不提交到 git）
cat > AGENT.local.md << 'EOF'
# 个人偏好
...
EOF
```

**加载优先级**（后者覆盖前者）：

| 优先级 | 文件 | 说明 |
|--------|------|------|
| 最低 | `~/.jdata/agent/AGENT.md` | 用户级，所有项目生效 |
| ↓ | `AGENT.md`（从 git root 到 CWD） | 项目级，提交到 git |
| ↓ | `.jcli/AGENT.md`（从 git root 到 CWD） | 项目级，与权限配置同目录 |
| ↓ | `AGENT.local.md`（从 git root 到 CWD） | 个人级，不提交 git |
| 最高 | `.jcli/AGENT.local.md`（从 git root 到 CWD） | 个人级，不提交 git |

> 每个文件上限 200 行 / 25KB。项目级指令自动带有 OVERRIDE 强制力，优先于默认行为。
> 在对话配置界面（Ctrl+E）中选择 "AGENT.md" 字段可编辑用户级指令。
