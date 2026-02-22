SHELL := /bin/bash

.PHONY: current_dir
current_dir:
	@echo "🔍 Current directory:"
	@echo "======================================"
	@echo "Current directory: $$(pwd)"
	@echo "======================================="

.PHONY: push
push: current_dir fmt
	@git add .\
	&& (git commit -m "新增了一些特性" || exit 0) \
	&& git push origin main

.PHONY: pull
pull: current_dir
	@git pull origin main

.PHONY: status
status: current_dir
	@git status

.PHONY: release
release: current_dir md_render
	@cargo build --release

.PHONY: install
install: release
	@cp target/release/j /usr/local/bin/j
	@echo "✅ j installed to /usr/local/bin/j"

.PHONY: uninstall
uninstall:
	@rm -f /usr/local/bin/j
	@echo "✅ j uninstalled"

# 发布到 crates.io
.PHONY: publish
publish: release push
	@echo "📦 Publishing to crates.io..."
	@cargo publish --registry crates-io
	@echo "✅ Published! Verify: cargo search j-cli"

# 发布前检查（dry-run）
.PHONY: publish-check
publish-check:
	@echo "🔍 Checking publish (dry-run)..."
	@cargo publish --registry crates-io --dry-run
	@echo "✅ Check passed"

# 创建 git tag 并推送（触发 GitHub Actions 自动构建发布）
.PHONY: tag
tag:
	@echo "📌 Creating git tag..."
	@read -p "Enter version (e.g., v1.0.0): " version; \
	if [ -z "$$version" ]; then \
		echo "❌ Version is required"; \
		exit 1; \
	fi; \
	git tag -a "$$version" -m "Release $$version"; \
	git push origin "$$version"; \
	echo "✅ Tag $$version created and pushed. GitHub Actions will build and release automatically."

# 本地测试安装脚本
.PHONY: test-install
test-install:
	@echo "🧪 Testing install script locally..."
	@./install.sh

# 查看远程 tag
.PHONY: tags
tags:
	@git tag -l | sort -V | tail -5

.PHONY: md_render
md_render:
	@echo "🔄 Building md_render..."
	@cd plugin/md_render/code \
	&& GOOS=darwin GOARCH=arm64 go build -o ../bin/md_render-darwin-arm64
	@echo "✅ md_render built to plugin/md_render/bin/md_render-darwin-arm64"

.PHONY: fmt
fmt:
	@echo "🧹 Formatting code..."
	@cargo fmt