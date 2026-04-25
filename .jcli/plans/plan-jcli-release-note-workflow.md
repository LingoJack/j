# jcli Release Note 工作流方案（修订版）

## 背景

jcli 项目当前：
- 日常提交格式：`更新: YYYY-MM-DD HH:MM:SS`（没有变更说明）
- GitHub Release 依赖 `generate_release_notes: true`（质量一般）
- 项目已有 `j ai` (chat oneshot) 能力，可直接调用 LLM

## 目标

通过 **git hook** 在 commit 时自动调用 `j ai`，根据 diff 生成有意义的 commit message。
同时提供 `make push-ai` 和 `make release-note` 集成到发布流程。

---

## 方案：Git Hook + Makefile 集成

### 1. 创建 `prepare-commit-msg` Git Hook

文件：`.githooks/prepare-commit-msg`

```bash
#!/bin/bash
# jcli AI commit message generator
# 当设置环境变量 J_AI_COMMIT=1 时，自动用 AI 生成 commit message
#
# 启用方式:
#   1. git config core.hooksPath .githooks
#   2. export J_AI_COMMIT=1
#
# 或一次性使用:
#   J_AI_COMMIT=1 git commit

if [ "$J_AI_COMMIT" != "1" ]; then
    exit 0
fi

COMMIT_MSG_FILE=$1
COMMIT_SOURCE=$2

# 只在无 message 时触发（git commit 不带 -m 时）
# 如果是 merge、squash、已有 message 则跳过
if [ -n "$COMMIT_SOURCE" ]; then
    exit 0
fi

# 获取 staged diff
DIFF=$(git diff --cached --stat)
DIFF_DETAIL=$(git diff --cached --no-color --unified=3 -- ":(exclude)package-lock.json" ":(exclude)yarn.lock" ":(exclude)Cargo.lock")

# 如果没有 staged changes，跳过
if [ -z "$DIFF" ]; then
    exit 0
fi

# 限制 diff 大小（避免 token 过多）
if [ ${#DIFF_DETAIL} -gt 8000 ]; then
    DIFF_DETAIL=$(git diff --cached --no-color --unified=1 -- ":(exclude)package-lock.json" ":(exclude)yarn.lock" ":(exclude)Cargo.lock")
fi

if [ ${#DIFF_DETAIL} -gt 12000 ]; then
    DIFF_DETAIL="$DIFF"
fi

# 调用 j ai 生成 commit message
PROMPT="你是一个 Git 提交信息生成器。根据以下代码变更，生成一个简洁的 commit message。

要求:
1. 格式：<类型>: <中文描述>
2. 类型可选: feat/fix/refactor/docs/style/test/chore/perf
3. 描述不超过 30 字，清晰概括变更
4. 只输出一行 commit message，不要其他内容

变更文件:
$DIFF

变更详情:
$DIFF_DETAIL"

AI_MSG=$(j ai "$PROMPT" 2>/dev/null | head -5 | tr -d '\n' | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')

if [ -n "$AI_MSG" ]; then
    echo "$AI_MSG" > "$COMMIT_MSG_FILE"
    echo ""  # 空行分隔
    echo "# AI 生成的 commit message (可编辑修改)" >> "$COMMIT_MSG_FILE"
    echo "# 原始变更: " >> "$COMMIT_MSG_FILE"
    echo "# $DIFF" >> "$COMMIT_MSG_FILE"
else
    # fallback
    echo "更新: $(date +'%Y-%m-%d %H:%M:%S')" > "$COMMIT_MSG_FILE"
fi
```

### 2. Makefile 集成

在 Makefile 中新增两个目标：

