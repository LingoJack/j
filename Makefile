SHELL := /bin/bash

# ============================================
# 变量定义
# ============================================
INSTALL_DIR := /usr/local/bin
REPO := LingoJack/jcli
TARGET_DIR := target/release
JSTUDIO_DIR := apps/jstudio
JSTUDIO_TAURI_DIR := $(JSTUDIO_DIR)/src-tauri
JSTUDIO_MACOS_APP := $(JSTUDIO_TAURI_DIR)/target/release/bundle/macos/jstudio.app
JSTUDIO_BIN := $(JSTUDIO_TAURI_DIR)/target/release/jstudio
JSTUDIO_INSTALL_APP_DIR ?= /Applications
VERSION := $(shell grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
J_AGENT_VERSION := $(shell grep '^version' j-agent/Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
GIT_BRANCH := $(shell git rev-parse --abbrev-ref HEAD)

# ============================================
# 伪目标声明
# ============================================
.PHONY: help \
        current_dir push push-non-ai commit pull status \
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
        init-jstudio dev-jstudio build-jstudio install-jstudio uninstall-jstudio clean-jstudio \
        status-jstudio commit-jstudio push-jstudio push-jstudio-non-ai \
        gui-dev gui-build gui-install gui-clean \
        ppt-serve ppt-stop ppt-build ppt-render ppt-deps ppt-clean

# ============================================
# 帮助信息
# ============================================
help: ## 显示此帮助信息
	@echo "📚 j-cli Makefile 帮助"
	@echo "============================================"
	@echo "版本: $(VERSION) | j-agent: $(J_AGENT_VERSION) | 分支: $(GIT_BRANCH)"
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
		echo "ℹ️ 没有检测到本地变更，直接 push 已有 commits..."; \
		git push origin $(GIT_BRANCH); \
		echo "✅ 已 push"; \
		exit 0; \
	fi; \
	prompt_file=$$(mktemp); \
	stat_file=$$(mktemp); \
	diff_file=$$(mktemp); \
	trap 'rm -f "$$prompt_file" "$$stat_file" "$$diff_file"' EXIT; \
	printf '%s\n' "$$diff_stat" > "$$stat_file"; \
	(git diff 2>/dev/null || git diff --cached 2>/dev/null) | head -200 > "$$diff_file"; \
	awk -v stat_file="$$stat_file" -v diff_file="$$diff_file" '\
		/\{\{diff_stat\}\}/ { while ((getline l < stat_file) > 0) print l; close(stat_file); next } \
		/\{\{diff\}\}/      { while ((getline l < diff_file) > 0) print l; close(diff_file); next } \
		{ print }' prompts/commit-message.md > "$$prompt_file"; \
	ai_out=$$(mktemp); \
	j ai --bypass --no-render -- "$$(cat "$$prompt_file")" > "$$ai_out" 2>/dev/null; \
	echo ""; \
	echo "📄 AI 原始输出:"; \
	echo "----------------------------------------"; \
	cat "$$ai_out"; \
	echo "----------------------------------------"; \
	msg=$$(perl -0777 -pe 's/<\s*\/\s*result\s*>/<\/result>/g' "$$ai_out" | awk '/<result>/{in_r=1;gsub(/.*<result>/,"")}/<\/result>/{gsub(/<\/result>.*/,"");in_r=0;print;next}in_r{print}'); \
	rm -f "$$ai_out"; \
	if [ -z "$$msg" ]; then msg="更新: $$(date +'%Y-%m-%d %H:%M:%S')"; fi; \
	git add . && git commit -m "$$msg" && git push origin $(GIT_BRANCH); \
	echo "✅ 已推送: $$msg"

push-non-ai: current_dir fmt build-web ## 提交并推送代码（手动 commit message）
	@echo "📤 推送代码到远程仓库..."
	@git add .\
	&& (git commit -m "更新: $(shell date +'%Y-%m-%d %H:%M:%S')" || exit 0) \
	&& git push origin $(GIT_BRANCH)
	@echo "☑️ 代码已推送"

commit: current_dir fmt build-web ## 自动提交（基于变更生成 message，不调用 AI）
	@echo "📝 自动生成 commit message..."
	@git add .; \
	staged_files=$$(git diff --cached --name-only 2>/dev/null); \
	if [ -z "$$staged_files" ]; then \
		echo "ℹ️ 没有检测到变更，无需提交"; \
		exit 0; \
	fi; \
	file_count=$$(echo "$$staged_files" | wc -l | tr -d ' '); \
	if [ "$$file_count" -eq 1 ]; then \
		msg="update: $$(echo "$$staged_files" | head -1)"; \
	else \
		first=$$(echo "$$staged_files" | head -1); \
		msg="update: $$first and $$((file_count - 1)) other file(s)"; \
	fi; \
	git commit -m "$$msg"; \
	echo "✅ 已提交: $$msg"

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
	@cd assets/remote && npm install --silent && npm run build
	@echo "☑️ Remote 前端构建完成（输出至 assets/remote/dist/remote.html）"

build-web: ## 构建 Web 前端
	@echo "🌐 构建 Web 前端..."
	@cd web && npm install --silent && npm run build
	@echo "☑️ Web 前端构建完成"

# ============================================
# jstudio Tauri App
# ============================================
init-jstudio: ## 初始化 jstudio submodule 并安装前端依赖
	@echo "📖 初始化 jstudio..."
	@git submodule update --init --recursive $(JSTUDIO_DIR)
	@cd $(JSTUDIO_DIR) && npm install --silent
	@echo "☑️ jstudio 依赖已就绪"

dev-jstudio: init-jstudio ## 启动 jstudio Tauri 开发模式
	@echo "📖 启动 jstudio 开发模式..."
	@cd $(JSTUDIO_DIR) && npm run tauri:dev

build-jstudio: init-jstudio ## 构建 jstudio Tauri 应用
	@echo "📖 构建 jstudio Tauri 应用..."
	@cd $(JSTUDIO_DIR) && npm run tauri:build -- --bundles app
	@echo "☑️ jstudio 构建完成"
	@if [ -d "$(JSTUDIO_MACOS_APP)" ]; then \
		echo "   macOS App: $(JSTUDIO_MACOS_APP)"; \
	elif [ -x "$(JSTUDIO_BIN)" ]; then \
		echo "   Binary: $(JSTUDIO_BIN)"; \
	fi

install-jstudio: build-jstudio ## 安装 jstudio app（macOS 安装到 /Applications，其他系统提示二进制路径）
	@echo "📦 安装 jstudio..."
	@if [ "$$(uname -s)" = "Darwin" ]; then \
		if [ ! -d "$(JSTUDIO_MACOS_APP)" ]; then \
			echo "✖️ 未找到 $(JSTUDIO_MACOS_APP)"; exit 1; \
		fi; \
		if [ ! -w "$(JSTUDIO_INSTALL_APP_DIR)" ]; then SUDO="sudo"; else SUDO=""; fi; \
		echo "   正在安装到 $(JSTUDIO_INSTALL_APP_DIR)/jstudio.app..."; \
		$$SUDO rm -rf "$(JSTUDIO_INSTALL_APP_DIR)/jstudio.app"; \
		$$SUDO cp -R "$(JSTUDIO_MACOS_APP)" "$(JSTUDIO_INSTALL_APP_DIR)/jstudio.app"; \
		echo "☑️ jstudio 已安装: $(JSTUDIO_INSTALL_APP_DIR)/jstudio.app"; \
		echo "   现在可以使用: j read <file_or_dir_path>"; \
	else \
		if [ -x "$(JSTUDIO_BIN)" ]; then \
			echo "☑️ jstudio 已构建: $(JSTUDIO_BIN)"; \
			echo "   请设置环境变量后使用:"; \
			echo "   export JSTUDIO_BIN=$$(pwd)/$(JSTUDIO_BIN)"; \
			echo "   j read <file_or_dir_path>"; \
		else \
			echo "✖️ 未找到 jstudio 二进制: $(JSTUDIO_BIN)"; exit 1; \
		fi; \
	fi

status-jstudio: ## 查看 jstudio submodule 状态
	@echo "🔍 jstudio submodule 状态:"
	@git -C $(JSTUDIO_DIR) status --short --branch --untracked-files=all

commit-jstudio: ## 提交 jstudio submodule 代码（非 AI，自动生成 message）
	@echo "📝 自动提交 jstudio submodule 代码..."
	@cd $(JSTUDIO_DIR) && \
		git add . && \
		staged_files=$$(git diff --cached --name-only 2>/dev/null); \
		if [ -z "$$staged_files" ]; then \
			echo "ℹ️ jstudio 没有待提交变更"; \
			exit 0; \
		fi; \
		file_count=$$(echo "$$staged_files" | wc -l | tr -d ' '); \
		if [ "$$file_count" -eq 1 ]; then \
			msg="update: $$(echo "$$staged_files" | head -1)"; \
		else \
			first=$$(echo "$$staged_files" | head -1); \
			msg="update: $$first and $$((file_count - 1)) other file(s)"; \
		fi; \
		git commit -m "$$msg"; \
		echo "☑️ jstudio 已提交: $$msg"
	@echo "🔗 更新主仓库 submodule 指针..."
	@git add $(JSTUDIO_DIR)
	@echo "☑️ 已暂存主仓库 submodule 指针；请在主仓库提交该指针变更"

push-jstudio: ## AI 生成 commit message 并推送 jstudio submodule
	@echo "🤖 AI 生成 jstudio 变更说明..."
	@cd $(JSTUDIO_DIR) && \
		diff_stat="$$(git diff --stat 2>/dev/null)"; \
		if [ -z "$$diff_stat" ]; then \
			diff_stat="$$(git diff --cached --stat 2>/dev/null)"; \
		fi; \
		if [ -z "$$diff_stat" ]; then \
			echo "ℹ️ jstudio 没有检测到本地变更，直接 push 已有 commits..."; \
			git push origin HEAD; \
			exit 0; \
		fi; \
		prompt_file=$$(mktemp); \
		stat_file=$$(mktemp); \
		diff_file=$$(mktemp); \
		trap 'rm -f "$$prompt_file" "$$stat_file" "$$diff_file"' EXIT; \
		printf '%s\n' "$$diff_stat" > "$$stat_file"; \
		git diff 2>/dev/null | head -200 > "$$diff_file"; \
		awk -v stat_file="$$stat_file" -v diff_file="$$diff_file" ' \
			/\{\{diff_stat\}\}/ { while ((getline l < stat_file) > 0) print l; close(stat_file); next } \
			/\{\{diff\}\}/      { while ((getline l < diff_file) > 0) print l; close(diff_file); next } \
			{ print }' ../../prompts/commit-message.md > "$$prompt_file"; \
		ai_out=$$(mktemp); \
		j ai --bypass --no-render -- "$$(cat "$$prompt_file")" > "$$ai_out" 2>/dev/null; \
		echo ""; \
		echo "📄 AI 原始输出:"; \
		echo "----------------------------------------"; \
		cat "$$ai_out"; \
		echo "----------------------------------------"; \
		msg=$$(perl -0777 -pe 's/<\s*\/\s*result\s*>/<\/result>/g' "$$ai_out" | awk '/<result>/{in_r=1;gsub(/.*<result>/,"")}/<\/result>/{gsub(/<\/result>.*/,"");in_r=0;print;next}in_r{print}'); \
		rm -f "$$ai_out"; \
		if [ -z "$$msg" ]; then msg="更新 jstudio: $$(date +'%Y-%m-%d %H:%M:%S')"; fi; \
		git add . && git commit -m "$$msg" && git push origin HEAD; \
		echo "✅ jstudio 已推送: $$msg"
	@echo "🔗 更新主仓库 submodule 指针..."
	@git add $(JSTUDIO_DIR)
	@echo "☑️ 已暂存主仓库 submodule 指针；请在主仓库提交该指针变更"

push-jstudio-non-ai: ## 提交并推送 jstudio submodule 代码（非 AI，MSG='message' 可指定提交信息）
	@echo "📤 非 AI 推送 jstudio submodule..."
	@cd $(JSTUDIO_DIR) && \
		git add . && \
		if git diff --cached --quiet; then \
			echo "ℹ️ jstudio 没有待提交变更，直接 push 已有 commits..."; \
			git push origin HEAD; \
		else \
			msg="$${MSG:-更新 jstudio: $$(date +'%Y-%m-%d %H:%M:%S')}"; \
			git commit -m "$$msg" && git push origin HEAD; \
			echo "☑️ jstudio 已推送: $$msg"; \
		fi
	@echo "🔗 更新主仓库 submodule 指针..."
	@git add $(JSTUDIO_DIR)
	@echo "☑️ 已暂存主仓库 submodule 指针；请在主仓库提交该指针变更"

uninstall-jstudio: ## 卸载 jstudio app（macOS）
	@echo "🗑️  卸载 jstudio..."
	@if [ "$$(uname -s)" = "Darwin" ]; then \
		if [ ! -w "$(JSTUDIO_INSTALL_APP_DIR)" ]; then SUDO="sudo"; else SUDO=""; fi; \
		$$SUDO rm -rf "$(JSTUDIO_INSTALL_APP_DIR)/jstudio.app"; \
		echo "☑️ 已卸载 $(JSTUDIO_INSTALL_APP_DIR)/jstudio.app"; \
	else \
		echo "ℹ️ 非 macOS 平台请删除自定义安装位置，并取消 JSTUDIO_BIN 环境变量。"; \
	fi

clean-jstudio: ## 清理 jstudio 构建产物
	@echo "🧹 清理 jstudio 构建产物..."
	@rm -rf $(JSTUDIO_DIR)/dist $(JSTUDIO_TAURI_DIR)/target
	@echo "☑️ jstudio 构建产物已清理"

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
bump-version: ## 递增版本号（最后一位 patch，同步 j-agent 和安装脚本）
	@echo "📌 递增版本号..."
	@cli_ver=$$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/'); \
	major=$$(echo $$cli_ver | cut -d. -f1); \
	minor=$$(echo $$cli_ver | cut -d. -f2); \
	patch=$$(echo $$cli_ver | cut -d. -f3); \
	new_patch=$$((patch + 1)); \
	new_version="$$major.$$minor.$$new_patch"; \
	agent_ver=$$(grep '^version' j-agent/Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/'); \
	echo "  j-cli: $$cli_ver → $$new_version"; \
	echo "  j-agent: $$agent_ver → $$new_version"; \
	if [[ "$$OSTYPE" == "darwin"* ]]; then \
		sed -i '' "s/^version = \"$$cli_ver\"/version = \"$$new_version\"/" Cargo.toml; \
		sed -i '' "s/^version = \"$$agent_ver\"/version = \"$$new_version\"/" j-agent/Cargo.toml; \
		sed -i '' "s/\(j-agent.*version = \"\)[^\"]*\"/\1$$new_version\"/" Cargo.toml; \
		sed -i '' "s/DEFAULT_VERSION=\"v[^\"]*\"/DEFAULT_VERSION=\"v$$new_version\"/" install.sh; \
		sed -i '' 's/\$$DefaultVersion = "v[^"]*"/\$$DefaultVersion = "v'"$$new_version"'"/' install.ps1; \
	else \
		sed -i "s/^version = \"$$cli_ver\"/version = \"$$new_version\"/" Cargo.toml; \
		sed -i "s/^version = \"$$agent_ver\"/version = \"$$new_version\"/" j-agent/Cargo.toml; \
		sed -i "s/\(j-agent.*version = \"\)[^\"]*\"/\1$$new_version\"/" Cargo.toml; \
		sed -i "s/DEFAULT_VERSION=\"v[^\"]*\"/DEFAULT_VERSION=\"v$$new_version\"/" install.sh; \
		sed -i 's/\$$DefaultVersion = "v[^"]*"/\$$DefaultVersion = "v'"$$new_version"'"/' install.ps1; \
	fi; \
	echo "☑️ j-cli、j-agent 和安装脚本版本号已更新为 $$new_version"

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
		prompt_file=$$(mktemp); \
		log_file=$$(mktemp); \
		trap 'rm -f "$$prompt_file" "$$log_file"' EXIT; \
		git log $$log_range --oneline --no-decorate 2>/dev/null | head -20 > "$$log_file"; \
		awk -v version="$$version" \
		    -v last_tag="$${last_tag:-HEAD~10}" \
		    -v log_range="$$log_range" \
		    -v log_file="$$log_file" '\
			{ \
				gsub(/\{\{version\}\}/, version); \
				gsub(/\{\{last_tag\}\}/, last_tag); \
				gsub(/\{\{log_range\}\}/, log_range); \
				if (/\{\{git_log\}\}/) { while ((getline l < log_file) > 0) print l; close(log_file); next } \
				print \
			}' prompts/release-notes.md > "$$prompt_file"; \
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
	echo "📤 发布 j-agent 到 crates.io..."; \
	cd j-agent && cargo publish --registry crates-io --allow-dirty && cd ..; \
	echo "📤 发布 j-cli 到 crates.io..."; \
	cargo publish --registry crates-io --allow-dirty; \
	echo "☑️ 已发布 v$$version! 验证: cargo search j-cli"

release-note: ## 预览 CHANGELOG.md 中最新版本的 release notes
	@awk '/^# v/{if(p++)exit}p' CHANGELOG.md | awk 'NR>1 || /^./'

publish-check: ## 发布前检查（dry-run）
	@echo "🔍 发布前检查（dry-run）..."
	@echo "📦 检查 j-agent..."
	@cd j-agent && cargo publish --registry crates-io --dry-run && cd ..
	@echo "📦 检查 j-cli..."
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

set-version: ## 设置指定版本号（用法：make set-version V=1.2.3，同步 j-agent 和安装脚本）
ifndef V
	@echo "✖️ 请指定版本号，例如: make set-version V=1.2.3"
	@exit 1
endif
	@echo "📌 设置版本号为 $(V)..."
	@cli_ver=$$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/'); \
	agent_ver=$$(grep '^version' j-agent/Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/'); \
	echo "  j-cli: $$cli_ver → $(V)"; \
	echo "  j-agent: $$agent_ver → $(V)"; \
	if [[ "$$OSTYPE" == "darwin"* ]]; then \
		sed -i '' "s/^version = \"$$cli_ver\"/version = \"$(V)\"/" Cargo.toml; \
		sed -i '' "s/^version = \"$$agent_ver\"/version = \"$(V)\"/" j-agent/Cargo.toml; \
		sed -i '' "s/\(j-agent.*version = \"\)[^\"]*\"/\1$(V)\"/" Cargo.toml; \
		sed -i '' "s/DEFAULT_VERSION=\"v[^\"]*\"/DEFAULT_VERSION=\"v$(V)\"/" install.sh; \
		sed -i '' 's/\$$DefaultVersion = "v[^"]*"/\$$DefaultVersion = "v$(V)"/' install.ps1; \
	else \
		sed -i "s/^version = \"$$cli_ver\"/version = \"$(V)\"/" Cargo.toml; \
		sed -i "s/^version = \"$$agent_ver\"/version = \"$(V)\"/" j-agent/Cargo.toml; \
		sed -i "s/\(j-agent.*version = \"\)[^\"]*\"/\1$(V)\"/" Cargo.toml; \
		sed -i "s/DEFAULT_VERSION=\"v[^\"]*\"/DEFAULT_VERSION=\"v$(V)\"/" install.sh; \
		sed -i 's/\$$DefaultVersion = "v[^"]*"/\$$DefaultVersion = "v$(V)"/' install.ps1; \
	fi; \
	echo "☑️ j-cli、j-agent 和安装脚本版本号已更新为 $(V)"

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
	@echo "☑️ 覆盖率报告生成完成: tarpaulin-report.html"️ 覆盖率报告生成完成: tarpaulin-report.html"

