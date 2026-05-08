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
        current_dir push push-non-ai pull status \
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

# --- j ai 输出提取辅助函数 ---
# prompt 中要求 AI 用 <result>...</result> 包裹输出
# 管道中直接用 awk 抓取标签内容，无需过滤任何噪音
# 支持单行 <result>xxx</result> 和多行 <result>\n...\n</result>
define J_AI_EXTRACT
awk '/<result>/{in_r=1;gsub(/.*<result>/,"")}/<\/result>/{gsub(/<\/result>.*/,"");in_r=0;print;next}in_r{print}'
endef

push: current_dir fmt build-web ## AI 生成 commit message 并推送
	@echo "🤖 AI 生成变更说明..."
	@diff_stat="$$(git diff --stat 2>/dev/null)"; \
	if [ -z "$$diff_stat" ]; then \
		diff_stat="$$(git diff --cached --stat 2>/dev/null)"; \
	fi; \
	if [ -z "$$diff_stat" ]; then \
		echo "ℹ️ 没有检测到变更"; exit 0; \
	fi; \
	prompt_file=$$(mktemp); \
	trap 'rm -f "$$prompt_file"' EXIT; \
	{ echo "你是一个自动化的 commit message 生成器。根据代码变更生成一个 commit message。"; \
	  echo ""; \
	  echo "## 输出要求"; \
	  echo "- 格式：<类型>: <中文描述>，类型可选 feat/fix/refactor/docs/style/test/chore/perf"; \
	  echo "- 描述不超过 30 字"; \
	  echo "- 用 <result>...</result> 包裹你的输出，不要输出任何其他内容"; \
	  echo ""; \
	  echo "## 行为规则（必须遵守）"; \
	  echo "- 不要向用户提问，不要要求确认，直接根据已有信息生成最佳结果"; \
	  echo "- 如果提供的上下文不完整，主动执行 shell 命令补充信息，而不是停下来"; \
	  echo "- 如果变更太多无法归类，选择最主要的变更类型概括"; \
	  echo ""; \
	  echo "## 提供的上下文"; \
	  echo "变更概览:"; \
	  echo "$$diff_stat"; \
	  echo ""; \
	  echo "详细变更（截断）:"; \
	  (git diff 2>/dev/null || git diff --cached 2>/dev/null) | head -200; \
	  echo ""; \
	  echo "## 补充上下文（按需执行）"; \
	  echo "以上信息可能被截断。你可以执行 shell 命令获取更多信息："; \
	  echo "- git diff / git diff --cached（查看完整变更）"; \
	  echo "- git diff -- <file>（查看特定文件）"; \
	  echo "- git status（查看工作区状态）"; \
	} > "$$prompt_file"; \
	ai_out=$$(mktemp); \
	j ai --bypass --no-render -- "$$(cat "$$prompt_file")" > "$$ai_out" 2>/dev/null; \
	echo ""; \
	echo "📄 AI 原始输出:"; \
	echo "----------------------------------------"; \
	cat "$$ai_out"; \
	echo "----------------------------------------"; \
	msg=$$(awk '/<result>/{in_r=1;gsub(/.*<result>/,"")}/<\/result>/{gsub(/<\/result>.*/,"");in_r=0;print;next}in_r{print}' "$$ai_out"); \
	rm -f "$$ai_out" "$$prompt_file"; \
	if [ -z "$$msg" ]; then msg="更新: $$(date +'%Y-%m-%d %H:%M:%S')"; fi; \
	git add . && git commit -m "$$msg" && git push origin $(GIT_BRANCH); \
	echo "✅ 已推送: $$msg"

push-non-ai: current_dir fmt build-web ## 提交并推送代码（手动 commit message）
	@echo "📤 推送代码到远程仓库..."
	@git add .\
	&& (git commit -m "更新: $(shell date +'%Y-%m-%d %H:%M:%S')" || exit 0) \
	&& git push origin $(GIT_BRANCH)
	@echo "☑️ 代码已推送"

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
release: ## 构建发布版本（release, INSTALL_SOURCE=github）
	@echo "🏗️  构建 release 版本..."
	@INSTALL_SOURCE=github cargo build --release
	@echo "☑️ release 构建完成"

# ============================================
# 安装相关
# ============================================
install: ## 从本地 cargo build --release 安装到 /usr/local/bin（与 GitHub 安装路径一致）
	@echo "📦 从本地构建安装 j-cli..."
	@$(MAKE) release
	@if [ ! -d "$(INSTALL_DIR)" ]; then \
		echo "   创建安装目录 $(INSTALL_DIR)..."; \
		sudo mkdir -p "$(INSTALL_DIR)"; \
	fi; \
	if [ ! -w "$(INSTALL_DIR)" ]; then SUDO="sudo"; else SUDO=""; fi; \
	echo "   正在安装到 $(INSTALL_DIR)..."; \
	$$SUDO rm -f "$(INSTALL_DIR)/j"; \
	$$SUDO cp "$(TARGET_DIR)/j" "$(INSTALL_DIR)/j"; \
	$$SUDO chmod +x "$(INSTALL_DIR)/j"; \
	for helper in j-indicator j-ax; do \
		if [ -f "$(TARGET_DIR)/$$helper" ]; then \
			$$SUDO rm -f "$(INSTALL_DIR)/$$helper"; \
			$$SUDO cp "$(TARGET_DIR)/$$helper" "$(INSTALL_DIR)/$$helper"; \
			$$SUDO chmod +x "$(INSTALL_DIR)/$$helper"; \
			echo "   ☑️ $$helper 已安装到 $(INSTALL_DIR)/$$helper"; \
		fi; \
	done; \
	version=$$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/'); \
	if [ -x "$(INSTALL_DIR)/j" ]; then \
		echo "☑️ 安装成功！"; \
		echo "   安装位置: $(INSTALL_DIR)/j"; \
		echo "   版本: v$$version (本地构建)"; \
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

