## Register App Aliases

### macOS / Linux

```bash
j set chrome "/Applications/Google Chrome.app"
j set vscode "/Applications/Visual Studio Code.app"

# Register URL aliases (auto-detected as inner_url)
j set github https://github.com
```

### Windows

```powershell
j set notepad "C:\Windows\notepad.exe"
j set vscode "C:\Users\%USERNAME%\AppData\Local\Programs\Microsoft VS Code\Code.exe"

# Register URL aliases
j set github https://github.com
```

## Mark Categories

```bash
j note chrome browser
j note vscode editor
```

## Open Apps

### macOS / Linux

```bash
j chrome                  # Open Chrome
j chrome github           # Open github URL with Chrome
j chrome "rust lang"      # Search "rust lang" with Chrome
j vscode ./src            # Open src directory with VSCode
```

### Windows

```powershell
j notepad                 # Open Notepad
j vscode .\src            # Open src directory with VSCode
j github                  # Open github URL
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