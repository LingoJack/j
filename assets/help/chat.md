---
name: AI 对话
order: 6
---

## AI 对话

| 命令 | 说明 |
|------|------|
| `j chat` / `j ai` | 进入 TUI 对话界面（全屏交互） |
| `j chat 你好` / `j ai 你好` | 快速发送消息并打印回复（oneshot 模式） |
| `j ai --remote` | 启用远程控制模式（手机扫码控制） |
| `j ai --remote --port 8080` | 指定远程控制端口（默认 9390） |
| `j ai -c` | 延续上一个会话（oneshot 模式） |
| `j ai --session <id>` | 指定会话 ID 继续 |
| `j ai --no-render 你好` | 禁用 Markdown 渲染，直接输出原始文本 |

> **提示**：oneshot 模式下，提问内容如果包含 `|`、`>`、`<`、`&` 等 shell 特殊字符，需要用引号包裹，否则会被 shell 解释为管道/重定向。例如：`j ai "请解释 ls | grep rs 的含义"`

### 远程控制模式

`--remote` 参数启用远程控制功能，可通过手机扫码在网页端控制终端中的 AI 对话：

```bash
j ai --remote              # 默认端口 9390
j ai --remote --port 8080  # 自定义端口
j ai -c --remote           # 延续会话 + 远程控制
```

启动后会显示二维码，手机扫描后即可在浏览器中操作。

> 注意：`--remote` 模式会强制进入 TUI 界面，忽略消息内容参数

### 配置

首次运行 `j chat` 时，若尚未配置模型提供方，会自动进入内置配置界面完成初始配置。已有配置后，也可随时在对话界面中按 **Ctrl+E** 或输入 `/config` 重新编辑。

配置文件路径: `~/.jdata/agent/data/agent_config.json`（也可手动编辑）

```json
{
  "providers": [
    {
      "name": "GPT-4o",
      "api_base": "https://api.openai.com/v1",
      "api_key": "sk-your-api-key",
      "model": "gpt-4o",
      "supports_vision": true
    }
  ],
  "active_index": 0,
  "max_history_messages": 20,
  "max_context_tokens": 100000,
  "theme": "midnight",
  "tools_enabled": true,
  "max_tool_rounds": 10,
  "tool_confirm_timeout": 0,
  "auto_restore_session": false
}
```

> 支持配置多个模型提供方，可在对话中切换

### 配置界面

按 `Ctrl+E` 或输入 `/config` 进入可视化配置界面。当前界面包含 `Model`、`Session`、`Global`、`Tools`、`Skills`、`Hooks`、`Commands`、`Teammates`、`Archive` 九个 Tab，不同 Tab 的按键略有不同。

| 按键 | 功能 |
|------|------|
| `←` / `→` | 切换 Tab |
| `↑` / `↓` / `j` / `k` | 在当前列表中移动 |
| `Enter` | 编辑字段 / 执行当前项动作 |
| `Esc` | 保存配置并返回对话 |

**Model Tab**：
- `Tab` / `Shift+Tab`：切换 Provider
- `a`：新增 Provider
- `d`：删除当前 Provider
- `s`：将当前 Provider 设为活跃模型

**Tools / Skills Tab**：
- `Enter` / `空格`：启用或禁用当前项
- `a`：全部启用
- `d`：全部禁用
- `t`：仅 Tools Tab 可切换总开关

**Session / Archive / Teammates Tab**：
- `Session`：`Enter` 恢复，`d` 删除，`n` 新建
- `Archive`：`Enter` 还原，`d` 删除
- `Teammates`：`Enter` 查看状态，`s` 停止选中 teammate

### 主题风格

支持以下主题（输入 `/theme` 切换或在配置界面中修改）：

| 主题 | 说明 |
|------|------|
| `midnight` | Midnight（默认） |
| `dark` | 深色主题 |
| `light` | 浅色主题 |
| `nord` | Nord 配色 |
| `monokai` | Monokai 配色 |
| `anthropic_light` | Anthropic Light |
| `anthropic_dark` | Anthropic Dark |

### 对话界面快捷键

| 按键 | 功能 |
|------|------|
| `Enter` | 发送消息 |
| `Shift+Enter` / `Alt+Enter` | 插入换行符（多行输入） |
| `↑` / `↓` | 滚动对话记录（多行输入时移动光标） |
| `PageUp` / `PageDown` | 快速滚动 |
| `←` / `→` | 移动输入光标 |
| `Home` / `End` | 跳到输入行首/行尾 |
| `Backspace` / `Delete` | 删除字符 |
| `Ctrl+Y` | 复制最后一条 AI 回复 |
| `Ctrl+B` | 进入消息浏览模式 |
| `Ctrl+G` | 打开日志窗口 |
| `Ctrl+M` | 切换鼠标模式（滚动 / 自由选中） |
| `Ctrl+O` | 切换工具详情展开/折叠 |
| `Ctrl+E` | 打开配置界面 |
| `F1` / `?`（输入框为空时） | 显示帮助 |
| `Esc` | 取消当前流式输出 / 退出对话 |
| `Ctrl+C` | 强制退出对话 |

### 斜杠命令（/ 命令）

在输入框中输入 `/` 即可唤起斜杠命令弹窗，支持模糊过滤：

