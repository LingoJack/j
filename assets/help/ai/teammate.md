# Teammate 系统

Teammate 是持久运行的子 Agent，拥有独立的上下文和消息历史，可通过 `/teammate` 或配置界面（Ctrl+E -> Teammates Tab）查看和管理。

支持三种 Agent 执行模式：**Sub-Agent**（单次任务）、**Teammate**（持久协作）、**AgentTeam**（批量创建），详见「工具」目录。

## 消息可见性

| 消息来源 | 主 Agent LLM 可见 | 主 Agent UI 可见 | 其他 Teammate 可见 |
|----------|-------------------|------------------|--------------------|
| Teammate 的 SendMessage | yes | yes | yes |
| Teammate 的文字回复（非 SendMessage） | no | yes | no |
| Teammate 的工具调用 | no | yes（`[调用工具 X]`） | no |
| 主 Agent 的消息 | -- | yes | yes（通过 broadcast） |

- **SendMessage** 是 teammate 之间、teammate 到主 Agent 的正式通信工具，消息会广播给所有成员
- Teammate 的文字回复和工具调用仅在主 Agent 的 UI 中显示，不影响主 Agent 的 LLM 上下文
- 主 Agent 的消息通过 broadcast 自动投递给所有 teammate 的 pending 队列

> Agent/AgentTeam 启动的子 Agent 拥有独立的上下文窗口，避免干扰主对话