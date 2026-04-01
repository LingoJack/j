## Register App Aliases

```bash
j set chrome "/Applications/Google Chrome.app"
j set vscode "/Applications/Visual Studio Code.app"

# Register URL aliases (auto-detected as inner_url)
j set github https://github.com
```

## Mark Categories

```bash
j note chrome browser
j note vscode editor
```

## Open Apps

```bash
j chrome                  # Open Chrome
j chrome github           # Open github URL with Chrome
j chrome "rust lang"      # Search "rust lang" with Chrome
j vscode ./src            # Open src directory with VSCode
```

## Daily Reports

```bash
j report "Completed feature development"
j check                   # View recent 10 lines
j check 20                # View recent 20 lines
```

## Todo Management

```bash
j todo add Buy milk       # Quick add
j todo                    # Enter TUI manager
```

## AI Chat

```bash
j chat                    # Enter TUI chat
j chat Hello              # Quick question
```

## Interactive Mode

```bash
j                         # Enter interactive mode with Tab completion
```