publish: ## 发布到 crates.io（NOTE='xxx' make publish 或 AI 自动生成）
	@echo "📦 开始发布流程..."
	@$(MAKE) fmt
	@$(MAKE) bump-version
	@$(MAKE) release
	@git add .
	@version=$$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/'); \
	note_file=$$(mktemp); \
	changelog_tmp=$$(mktemp); \
	prompt_file=$$(mktemp); \
	trap 'rm -f "$$note_file" "$$changelog_tmp" "$$prompt_file"' EXIT; \
	current_tag="v$$version"; \
	changelog_top=$$(awk '/^# v/{print; exit}' CHANGELOG.md 2>/dev/null); \
	if [ -n "$${NOTE:-}" ]; then \
		echo "📝 使用手动指定的 release notes..."; \
		{ echo "# v$$version"; echo ""; printf '%s\n' "$$NOTE"; echo ""; } > "$$changelog_tmp"; \
		if [ -f CHANGELOG.md ]; then cat CHANGELOG.md >> "$$changelog_tmp"; fi; \
		mv "$$changelog_tmp" CHANGELOG.md; \
	elif [ "$$changelog_top" = "# $$current_tag" ]; then \
		echo "📝 使用 CHANGELOG.md 中已有的 release notes..."; \
	else \
		echo "🤖 AI 生成 release notes..."; \
		last_tag=$$(git describe --tags --abbrev=0 2>/dev/null || echo ""); \
		if [ -n "$$last_tag" ]; then \
			log_range="$$last_tag..HEAD"; \
		else \
			log_range="HEAD~10..HEAD"; \
		fi; \
		{ echo "你是一个自动化的 release notes 生成器。根据 git 历史生成 release notes。"; \
		  echo ""; \
		  echo "## 输出要求"; \
		  echo "- 第一行不要标题，直接从分类开始"; \
		  echo "- 使用 Markdown 格式，分类用三级标题如 ### 新功能、### 改进、### Bug 修复"; \
		  echo "- 每个条目格式：- **功能名**: 描述"; \
		  echo "- 只包含有意义的变更，忽略 minor 重构"; \
		  echo "- 用 <result>...</result> 包裹你的输出，不要输出任何其他内容"; \
		  echo ""; \
		  echo "## 行为规则（必须遵守）"; \
		  echo "- 不要向用户提问，不要要求确认，直接根据已有信息生成最佳结果"; \
		  echo "- 如果提供的 git log 为空或 HEAD 与上个 tag 相同，说明这是 bump-version 后的自动提交"; \
		  echo "  此时应该回退到更早的 tag（如倒数第二个 tag）来获取变更范围，或者用 HEAD~10..HEAD"; \
		  echo "- 如果 commit message 不够详细，主动执行 shell 命令查看 diff，而不是停下来"; \
		  echo "- 如果实在无法获取足够的变更信息，基于仅有的信息生成简洁的 release notes，宁可简短也不要提问"; \
		  echo ""; \
		  echo "## 提供的上下文"; \
		  echo "当前版本: $$version"; \
		  echo "上一个 tag: $${last_tag:-无}"; \
		  echo "Log 范围: $$log_range"; \
		  echo ""; \
		  echo "Git log ($$log_range):"; \
		  git log $$log_range --oneline --no-decorate 2>/dev/null | head -20; \
		  echo ""; \
		  echo "## 补充上下文（按需执行）"; \
		  echo "你可以执行 shell 命令获取更多信息："; \
		  echo "- git log $$log_range --stat（查看每次提交涉及的文件）"; \
		  echo "- git log $$log_range -p（查看每次提交的完整 diff）"; \
		  echo "- git show <commit>（查看某次提交的详细变更）"; \
		  echo "- git diff $${last_tag:-HEAD~10}..HEAD（查看与上个标签之间的完整差异）"; \
		  echo "- git tag -l | sort -V | tail -5（查看最近的标签列表）"; \
		  echo "- git log $$last_tag~1..$$last_tag --oneline（回退查看上个 tag 的变更）"; \
		} > "$$prompt_file"; \
		ai_out=$$(mktemp); \
	j ai --bypass --no-render -- "$$(cat "$$prompt_file")" 2>/dev/null | tee "$$ai_out"; \
	echo ""; \
	echo "📄 AI 原始输出:"; \
	echo "----------------------------------------"; \
	cat "$$ai_out"; \
	echo "----------------------------------------"; \
	ai_note=$$(awk '/<result>/{in_r=1;gsub(/.*<result>/,"")}/<\/result>/{gsub(/<\/result>.*/,"");in_r=0;print;next}in_r{print}' "$$ai_out"); \
	rm -f "$$ai_out"; \
		if [ -z "$$ai_note" ]; then \
			echo "⚠️ AI 生成失败，请手动指定 NOTE 参数"; \
			exit 1; \
		fi; \
		{ echo "# v$$version"; echo ""; echo "$$ai_note"; echo ""; } > "$$changelog_tmp"; \
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