```makefile
# AI 增强的推送（日常使用推荐）
push-ai: current_dir fmt build-web ## AI 生成 commit message 并推送
	@echo "🤖 AI 生成变更说明..."
	@diff=$$(git diff --stat); \
	if [ -z "$$diff" ]; then \
		echo "ℹ️ 没有未暂存的变更"; \
		diff=$$(git diff --cached --stat); \
	fi; \
	msg=$$(j ai "你是一个 Git 提交信息生成器。根据以下代码变更，生成一个简洁的 commit message。要求: 1. 格式：<类型>: <中文描述> 2. 类型: feat/fix/refactor/docs/style/test/chore/perf 3. 描述不超过 30 字 4. 只输出一行。变更: $$diff" 2>/dev/null | head -3); \
	if [ -z "$$msg" ]; then msg="更新: $$(date +'%Y-%m-%d %H:%M:%S')"; fi; \
	git add . && git commit -m "$$msg" && git push origin $(GIT_BRANCH)
	@echo "✅ 已推送: $$msg"

# 生成 Release Note（发布时使用）
release-note: ## 生成自上次 tag 以来的 Release Note
	@last_tag=$$(git describe --tags --abbrev=0 2>/dev/null); \
	if [ -z "$$last_tag" ]; then \
		log=$$(git log --oneline -20); \
	else \
		log=$$(git log --oneline "$$last_tag"..HEAD); \
	fi; \
	if [ -z "$$log" ]; then echo "没有新的提交"; exit 0; fi; \
	echo "📋 自 $$last_tag 以来的变更:"; \
	echo ""; \
	j ai "你是 Release Note 撰写者。根据以下 git log 生成中文版本发布说明，按类型分组（新功能/Bug修复/改进/其他），Markdown 格式，简洁明了。Git Log: $$log" 2>/dev/null
```

### 3. 修改现有 `publish` 流程

在发布流程中自动调用 `j ai` 生成 release note 并更新 GitHub Release：

```makefile
publish: ## 发布到 crates.io
	@echo "📦 开始发布流程..."
	@$(MAKE) bump-version
	@$(MAKE) release
	@version=$$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/'); \
	last_tag=$$(git describe --tags --abbrev=0 2>/dev/null); \
	if [ -n "$$last_tag" ]; then \
		log=$$(git log --oneline "$$last_tag"..HEAD); \
	else \
		log=$$(git log --oneline -20); \
	fi; \
	release_note=$$(j ai "你是 Release Note 撰写者。版本 v$$version 的变更如下，生成中文版本发布说明，按类型分组（新功能/Bug修复/改进/其他），Markdown 格式。Git Log: $$log" 2>/dev/null || echo "Release v$$version"); \
	git add . && git commit -m "chore: bump version to v$$version"; \
	git tag -a "v$$version" -m "$$release_note"; \
	git push origin $(GIT_BRANCH); \
	git push origin "v$$version"; \
	echo "📤 发布到 crates.io..."; \
	cargo publish --registry crates-io --allow-dirty; \
	echo "✅ 已发布 v$$version!"
```

### 4. 修改 GitHub Actions Release 工作流

在 `.github/workflows/release.yml` 中使用 tag message 作为 release body（而不是 `generate_release_notes: true`）：

```yaml
- name: Create GitHub Release
  uses: softprops/action-gh-release@v2
  with:
    body: ${{ steps.tag_message.outputs.message }}
  # 移除 generate_release_notes: true
```

或者更简单的方式：保持 `generate_release_notes: true` 不变，因为 tag message 已经包含了 AI 生成的说明。

---

## 使用方式

### 日常开发

```bash
# 方式1：使用 git hook（需先启用）
git config core.hooksPath .githooks
export J_AI_COMMIT=1
git add .
git commit  # 自动弹出 AI 生成的 message，可编辑

# 方式2：使用 make push-ai
make push-ai  # 自动生成 AI commit message 并推送

# 方式3：传统方式（保持兼容）
make push
```

### 版本发布

```bash
# 发布时自动生成 Release Note 作为 tag message
make publish

# 或手动查看 Release Note
make release-note
```

---

## 文件变更清单

| 文件 | 操作 | 说明 |
|------|------|------|
| `.githooks/prepare-commit-msg` | 新增 | Git hook，AI 生成 commit message |
| `Makefile` | 修改 | 新增 `push-ai`、`release-note` 目标，改进 `publish` |

无需修改任何 Rust 代码，完全利用现有的 `j ai` 能力。

---

## 实施步骤

1. 创建 `.githooks/prepare-commit-msg` 脚本
2. 修改 `Makefile`：新增 `push-ai` 和 `release-note` 目标
3. 修改 `Makefile`：改进 `publish` 目标，使用 AI 生成 tag message
4. 测试验证
