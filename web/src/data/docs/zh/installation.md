## 一键安装（推荐）

### macOS / Linux

```bash
# 安装最新版本
curl -fsSL https://raw.githubusercontent.com/LingoJack/jcli/main/install.sh | sh

# 安装指定版本
curl -fsSL https://raw.githubusercontent.com/LingoJack/jcli/main/install.sh | sh -s -- v1.0.0
```

### Windows

```powershell
# 安装最新版本
irm https://raw.githubusercontent.com/LingoJack/jcli/main/install.ps1 | iex

# 安装指定版本
$v="v1.0.0"; irm https://raw.githubusercontent.com/LingoJack/jcli/main/install.ps1 | iex
```

> Windows 安装位置: `%LOCALAPPDATA%\j-cli\j.exe`，自动添加到用户 PATH

## 从 crates.io 安装

```bash
# 标准版（Lite 浏览器模式，无额外依赖）
cargo install j-cli

# 完整版（CDP 浏览器模式，需要 Chrome/Chromium）
cargo install j-cli --features browser_cdp
```

## 从源码构建

```bash
git clone https://github.com/LingoJack/jcli.git
cd j && cargo install --path .

# 包含完整浏览器自动化功能
cargo install --path . --features browser_cdp
```

## 验证安装

```bash
j --version
j --help
```

## 更新

```bash
# 使用内置更新命令（自动检测安装来源）
j update

# 仅检查版本
j update --check

# 通过 cargo 手动更新
cargo install j-cli
```

## 卸载

### macOS / Linux

```bash
# 使用安装脚本（推荐）
curl -fsSL https://raw.githubusercontent.com/LingoJack/jcli/main/install.sh | sh -s -- --uninstall

# 或通过 cargo
cargo uninstall j-cli

# 或手动删除
sudo rm /usr/local/bin/j  # 一键安装
rm ~/.cargo/bin/j          # Cargo 安装

# （可选）删除数据目录
rm -rf ~/.jdata
```

### Windows

```powershell
# 使用安装脚本（推荐）
powershell -ExecutionPolicy Bypass -File install.ps1 -Uninstall

# 或手动删除
Remove-Item "$env:USERPROFILE\.jdata\bin" -Recurse -Force

# （可选）删除数据目录
Remove-Item "$env:USERPROFILE\.jdata" -Recurse -Force
```

## 平台差异说明

| 功能 | macOS / Linux | Windows |
|------|---------------|---------|
| AI Shell 工具 | Bash | PowerShell |
| 默认脚本 | `.sh` + bash shebang | `.cmd` |
| 自动更新 | `.tar.gz` 解压 | `.zip` 解压 |
| 数据目录 | `~/.jdata/` | `%USERPROFILE%\.jdata\` |
| 安装位置 | `/usr/local/bin/j` | `%LOCALAPPDATA%\j-cli\j.exe` |
| Computer Use | 支持 | 不支持 |
| j-indicator | 菜单栏指示灯 | 不支持 |
