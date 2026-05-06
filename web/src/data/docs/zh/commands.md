## 概述

Commands 是可复用的提示词片段，帮助快速调用预设的提示词。

## 斜杠命令

在输入框中输入 `/` 触发斜杠命令：

| 命令 | 描述 |
|------|------|
| `/copy` | 复制最后一条 AI 回复 |
| `/log` | 打开日志窗口 |
| `/browse` | 浏览历史消息 |
| `/config` | 打开配置界面 |
| `/model` | 切换模型 |
| `/archive` | 归档当前对话 |

## 自定义命令

### 使用方式

在输入框中以 `@command:<名称>` 引用：

```
@command:review 请审查这段代码
```

### 创建命令

#### 目录位置

| 级别 | macOS / Linux | Windows |
|------|---------------|---------|
| 用户级 | `~/.jdata/agent/commands/` | `%USERPROFILE%\.jdata\agent\commands\` |
| 项目级 | `.jcli/commands/` | `.jcli\commands\` |

项目级命令优先级更高。

#### 文件格式

每个命令是一个 Markdown 文件，包含 YAML frontmatter 和提示词正文：

```markdown
---
name: review
description: 代码审查提示词
---
请对以下代码进行全面审查，关注：
- 代码质量
- 潜在问题
- 改进建议
```

#### 两种组织方式

**方式一：单文件制**

直接在 commands 目录下创建 `.md` 文件：

```
commands/
  review.md
  test.md
```

**方式二：目录制**

创建目录并在其中放置 `COMMAND.md`（适合复杂的命令，可附带资源文件）：

```
commands/
  review/
    COMMAND.md
    checklist.txt
```

### 示例

创建一个 `plan.md`：

```markdown
---
name: plan
description: 进入 PLAN 模式
---
请进入 plan 模式规划任务
```

使用：

```
@command:plan
```

### 管理命令

在 TUI 中按 `Ctrl+E` 打开配置界面，切换到 Commands 标签页，可以启用或禁用命令。
