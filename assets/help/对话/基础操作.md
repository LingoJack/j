# AI 对话基础

| 命令 | 说明 |
|------|------|
| `j chat` / `j ai` | 进入 TUI 对话界面（全屏交互） |
| `j chat 你好` / `j ai 你好` | 快速发送消息并打印回复（oneshot 模式） |
| `j ai -c` | 延续上一个会话（oneshot 模式） |
| `j ai --session <id>` | 指定会话 ID 继续 |
| `j ai --no-render 你好` | 禁用 Markdown 渲染，直接输出原始文本 |

> oneshot 模式下，提问内容如果包含 `|`、`>`、`<`、`&` 等 shell 特殊字符，需要用引号包裹。例如：`j ai "请解释 ls | grep rs 的含义"`