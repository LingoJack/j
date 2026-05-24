#!/bin/bash

# j-cli 安装脚本
# 使用方式: curl -fsSL https://raw.githubusercontent.com/LingoJack/jcli/main/install.sh | sh
# 或者指定版本: curl -fsSL https://raw.githubusercontent.com/LingoJack/jcli/main/install.sh | sh -s -- v12.10.64

set -e

# 配置
REPO="LingoJack/jcli"
BINARY_NAME="j"
INSTALL_DIR="/usr/local/bin"
DATA_DIR="$HOME/.jdata"
DEFAULT_VERSION="v12.10.71"  # 备用默认版本（publish 时自动更新）


# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

info() {
    printf "${GREEN}[INFO]${NC} %s\n" "$1" >&2
}

warn() {
    printf "${YELLOW}[WARN]${NC} %s\n" "$1" >&2
}

error() {
    printf "${RED}[ERROR]${NC} %s\n" "$1" >&2
    exit 1
}

# 检测操作系统和架构
detect_platform() {
    OS=$(uname -s | tr '[:upper:]' '[:lower:]')
    ARCH=$(uname -m)

    # 只支持 macOS ARM64 (M 系列)
    if [ "$OS" != "darwin" ]; then
        error "当前仅支持 macOS (M1/M2/M3/M4)，检测到: $OS"
    fi

    if [ "$ARCH" != "arm64" ]; then
        error "当前仅支持 Apple Silicon (M1/M2/M3/M4)，检测到: $ARCH"
    fi

    echo "darwin-arm64"
}

# 获取最新版本号
get_latest_version() {
    local latest
    local api_url="https://api.github.com/repos/${REPO}/releases/latest"
    
    # 方法1: 尝试使用 gh auth token 如果可用
    if command -v gh &> /dev/null && gh auth status &> /dev/null 2>&1; then
        info "使用 GitHub CLI 认证..." >&2
        latest=$(curl -fsSL -H "User-Agent: j-cli-installer" -H "Authorization: token $(gh auth token)" "$api_url" 2>/dev/null | grep -oE '"tag_name"[[:space:]]*:[[:space:]]*"v[0-9]+\.[0-9]+\.[0-9]+"' | head -1 | grep -oE 'v[0-9]+\.[0-9]+\.[0-9]+')
    fi

    # 方法2: 直接访问 API（带 User-Agent）
    if [ -z "$latest" ]; then
        info "尝试从 GitHub API 获取..." >&2
        latest=$(curl -fsSL -H "User-Agent: j-cli-installer" "$api_url" 2>/dev/null | grep -oE '"tag_name"[[:space:]]*:[[:space:]]*"v[0-9]+\.[0-9]+\.[0-9]+"' | head -1 | grep -oE 'v[0-9]+\.[0-9]+\.[0-9]+')
    fi
    
    # 方法3: 跟随 releases/latest 重定向，直接从最终 URL 提取 tag
    if [ -z "$latest" ]; then
        info "尝试从 releases 页面获取..." >&2
        latest=$(curl -fsSL -o /dev/null -w "%{url_effective}" -H "User-Agent: j-cli-installer" "https://github.com/${REPO}/releases/latest" 2>/dev/null | grep -oE 'v[0-9]+\.[0-9]+\.[0-9]+')
    fi
    
    # 所有网络方法均失败，直接报错
    if [ -z "$latest" ]; then
        error "无法从网络获取最新版本，请检查网络连接后重试。如需安装指定版本，请使用: install.sh $DEFAULT_VERSION"
    fi
    
    # 验证版本号格式
    if ! echo "$latest" | grep -qE '^v[0-9]+\.[0-9]+\.[0-9]+$'; then
        error "获取到的版本号格式不正确: $latest，请检查网络连接后重试"
    fi
    
    echo "$latest"
}

