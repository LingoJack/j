## Commands

| Command | Description |
|---------|-------------|
| `j set <alias> <path>` | Set alias (paths → path section, URLs → inner_url) |
| `j rm <alias>` | Remove alias (cleans associated category marks) |
| `j rename <alias> <new>` | Rename alias (updates all category references) |
| `j mf <alias> <new_path>` | Modify alias path |

## Category Marking

```bash
j note <alias> <category>   # Mark alias with category
j find <category>           # Find aliases by category
j note chrome browser       # Mark chrome as browser
j note github outer_url     # Mark github as outer_url (auto-connect VPN)
```

## Categories

| Category | Description |
|----------|-------------|
| `browser` | Web browsers |
| `editor` | Code editors |
| `vpn` | VPN applications |
| `script` | Custom scripts |
| `inner_url` | Internal URLs |
| `outer_url` | External URLs (auto-connect VPN) |

## Examples

```bash
# Set app aliases
j set chrome "/Applications/Google Chrome.app"
j set safari "/Applications/Safari.app"
j set vscode "/Applications/Visual Studio Code.app"

# Set URL aliases
j set github https://github.com
j set google https://google.com

# Set directory aliases
j set proj ~/Projects
j set docs ~/Documents

# Open apps
j chrome                   # Open Chrome
j chrome "rust lang"       # Search with Chrome
j chrome github            # Open github URL with Chrome
j vscode proj              # Open proj directory with VSCode
```