| 命令 | 说明 |
|------|------|
| `/copy` | 复制最后一条 AI 回复 |
| `/log` | 打开日志窗口 |
| `/browse` | 浏览历史消息 |
| `/config` | 打开配置界面 |
| `/model` | 切换模型 |
| `/archive` | 归档当前对话 |
| `/clear` | 新建对话（清空当前会话） |
| `/theme` | 切换主题 |
| `/resume` | 恢复历史会话 |
| `/dump` | 导出原始 session messages（未经处理管线） |
| `/dump-processed` | 导出经完整处理管线（window → compact → hooks → sanitize）后的最终数据 |
| `/teammate` | 打开 Teammate 面板 |

### @ 补全系统

在输入框中输入 `@` 唤起补全弹窗，支持多种引用类型：

| 输入 | 补全类型 | 说明 |
|------|----------|------|
| `@` | 混合列表 | 弹出分类入口（skill:、command:、file:）及匹配项 |
| `@skill:` | 技能补全 | 从已安装技能中搜索并补全 |
| `@command:` | 命令补全 | 从内置命令中搜索并补全 |
| `@file:` | 文件补全 | 从当前工作目录搜索文件路径，支持目录导航 |

> 补全弹窗操作：`↑↓` 选择、`Tab`/`Enter` 确认、`Esc` 取消、`Backspace` 回退、`空格` 关闭弹窗

### 消息浏览模式

按 `Ctrl+B` 或输入 `/browse` 进入浏览模式，可选中任意历史消息并复制到剪切板：

| 按键 | 功能 |
|------|------|
| `↑` / `k` | 选中上一条消息 |
| `↓` / `j` | 选中下一条消息 |
| `PageUp` | 当前消息内容向上微调滚动 |
| `PageDown` | 当前消息内容向下微调滚动 |
| `Tab` | 切换角色过滤（全部 / AI / 用户） |
| `y` / `Enter` | 复制选中消息到剪切板 |
| 直接输入字符 | 按关键词过滤消息 |
| `Backspace` | 删除过滤字符 |
| `Esc` | 有过滤时先清除过滤；无过滤时返回对话模式 |

### 归档对话功能

对话支持归档和还原，方便保存有价值的对话历史：

**归档对话（/archive）**：
- 输入 `/archive` 后确认，当前对话会被保存到归档
- 默认归档名称格式：`archive-YYYY-MM-DD`
- 如果同名归档已存在，自动添加后缀（如 `archive-2026-02-25(1)`）
- 归档后当前会话自动清空

**恢复会话（/resume）**：
- 输入 `/resume` 进入会话列表
- 使用 `↑` / `↓` 或 `j` / `k` 选择历史会话
- 按 `Enter` 恢复选中的会话

### Teammate 系统

Teammate 是持久运行的子 Agent，拥有独立的上下文和消息历史，可通过 `/teammate` 或配置界面（Ctrl+E → Teammates Tab）查看和管理。

支持三种 Agent 执行模式：**Sub-Agent**（单次任务）、**Teammate**（持久协作）、**AgentTeam**（批量创建），详见「工具 & 权限」Tab。

**消息可见性**：

| 消息来源 | 主 Agent LLM 可见 | 主 Agent UI 可见 | 其他 Teammate 可见 |
|----------|-------------------|------------------|--------------------|
| Teammate 的 SendMessage | ✅ | ✅ | ✅ |
| Teammate 的文字回复（非 SendMessage） | ❌ | ✅ | ❌ |
| Teammate 的工具调用 | ❌ | ✅（`[调用工具 X]`） | ❌ |
| 主 Agent 的消息 | — | ✅ | ✅（通过 broadcast） |

- **SendMessage** 是 teammate 之间、teammate→主 Agent 的正式通信工具，消息会广播给所有成员
- Teammate 的文字回复和工具调用仅在主 Agent 的 UI 中显示，不影响主 Agent 的 LLM 上下文
- 主 Agent 的消息通过 broadcast 自动投递给所有 teammate 的 pending 队列

> Agent/AgentTeam 启动的子 Agent 拥有独立的上下文窗口，避免干扰主对话

### 功能特性

- **Markdown 渲染**：AI 回复支持标题、加粗、斜体、行内代码、代码块（语法高亮）、列表、表格、引用块
- **代码高亮**：支持 Rust、Python、JavaScript/TypeScript、Go、Java、Bash/Shell、C/C++、SQL、Ruby 等语言
- **流式渲染**：TUI 会在生成过程中持续刷新回复内容和工具状态
- **对话持久化**：对话自动保存到 `~/.jdata/agent/data/sessions/`，重启后恢复
- **多模型支持**：可配置多个 LLM 提供方（OpenAI、DeepSeek 等），运行时通过 `/model` 切换
- **工具调用**：支持 Function Calling，AI 可执行 shell 命令和读取文件（危险命令需确认）
- **Context Compact**：三层对话压缩机制（rolling window + micro_compact + auto_compact），自动管理上下文窗口
- **多行输入**：支持 `Shift+Enter` / `Alt+Enter` 插入换行符，多行输入时方向键移动光标
- **@ 引用系统**：支持引用技能（`@skill:`）、命令（`@command:`）、文件（`@file:`）到对话中
- **斜杠命令**：输入 `/` 弹出命令面板，快速执行操作
