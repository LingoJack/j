## 一键安装（推荐）

```bash
# 安装最新版本
curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh

# 安装指定版本
curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh -s -- v1.0.0
```

## 从 crates.io 安装

```bash
# 标准版（Lite 浏览器模式，无额外依赖）
cargo install j-cli

# 完整版（CDP 浏览器模式，需要 Chrome/Chromium）
cargo install j-cli --features browser_cdp
```

## 从源码构建

```bash
git clone https://github.com/LingoJack/j.git
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

```bash
# 使用安装脚本（推荐）
curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh -s -- --uninstall

# 或通过 cargo
cargo uninstall j-cli

# 或手动删除
sudo rm /usr/local/bin/j  # 一键安装
rm ~/.cargo/bin/j          # Cargo 安装

# （可选）删除数据目录
rm -rf ~/.jdata
```
