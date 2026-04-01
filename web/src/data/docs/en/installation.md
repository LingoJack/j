## One-click Install (Recommended)

```bash
# Install latest version
curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh

# Install specific version
curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh -s -- v1.0.0
```

## Install from crates.io

```bash
# Standard version (Lite browser mode, no extra dependencies)
cargo install j-cli

# Full version (CDP browser mode, requires Chrome/Chromium)
cargo install j-cli --features browser_cdp
```

## Build from Source

```bash
git clone https://github.com/LingoJack/j.git
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

```bash
# Using install script (recommended)
curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh -s -- --uninstall

# Or via cargo
cargo uninstall j-cli

# Or manual removal
sudo rm /usr/local/bin/j  # One-click install
rm ~/.cargo/bin/j          # Cargo install

# (Optional) Remove data directory
rm -rf ~/.jdata
```
