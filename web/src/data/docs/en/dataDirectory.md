All data is stored in `~/.jdata/` (customizable via `J_DATA_PATH` environment variable):

```
~/.jdata/
├── config.yaml          # Main config (aliases, categories, settings)
├── history.txt          # Command history
├── agent/               # AI Agent data
│   ├── data/            # Agent data directory
│   │   ├── agent_config.yaml   # Agent config (model, API)
│   │   ├── sessions/           # Chat sessions storage
│   │   ├── archives/           # Archived conversations
│   │   ├── system_prompt.md    # System prompt
│   │   ├── memory.md           # Memory file
│   │   └── soul.md             # Soul file
│   ├── logs/            # Agent logs
│   │   ├── info.log
│   │   └── error.log
│   ├── skills/          # User-level skills directory
│   ├── commands/        # User-level custom commands
│   └── hooks.yaml       # User-level hooks config
├── report/              # Daily reports
│   ├── week_report.md   # Week report file
│   ├── settings.json    # Report settings
│   ├── todo.json        # Todo data
│   └── .git/            # Git repository
└── scripts/             # Scripts created via j concat
```

## Project-level Config

Create `.jcli/` in project directory for project-level configuration:

```
.jcli/
├── config.yaml          # Project-level config
├── permissions.yaml     # Tool permissions
├── hooks.yaml           # Project-level hooks
├── skills/              # Project-level skills (override user-level)
└── commands/            # Project-level custom commands
```

## Config File Structure (`config.yaml`)

| Section | Description | Example |
|---------|-------------|---------|
| `path` | Local app/file paths | `chrome: /Applications/Google Chrome.app` |
| `inner_url` | URL links | `github: https://github.com` |
| `outer_url` | URLs requiring VPN | `docs: https://internal.example.com` |
| `browser` | Browser list | `chrome: chrome` |
| `editor` | Editor list | `vscode: vscode` |
| `vpn` | VPN application | |
| `script` | Registered scripts | `deploy: ~/.jdata/scripts/deploy.sh` |
| `report` | Report system config | `git_repo: https://github.com/xxx/report` |
| `setting` | Global settings | `search-engine: bing` |
| `log` | Log settings | `mode: concise` |

## Agent Config (`agent_config.yaml`)

| Setting | Description | Default |
|---------|-------------|---------|
| `providers` | Model provider list | - |
| `active_index` | Current active provider index | 0 |
| `system_prompt` | System prompt | - |
| `stream_mode` | Stream output | true |
| `max_history_messages` | Max history messages sent to API | 20 |
| `tools_enabled` | Enable tool calling | false |
| `max_tool_rounds` | Max tool call rounds | 100 |
| `tool_confirm_timeout` | Tool confirm timeout seconds | 0 (no timeout) |
| `disabled_tools` | Disabled tools list | [] |
| `disabled_skills` | Disabled skills list | [] |
| `disabled_commands` | Disabled commands list | [] |
| `auto_restore_session` | Auto restore last session on startup | false |
