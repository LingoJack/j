SHELL := /bin/bash

# ============================================
# 变量定义
# ============================================
INSTALL_DIR := /usr/local/bin
REPO := LingoJack/jcli
TARGET_DIR := target/release
VERSION := $(shell grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
GIT_BRANCH := $(shell git rev-parse --abbrev-ref HEAD)

# ============================================
# 伪目标声明
# ============================================
.PHONY: help \
        current_dir push push-ai pull status \
        build release debug build-indicator build-ax \
        install uninstall reinstall \
        publish publish-check tag tags bump-version set-version \
        release-note \
        test test-all bench \
        fmt lint check clippy check-lint \
        clean clean-all \
        doc docs \
        run run-release \
        test-install \
        deps update-deps \
        watch watch-test \
        coverage \
        docker-build docker-run \
        pre-commit \
        build-remote \
        gui-dev gui-build gui-install gui-clean

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

push: current_dir fmt build-web ## 提交并推送代码
	@echo "📤 推送代码到远程仓库..."
	@git add .\
	&& (git commit -m "更新: $(shell date +'%Y-%m-%d %H:%M:%S')" || exit 0) \
	&& git push origin $(GIT_BRANCH)
	@echo "☑️ 代码已推送"

# --- j ai 输出提取辅助函数 ---
# prompt 中要求 AI 用 <result>...</result> 包裹输出
# 管道中直接用 awk 抓取标签内容，无需过滤任何噪音
# 支持单行 <result>xxx</result> 和多行 <result>\n...\n</result>
define J_AI_EXTRACT
awk '/<result>/{in_r=1;gsub(/.*<result>/,"")}/<\/result>/{gsub(/<\/result>.*/,"");in_r=0;print;next}in_r{print}'
endef

push-ai: current_dir fmt build-web ## AI 生成 commit message 并推送
	@echo "🤖 AI 生成变更说明..."
	@diff_stat="$$(git diff --stat 2>/dev/null)"; \
	if [ -z "$$diff_stat" ]; then \
		diff_stat="$$(git diff --cached --stat 2>/dev/null)"; \
	fi; \
	if [ -z "$$diff_stat" ]; then \
		echo "ℹ️ 没有检测到变更"; exit 0; \
	fi; \
	msg=$$(timeout 30 j ai --bypass -- "根据以下代码变更生成一个 commit message。格式：<类型>: <中文描述>，类型可选 feat/fix/refactor/docs/style/test/chore/perf，描述不超过 30 字。请用 <result>...</result> 包裹你的输出。变更: $$diff_stat" 2>/dev/null | $(J_AI_EXTRACT)); \
	if [ -z "$$msg" ]; then msg="更新: $$(date +'%Y-%m-%d %H:%M:%S')"; fi; \
	git add . && git commit -m "$$msg" && git push origin $(GIT_BRANCH); \
	echo "✅ 已推送: $$msg"

pull: current_dir ## 拉取最新代码
	@echo "📥 拉取最新代码..."
	@git pull origin $(GIT_BRANCH)
	@echo "☑️ 代码已更新"

status: current_dir ## 查看 Git 状态
	@git status

# ============================================
# 构建相关
# ============================================
build-remote: ## 构建 Remote 前端
	@echo "🌐 构建 Remote 前端..."
	@cd assets/remote && npm install --silent && npm run build && cp dist/remote.html ..
	@echo "☑️ Remote 前端构建完成"

build-web: ## 构建 Web 前端
	@echo "🌐 构建 Web 前端..."
	@cd web && npm install --silent && npm run build
	@echo "☑️ Web 前端构建完成"

build-indicator: ## 构建 j-indicator (macOS 点击光圈指示器)
	@echo "🔴 构建 j-indicator..."
	@mkdir -p $(TARGET_DIR)
	@swiftc helpers/indicator.swift -o $(TARGET_DIR)/j-indicator -O
	@echo "☑️ j-indicator 构建完成: $(TARGET_DIR)/j-indicator"

build-ax: ## 构建 j-ax (macOS Accessibility API helper)
	@echo "♿ 构建 j-ax..."
	@mkdir -p $(TARGET_DIR)
	@swiftc helpers/ax.swift -o $(TARGET_DIR)/j-ax -O -framework Cocoa -framework ApplicationServices
	@echo "☑️ j-ax 构建完成: $(TARGET_DIR)/j-ax"

# ============================================
# 构建相关（续）
# ============================================
release: ## 构建发布版本（release）
	@echo "🏗️  构建 release 版本..."
	@cargo build --release
	@echo "☑️ release 构建完成"

# ============================================
# 安装相关
# ============================================
install: ## 从 GitHub Releases 下载并安装到 /usr/local/bin
	@echo "📦 从 GitHub Releases 安装 j-cli..."
	@platform="darwin-arm64"; \
	version="v$(VERSION)"; \
	asset_name="j-$$platform"; \
	download_url="https://github.com/$(REPO)/releases/download/$$version/$$asset_name.tar.gz"; \
	echo "   版本: $$version  平台: $$platform"; \
	echo "   下载: $$download_url"; \
	tmp_dir=$$(mktemp -d); \
	trap "rm -rf $$tmp_dir" EXIT; \
	if ! curl -fsSL --progress-bar -H "User-Agent: j-cli-installer" -o "$$tmp_dir/j.tar.gz" "$$download_url"; then \
		echo "✖️ 下载失败，请检查版本号或网络连接"; exit 1; \
	fi; \
	echo "   正在解压..."; \
	tar -xzf "$$tmp_dir/j.tar.gz" -C "$$tmp_dir"; \
	if [ ! -d "$(INSTALL_DIR)" ]; then \
		echo "   创建安装目录 $(INSTALL_DIR)..."; \
		sudo mkdir -p "$(INSTALL_DIR)"; \
	fi; \
	if [ ! -w "$(INSTALL_DIR)" ]; then SUDO="sudo"; else SUDO=""; fi; \
	echo "   正在安装到 $(INSTALL_DIR)..."; \
	$$SUDO rm -f "$(INSTALL_DIR)/j"; \
	$$SUDO mv "$$tmp_dir/j" "$(INSTALL_DIR)/j"; \
	$$SUDO chmod +x "$(INSTALL_DIR)/j"; \
	for helper in j-indicator j-ax; do \
		if [ -f "$$tmp_dir/$$helper" ]; then \
			$$SUDO rm -f "$(INSTALL_DIR)/$$helper"; \
			$$SUDO mv "$$tmp_dir/$$helper" "$(INSTALL_DIR)/$$helper"; \
			$$SUDO chmod +x "$(INSTALL_DIR)/$$helper"; \
			echo "   ☑️ $$helper 已安装到 $(INSTALL_DIR)/$$helper"; \
		fi; \
	done; \
	if [ -x "$(INSTALL_DIR)/j" ]; then \
		echo "☑️ 安装成功！"; \
		echo "   安装位置: $(INSTALL_DIR)/j"; \
		echo "   版本: $$version"; \
	else \
		echo "✖️ 安装失败"; exit 1; \
	fi

uninstall: ## 卸载
	@echo "🗑️  卸载..."
	@if [ ! -w "$(INSTALL_DIR)" ]; then SUDO="sudo"; else SUDO=""; fi; \
	$$SUDO rm -f "$(INSTALL_DIR)/j" "$(INSTALL_DIR)/j-indicator" "$(INSTALL_DIR)/j-ax"; \
	echo "☑️ j 及 helpers 已从 $(INSTALL_DIR) 卸载"

# ============================================
# 发布相关
# ============================================
bump-version: ## 递增版本号（最后一位 patch）
	@echo "📌 递增版本号..."
	@current=$$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/'); \
	major=$$(echo $$current | cut -d. -f1); \
	minor=$$(echo $$current | cut -d. -f2); \
	patch=$$(echo $$current | cut -d. -f3); \
	new_patch=$$((patch + 1)); \
	new_version="$$major.$$minor.$$new_patch"; \
	echo "  当前版本: $$current"; \
	echo "  新版本: $$new_version"; \
	if [[ "$$OSTYPE" == "darwin"* ]]; then \
		sed -i '' "s/^version = \"$$current\"/version = \"$$new_version\"/" Cargo.toml; \
	else \
		sed -i "s/^version = \"$$current\"/version = \"$$new_version\"/" Cargo.toml; \
	fi; \
	echo "☑️ 版本号已更新为 $$new_version"

publish: ## 发布到 crates.io（NOTE='xxx' make publish 或从 CHANGELOG.md 读取）
	@echo "📦 开始发布流程..."
	@$(MAKE) bump-version
	@$(MAKE) release
	@git add .
	@version=$$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/'); \
	note_file=$$(mktemp); \
	changelog_tmp=$$(mktemp); \
	trap 'rm -f "$$note_file" "$$changelog_tmp"' EXIT; \
	if [ -n "$${NOTE:-}" ]; then \
		{ echo "# v$$version"; echo ""; printf '%s\n' "$$NOTE"; echo ""; } > "$$changelog_tmp"; \
		if [ -f CHANGELOG.md ]; then cat CHANGELOG.md >> "$$changelog_tmp"; fi; \
		mv "$$changelog_tmp" CHANGELOG.md; \
	fi; \
	{ echo "Release v$$version"; echo ""; awk 'NR==1{next} /^# v/{exit} {print}' CHANGELOG.md; } > "$$note_file"; \
	git add CHANGELOG.md; \
	git commit -m "chore: bump version to v$$version"; \
	git tag -a --cleanup=verbatim "v$$version" -F "$$note_file"; \
	git push origin $(GIT_BRANCH); \
	git push origin "v$$version"; \
	echo "📤 发布到 crates.io..."; \
	cargo publish --registry crates-io --allow-dirty; \
	echo "☑️ 已发布 v$$version! 验证: cargo search j-cli"

release-note: ## 预览 CHANGELOG.md 中最新版本的 release notes
	@awk '/^# v/{if(p++)exit}p' CHANGELOG.md | awk 'NR>1 || /^./'

publish-check: ## 发布前检查（dry-run）
	@echo "🔍 发布前检查（dry-run）..."
	@cargo publish --registry crates-io --dry-run
	@echo "☑️ 检查通过"

tag: ## 创建 git tag（基于当前版本号）
	@version=$(VERSION); \
	tag="v$$version"; \
	if git rev-parse "$$tag" >/dev/null 2>&1; then \
		echo "✖️ 标签 $$tag 已存在 (Cargo.toml 版本 = $$version)"; \
		echo "   请先使用 'make bump-version' 递增版本号"; \
		echo "   或使用 'make set-version V=x.x.x' 设置新版本号"; \
		exit 1; \
	fi; \
	echo "📌 创建标签 $$tag (来自 Cargo.toml)..."; \
	git tag -a "$$tag" -m "Release $$tag"; \
	git push origin "$$tag"; \
	echo "☑️ 标签 $$tag 已创建并推送。GitHub Actions 将自动构建和发布。"

set-version: ## 设置指定版本号（用法：make set-version V=1.2.3）
ifndef V
	@echo "✖️ 请指定版本号，例如: make set-version V=1.2.3"
	@exit 1
endif
	@echo "📌 设置版本号为 $(V)..."
	@current=$$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/'); \
	echo "  当前版本: $$current"; \
	echo "  新版本: $(V)"; \
	if [[ "$$OSTYPE" == "darwin"* ]]; then \
		sed -i '' "s/^version = \"$$current\"/version = \"$(V)\"/" Cargo.toml; \
	else \
		sed -i "s/^version = \"$$current\"/version = \"$(V)\"/" Cargo.toml; \
	fi; \
	echo "☑️ 版本号已更新为 $(V)"

tags: ## 查看最近的标签
	@echo "🏷️  最近的标签:"
	@git tag -l | sort -V | tail -10

# ============================================
# 测试相关
# ============================================
test: ## 运行测试
	@echo "🧪 运行测试..."
	@cargo test
	@echo "☑️ 测试完成"

test-all: ## 运行所有测试（包括集成测试）
	@echo "🧪 运行所有测试..."
	@cargo test --all-features
	@echo "☑️ 所有测试完成"

bench: ## 运行性能测试
	@echo "⚡ 运行性能测试..."
	@cargo bench
	@echo "☑️ 性能测试完成"

# ============================================
# 代码质量
# ============================================
fmt: ## 格式化代码
	@echo "🧹 格式化代码..."
	@cargo fmt
	@echo "☑️ 代码格式化完成"

lint: ## 运行 clippy 检查
	@echo "🔍 运行 clippy 检查..."
	@cargo clippy -- -D warnings
	@echo "☑️ clippy 检查完成"

check: ## 检查代码（不构建）
	@echo "🔍 检查代码..."
	@cargo check
	@echo "☑️ 代码检查完成"

check-lint: ## 运行完整合规性检查脚本
	@bash scripts/check_lint.sh

clippy: lint ## clippy 别名

pre-commit: fmt lint test ## 提交前检查
	@echo "☑️ 所有检查通过，可以提交"

# ============================================
# 清理相关
# ============================================
clean: ## 清理构建产物
	@echo "🧹 清理构建产物..."
	@cargo clean
	@echo "☑️ 清理完成"

# ============================================
# 运行相关
# ============================================
run: build-remote ## 运行项目
	@echo "🚀 运行项目..."
	@cargo run --features browser_cdp

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
	@echo "☑️ 覆盖率报告生成完成: tarpaulin-report.html"