# ============================================
# PPT 演示
# ============================================
PPT_PORT := 8765
PPT_PATH := docs/thesis-ppt/

ppt-serve: ## 启动毕业设计 PPT 本地服务（http://localhost:$(PPT_PORT)/$(PPT_PATH)）
	@echo "🎤 启动 PPT 服务..."
	@if lsof -ti:$(PPT_PORT) >/dev/null 2>&1; then \
		echo "ℹ️ 端口 $(PPT_PORT) 已被占用，先停止旧服务..."; \
		lsof -ti:$(PPT_PORT) | xargs kill -9 2>/dev/null || true; \
		sleep 0.5; \
	fi
	@cd $(CURDIR) && python3 -m http.server $(PPT_PORT) >/dev/null 2>&1 &
	@sleep 1
	@url="http://localhost:$(PPT_PORT)/$(PPT_PATH)"; \
	echo "☑️ PPT 已启动: $$url"; \
	echo "   按键: ← → 翻页 · S 演讲者视图 · T 切换主题 · F 全屏"; \
	echo "   停止: make ppt-stop"; \
	open "$$url"

ppt-stop: ## 停止 PPT 本地服务
	@echo "🛑 停止 PPT 服务..."
	@if lsof -ti:$(PPT_PORT) >/dev/null 2>&1; then \
		lsof -ti:$(PPT_PORT) | xargs kill -9 2>/dev/null || true; \
		echo "☑️ 已停止端口 $(PPT_PORT) 上的服务"; \
	else \
		echo "ℹ️ 端口 $(PPT_PORT) 未在运行"; \
	fi

