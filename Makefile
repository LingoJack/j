SHELL := /bin/bash

.PHONY: current_dir
current_dir:
	@echo "Current directory: $$(pwd)"

.PHONY: push
push: current_dir
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
release: current_dir
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
publish: release
	@echo "📦 Publishing to crates.io..."
	@cargo publish
	@echo "✅ Published! Verify: cargo search j-cli"

# 发布前检查（dry-run）
.PHONY: publish-check
publish-check:
	@echo "🔍 Checking publish (dry-run)..."
	@cargo publish --dry-run
	@echo "✅ Check passed"
