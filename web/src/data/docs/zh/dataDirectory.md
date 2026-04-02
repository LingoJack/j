所有数据存储在 `~/.jdata/` 目录（可通过 `J_DATA_PATH` 环境变量自定义）：

```
~/.jdata/
├── config.yaml          # 主配置（别名、分类、设置）
├── history.txt          # 命令历史
├── agent/               # AI Agent 数据
│   ├── data/            # Agent 数据目录
│   │   ├── agent_config.yaml   # Agent 配置（模型、API）
│   │   ├── sessions/           # 对话会话存储
│   │   ├── archives/           # 归档对话
│   │   ├── system_prompt.md    # 系统提示词
│   │   ├── memory.md           # 记忆文件
│   │   └── soul.md             # 灵魂文件
│   ├── logs/            # Agent 日志
│   │   ├── info.log
│   │   └── error.log
│   ├── skills/          # 用户级技能目录
│   ├── commands/        # 用户级自定义命令
│   └── hooks.yaml       # 用户级钩子配置
├── report/              # 日报数据
│   ├── week_report.md   # 周报文件
│   ├── settings.json    # 报告设置
│   ├── todo.json        # 待办数据
│   └── .git/            # Git 仓库
└── scripts/             # 通过 j concat 创建的脚本
```

## 项目级配置

项目目录下可创建 `.jcli/` 存放项目级配置：

```
.jcli/
├── config.yaml          # 项目级配置
├── permissions.yaml     # 工具权限配置
├── hooks.yaml           # 项目级钩子
├── skills/              # 项目级技能（覆盖用户级）
└── commands/            # 项目级自定义命令
```

## 配置文件结构（`config.yaml`）

| 配置项 | 描述 | 示例 |
|--------|------|------|
| `path` | 本地应用/文件路径 | `chrome: /Applications/Google Chrome.app` |
| `inner_url` | URL 链接 | `github: https://github.com` |
| `outer_url` | 需要 VPN 的 URL | `docs: https://internal.example.com` |
| `browser` | 浏览器列表 | `chrome: chrome` |
| `editor` | 编辑器列表 | `vscode: vscode` |
| `vpn` | VPN 应用 | |
| `script` | 注册脚本 | `deploy: ~/.jdata/scripts/deploy.sh` |
| `report` | 日报系统配置 | `git_repo: https://github.com/xxx/report` |
| `setting` | 全局设置 | `search-engine: bing` |
| `log` | 日志设置 | `mode: concise` |

## Agent 配置（`agent_config.yaml`）

| 配置项 | 描述 | 默认值 |
|--------|------|--------|
| `providers` | 模型提供方列表 | - |
| `active_index` | 当前选中的 provider 索引 | 0 |
| `system_prompt` | 系统提示词 | - |
| `stream_mode` | 流式输出 | true |
| `max_history_messages` | 发送给 API 的历史消息数量限制 | 20 |
| `tools_enabled` | 启用工具调用 | false |
| `max_tool_rounds` | 工具调用最大轮数 | 100 |
| `tool_confirm_timeout` | 工具确认超时秒数 | 0（不超时） |
| `disabled_tools` | 禁用的工具列表 | [] |
| `disabled_skills` | 禁用的 skill 列表 | [] |
| `disabled_commands` | 禁用的 command 列表 | [] |
| `auto_restore_session` | 启动时自动恢复最近的 session | false |