# ============================================
# PPT 一键导出 .pptx（图片版 · 与 HTML 视觉 100% 一致 + 内嵌逐字稿）
# ============================================
PPT_DECK_DIR  := presentation/ppt
PPT_SCRIPT_DIR := $(PPT_DECK_DIR)/scripts
PPT_VENV      := $(PPT_SCRIPT_DIR)/.venv
PPT_PY        := $(PPT_VENV)/bin/python3
PPT_OUT       := $(PPT_DECK_DIR)/jcli-thesis.pptx

ppt-deps: ## 安装 PPT 导出依赖（puppeteer + python-pptx，幂等）
	@echo "📦 检查 PPT 导出依赖..."
	@if [ ! -d "$(PPT_SCRIPT_DIR)/node_modules/puppeteer" ]; then \
		echo "  安装 puppeteer..."; \
		cd $(PPT_SCRIPT_DIR) && npm install --silent; \
	else \
		echo "  ✓ puppeteer 已安装"; \
	fi
	@if [ ! -x "$(PPT_PY)" ]; then \
		echo "  创建 venv..."; \
		python3 -m venv $(PPT_VENV); \
	fi
	@if ! $(PPT_PY) -c "import pptx, bs4" 2>/dev/null; then \
		echo "  安装 python-pptx + beautifulsoup4..."; \
		$(PPT_VENV)/bin/pip install --quiet python-pptx beautifulsoup4 lxml Pillow; \
	else \
		echo "  ✓ python-pptx 已安装"; \
	fi
	@echo "☑️ 依赖就绪"

ppt-render: ppt-deps ## 渲染 HTML 每页为高清 PNG（2560×1440）
	@echo "🖼️ 渲染 HTML → PNG..."
	@cd $(PPT_SCRIPT_DIR) && node render-png.mjs

ppt-build: ppt-render ## 一键导出 .pptx（图片版 + 逐字稿）
	@echo "📑 打包 PNG → pptx..."
	@$(PPT_PY) $(PPT_SCRIPT_DIR)/png-to-pptx.py
	@echo ""
	@echo "✅ 完成！文件: $(PPT_OUT)"
	@du -h $(PPT_OUT) | awk '{print "   大小: "$$1}'
	@open $(PPT_OUT) 2>/dev/null || true

ppt-clean: ## 清理 PPT 导出产物
	@echo "🧹 清理 PPT 导出产物..."
	@rm -rf $(PPT_DECK_DIR)/ppt-png
	@rm -f $(PPT_OUT)
	@echo "☑️ 已清理 ppt-png/ 与 $(notdir $(PPT_OUT))"