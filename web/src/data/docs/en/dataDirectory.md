All data is stored in `~/.jdata/` (customizable via `J_DATA_PATH` environment variable):

```
~/.jdata/
├── config.yaml          # Main config (aliases, categories, settings)
├── agent/               # AI Agent data
│   ├── data/            # Agent data directory
│   │   ├── agent_config.json   # Agent config (model, API)
│   │   ├── chat_history.json   # Chat history
│   │   ├── archives/           # Archived conversations
│   │   ├── system_prompt.md    # System prompt
│   │   ├── memory.md           # Memory file
│   │   ├── soul.md             # Soul file
│   │   └── style.md            # Response style
│   ├── logs/            # Agent logs
│   │   ├── info.log
│   │   └── error.log
│   └── skills/          # Skills directory
├── bin/                 # Built-in tools
│   └── md_render        # Markdown renderer
├── report/              # Daily reports
│   ├── week_report.md   # Week report file
│   ├── settings.json    # Report settings
│   ├── todo.json        # Todo data
│   └── .git/            # Git repository
├── scripts/             # Scripts created via j concat
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
