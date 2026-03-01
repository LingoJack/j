SHELL := /bin/bash

# ============================================
# 变量定义
# ============================================
BIN_PATH := /usr/local/bin/j
TARGET_DIR := target/release
MD_RENDER_DIR := plugin/md_render
VERSION := $(shell grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
GIT_BRANCH := $(shell git rev-parse --abbrev-ref HEAD)

# ============================================
# 伪目标声明
# ============================================
.PHONY: help \
        current_dir push pull status \
        build release debug \
        install uninstall reinstall \
        publish publish-check tag tags \
        test test-all bench \
        fmt lint check clippy \
        clean clean-all \
        doc docs \
        run run-release \
        md_render test-install \
        deps update-deps \
        watch watch-test \
        coverage \
        docker-build docker-run \
        pre-commit

# ============================================
# 帮助信息
# ============================================
help: ## 显示此帮助信息
	@echo "📚 j-cli Makefile 帮助"
	@echo "============================================"
	@echo "版本: $(VERSION) | 分支: $(GIT_BRANCH)"
	@echo "============================================"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'
	@echo ""
	@echo "📋 常用命令:"
	@echo "  make build      # 构建项目"
	@echo "  make install    # 安装到系统"
	@echo "  make test       # 运行测试"
	@echo "  make fmt        # 格式化代码"
	@echo "  make clean      # 清理构建产物"

# ============================================
# 目录和 Git 操作
# ============================================
current_dir: ## 显示当前目录信息
	@echo "🔍 当前目录信息:"
	@echo "======================================"
	@echo "目录: $$(pwd)"
	@echo "版本: $(VERSION)"
	@echo "分支: $(GIT_BRANCH)"
	@echo "======================================"

push: current_dir fmt ## 提交并推送代码
	@echo "📤 推送代码到远程仓库..."
	@git add .\
	&& (git commit -m "更新: $(shell date +'%Y-%m-%d %H:%M:%S')" || exit 0) \
	&& git push origin $(GIT_BRANCH)
	@echo "✅ 代码已推送"

pull: current_dir ## 拉取最新代码
	@echo "📥 拉取最新代码..."
	@git pull origin $(GIT_BRANCH)
	@echo "✅ 代码已更新"

status: current_dir ## 查看 Git 状态
	@git status

# ============================================
# 构建相关
# ============================================
build: ## 构建项目（调试模式）
	@echo "🔨 构建项目..."
	@cargo build
	@echo "✅ 构建完成"

release: current_dir ## 构建发布版本
	@echo "🚀 构建发布版本..."
	@cargo build --release
	@echo "✅ 发布版本构建完成: $(TARGET_DIR)/j"

debug: ## 构建调试版本
	@echo "🐛 构建调试版本..."
	@cargo build --debug
	@echo "✅ 调试版本构建完成"

# ============================================
# 安装相关
# ============================================
install: release ## 安装到系统
	@echo "📦 安装到系统..."
	@cp $(TARGET_DIR)/j $(BIN_PATH)
	@chmod +x $(BIN_PATH)
	@echo "✅ j 已安装到 $(BIN_PATH)"
	@echo "   版本: $(VERSION)"

uninstall: ## 卸载
	@echo "🗑️  卸载..."
	@rm -f $(BIN_PATH)
	@echo "✅ j 已卸载"

reinstall: uninstall install ## 重新安装
	@echo "🔄 重新安装完成"

# ============================================
# 发布相关
# ============================================
publish: md_render push tag release ## 发布到 crates.io
	@echo "📦 发布到 crates.io..."
	@cargo publish --registry crates-io
	@echo "✅ 已发布! 验证: cargo search j-cli"

publish-check: ## 发布前检查（dry-run）
	@echo "🔍 发布前检查（dry-run）..."
	@cargo publish --registry crates-io --dry-run
	@echo "✅ 检查通过"

tag: ## 创建 git tag
	@version=$(VERSION); \
	tag="v$$version"; \
	if git rev-parse "$$tag" >/dev/null 2>&1; then \
		echo "❌ 标签 $$tag 已存在 (Cargo.toml 版本 = $$version)"; \
		echo "   请先在 Cargo.toml 中更新版本号"; \
		exit 1; \
	fi; \
	echo "📌 创建标签 $$tag (来自 Cargo.toml)..."; \
	git tag -a "$$tag" -m "Release $$tag"; \
	git push origin "$$tag"; \
	echo "✅ 标签 $$tag 已创建并推送。GitHub Actions 将自动构建和发布。"

tags: ## 查看最近的标签
	@echo "🏷️  最近的标签:"
	@git tag -l | sort -V | tail -10

# ============================================
# 测试相关
# ============================================
test: ## 运行测试
	@echo "🧪 运行测试..."
	@cargo test
	@echo "✅ 测试完成"

test-all: ## 运行所有测试（包括集成测试）
	@echo "🧪 运行所有测试..."
	@cargo test --all-features
	@echo "✅ 所有测试完成"

bench: ## 运行性能测试
	@echo "⚡ 运行性能测试..."
	@cargo bench
	@echo "✅ 性能测试完成"

# ============================================
# 代码质量
# ============================================
fmt: ## 格式化代码
	@echo "🧹 格式化代码..."
	@cargo fmt
	@echo "✅ 代码格式化完成"

lint: ## 运行 clippy 检查
	@echo "🔍 运行 clippy 检查..."
	@cargo clippy -- -D warnings
	@echo "✅ clippy 检查完成"

check: ## 检查代码（不构建）
	@echo "🔍 检查代码..."
	@cargo check
	@echo "✅ 代码检查完成"

clippy: lint ## clippy 别名

pre-commit: fmt lint test ## 提交前检查
	@echo "✅ 所有检查通过，可以提交"

# ============================================
# 清理相关
# ============================================
clean: ## 清理构建产物
	@echo "🧹 清理构建产物..."
	@cargo clean
	@echo "✅ 清理完成"

clean-all: clean ## 彻底清理（包括依赖）
	@echo "🧹 彻底清理..."
	@rm -rf target/
	@echo "✅ 彻底清理完成"

# ============================================
# 文档相关
# ============================================
doc: ## 生成文档
	@echo "📚 生成文档..."
	@cargo doc --no-deps
	@echo "✅ 文档生成完成: target/doc/j_cli/index.html"

docs: doc ## 文档别名

# ============================================
# 运行相关
# ============================================
run: ## 运行项目
	@echo "🚀 运行项目..."
	@cargo run

run-release: release ## 运行发布版本
	@echo "🚀 运行发布版本..."
	@$(TARGET_DIR)/j

# ============================================
# 插件相关
# ============================================
md_render: ## 构建 md_render 插件
	@echo "🔄 构建 md_render 插件..."
	@cd $(MD_RENDER_DIR)/code \
	&& GOOS=darwin GOARCH=arm64 go build -o ../bin/md_render-darwin-arm64
	@echo "✅ md_render 插件构建完成: $(MD_RENDER_DIR)/bin/md_render-darwin-arm64"

test-install: ## 测试安装脚本
	@echo "🧪 测试安装脚本..."
	@./install.sh

# ============================================
# 依赖管理
# ============================================
deps: ## 显示依赖信息
	@echo "📦 依赖信息:"
	@cargo tree

update-deps: ## 更新依赖
	@echo "🔄 更新依赖..."
	@cargo update
	@echo "✅ 依赖更新完成"

# ============================================
# 开发工具
# ============================================
watch: ## 监视文件变化并重新构建
	@echo "👀 监视文件变化..."
	@cargo watch -x run

watch-test: ## 监视文件变化并运行测试
	@echo "👀 监视文件变化并运行测试..."
	@cargo watch -x test

coverage: ## 生成代码覆盖率报告
	@echo "📊 生成代码覆盖率报告..."
	@cargo tarpaulin --out Html
	@echo "✅ 覆盖率报告生成完成: tarpaulin-report.html"

# ============================================
# Docker 支持
# ============================================
docker-build: ## 构建 Docker 镜像
	@echo "🐳 构建 Docker 镜像..."
	@docker build -t j-cli:$(VERSION) .
	@echo "✅ Docker 镜像构建完成: j-cli:$(VERSION)"

docker-run: docker-build ## 运行 Docker 容器
	@echo "🐳 运行 Docker 容器..."
	@docker run -it --rm j-cli:$(VERSION)