# 下载并安装
install() {
    local version="$1"
    local platform="$2"

    if [ -z "$version" ]; then
        info "正在获取最新版本..."
        version=$(get_latest_version)
    fi

    info "安装版本: $version"
    info "平台: $platform"

    # 构建下载 URL
    local asset_name="j-${platform}"
    local download_url="https://github.com/${REPO}/releases/download/${version}/${asset_name}.tar.gz"

    info "下载地址: $download_url"

    # 创建临时目录
    local tmp_dir
    tmp_dir=$(mktemp -d)
    trap "rm -rf $tmp_dir" EXIT

    # 下载文件
    info "正在下载..."
    if ! curl -fsSL --progress-bar -H "User-Agent: j-cli-installer" -o "$tmp_dir/j.tar.gz" "$download_url"; then
        error "下载失败，请检查版本号是否正确或网络连接是否正常"
    fi

    # 解压
    info "正在解压..."
    tar -xzf "$tmp_dir/j.tar.gz" -C "$tmp_dir"

    # 检查安装目录是否存在，不存在则创建
    if [ ! -d "$INSTALL_DIR" ]; then
        info "创建安装目录 $INSTALL_DIR..."
        sudo mkdir -p "$INSTALL_DIR"
    fi

    # 检查安装目录权限
    if [ ! -w "$INSTALL_DIR" ]; then
        warn "安装目录 $INSTALL_DIR 需要管理员权限"
        SUDO="sudo"
    else
        SUDO=""
    fi

    # 安装
    info "正在安装到 $INSTALL_DIR..."
    $SUDO mv "$tmp_dir/$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME"
    $SUDO chmod +x "$INSTALL_DIR/$BINARY_NAME"

    # 安装 Swift helpers（如果存在）
    for helper in j-indicator j-ax; do
        if [ -f "$tmp_dir/$helper" ]; then
            $SUDO mv "$tmp_dir/$helper" "$INSTALL_DIR/$helper"
            $SUDO chmod +x "$INSTALL_DIR/$helper"
            info "已安装 $helper"
        fi
    done

    # 验证安装
    if [ -x "$INSTALL_DIR/$BINARY_NAME" ]; then
        info "☑️ 安装成功！"
        info ""
        info "安装位置: $INSTALL_DIR/$BINARY_NAME"
        info "数据目录: $DATA_DIR"
        info ""
        info "运行 'j version' 查看版本信息"
        info "运行 'j help' 查看帮助文档"
    else
        error "安装失败"
    fi
}

# 卸载
uninstall() {
    info "正在卸载..."
    
    if [ -f "$INSTALL_DIR/$BINARY_NAME" ]; then
        if [ ! -w "$INSTALL_DIR" ]; then
            SUDO="sudo"
        else
            SUDO=""
        fi
        $SUDO rm -f "$INSTALL_DIR/$BINARY_NAME"
        $SUDO rm -f "$INSTALL_DIR/j-indicator"
        $SUDO rm -f "$INSTALL_DIR/j-ax"
        info "☑️ 已卸载程序及 helpers"
    else
        warn "程序未安装"
    fi

    info ""
    warn "数据目录 $DATA_DIR 未删除，如需彻底清理请手动执行:"
    echo "  rm -rf $DATA_DIR"
}

# 显示帮助
show_help() {
    echo "j-cli 安装脚本"
    echo ""
    echo "使用方式:"
    echo "  curl -fsSL https://raw.githubusercontent.com/${REPO}/main/install.sh | sh"
    echo ""
    echo "指定版本安装:"
    echo "  curl -fsSL https://raw.githubusercontent.com/${REPO}/main/install.sh | sh -s -- $DEFAULT_VERSION"
    echo ""
    echo "卸载:"
    echo "  curl -fsSL https://raw.githubusercontent.com/${REPO}/main/install.sh | sh -s -- --uninstall"
    echo ""
    echo "选项:"
    echo "  --uninstall    卸载程序"
    echo "  --help         显示帮助信息"
    echo "  --version      指定安装版本"
}

# 主入口
main() {
    local version=""
    local action="install"

    # 解析参数
    while [ $# -gt 0 ]; do
        case "$1" in
            --uninstall)
                action="uninstall"
                shift
                ;;
            --help|-h)
                show_help
                exit 0
                ;;
            v*)
                version="$1"
                shift
                ;;
            *)
                shift
                ;;
        esac
    done

    echo ""
    echo "╔══════════════════════════════════════╗"
    echo "║       j-cli 安装程序                 ║"
    echo "║   快捷命令行工具 - Rust 实现         ║"
    echo "╚══════════════════════════════════════╝"
    echo ""

    case "$action" in
        install)
            platform=$(detect_platform)
            install "$version" "$platform"
            ;;
        uninstall)
            uninstall
            ;;
    esac
}

main "$@"
