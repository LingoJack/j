## One-click Install (Recommended)

### macOS / Linux

```bash
# Install latest version
curl -fsSL https://raw.githubusercontent.com/LingoJack/jcli/main/install.sh | sh

# Install specific version
curl -fsSL https://raw.githubusercontent.com/LingoJack/jcli/main/install.sh | sh -s -- v1.0.0
```

### Windows

```powershell
# Install latest version
irm https://raw.githubusercontent.com/LingoJack/jcli/main/install.ps1 | iex

# Install specific version
$v="v1.0.0"; irm https://raw.githubusercontent.com/LingoJack/jcli/main/install.ps1 | iex
```

> Windows install location: `%LOCALAPPDATA%\j-cli\j.exe`, automatically added to user PATH

## Install from crates.io

```bash
# Standard version (Lite browser mode, no extra dependencies)
cargo install j-cli

# Full version (CDP browser mode, requires Chrome/Chromium)
cargo install j-cli --features browser_cdp
```

## Build from Source

```bash
git clone https://github.com/LingoJack/jcli.git
cd j && cargo install --path .

# With full browser automation
cargo install --path . --features browser_cdp
```

## Verify Installation

```bash
j --version
j --help
```

## Update

```bash
# Built-in update command (auto-detects installation source)
j update

# Check version only
j update --check

# Manual update via cargo
cargo install j-cli
```

## Uninstall

### macOS / Linux

```bash
# Using install script (recommended)
curl -fsSL https://raw.githubusercontent.com/LingoJack/jcli/main/install.sh | sh -s -- --uninstall

# Or via cargo
cargo uninstall j-cli

# Or manual removal
sudo rm /usr/local/bin/j  # One-click install
rm ~/.cargo/bin/j          # Cargo install

# (Optional) Remove data directory
rm -rf ~/.jdata
```

### Windows

```powershell
# Using install script (recommended)
powershell -ExecutionPolicy Bypass -File install.ps1 -Uninstall

# Or manual removal
Remove-Item "$env:LOCALAPPDATA\j-cli" -Recurse -Force

# (Optional) Remove data directory
Remove-Item "$env:USERPROFILE\.jdata" -Recurse -Force
```

## Platform Differences

| Feature | macOS / Linux | Windows |
|---------|---------------|---------|
| AI Shell Tool | Bash | PowerShell |
| Default Script | `.sh` + bash shebang | `.cmd` |
| Auto Update | `.tar.gz` extract | `.zip` extract |
| Data Directory | `~/.jdata/` | `%USERPROFILE%\.jdata\` |
| Install Location | `/usr/local/bin/j` | `%LOCALAPPDATA%\j-cli\j.exe` |
| Computer Use | Supported | Not supported |
| j-indicator | Menu bar indicator | Not supported |
