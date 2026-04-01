所有数据存储在 `~/.jdata/` 目录（可通过 `J_DATA_PATH` 环境变量自定义）：

```
~/.jdata/
├── config.yaml          # 主配置（别名、分类、设置）
├── agent/               # AI Agent 数据
│   ├── data/            # Agent 数据目录
│   │   ├── agent_config.json   # Agent 配置（模型、API）
│   │   ├── chat_history.json   # 对话历史
│   │   ├── archives/           # 归档对话
│   │   ├── system_prompt.md    # 系统提示词
│   │   ├── memory.md           # 记忆文件
│   │   ├── soul.md             # 灵魂文件
│   │   └── style.md            # 响应风格
│   ├── logs/            # Agent 日志
│   │   ├── info.log
│   │   └── error.log
│   └── skills/          # 技能目录
├── bin/                 # 内置工具
│   └── md_render        # Markdown 渲染器
├── report/              # 日报数据
│   ├── week_report.md   # 周报文件
│   ├── settings.json    # 报告设置
│   ├── todo.json        # 待办数据
│   └── .git/            # Git 仓库
├── scripts/             # 通过 j concat 创建的脚本
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
