# 计划：为 make publish 添加 AI 自动生成 Release Notes

## 背景

当前 `make publish` 支持两种方式获取 release notes：
1. 通过 `NOTE` 环境变量手动传入
2. 从 `CHANGELOG.md` 读取已有的最新版本段落

`make push` 目标已实现 AI 生成 commit message（使用 `j ai`），但 `make publish` 缺少这种自动生成能力。

## 需求

参考 `make push` 的 AI 生成模式，为 `make publish` 增加：**当没有 `NOTE` 且 `CHANGELOG.md` 中没有当前版本的条目时，使用 AI 自动生成 release notes**。

## 改动内容

仅修改 `Makefile` 中的 `publish` 目标（第 214-236 行）。

### 具体改动

在现有 `if [ -n "$${NOTE:-}" ]` 判断之后，增加一个 `elif` 分支：

1. 检查 `CHANGELOG.md` 的第一个 `# v` 标题是否匹配当前版本
2. 如果不匹配（即没有当前版本条目），使用 `j ai` 基于 `git log` 生成 release notes
3. AI 生成的内容格式为 Markdown 分类（新功能/改进/Bug 修复等）
4. 自动将生成的内容写入 `CHANGELOG.md` 顶部

### AI Prompt 设计

参考 `push` 目标的模式：
- 复用 `J_AI_EXTRACT` awk 辅助函数
- 使用临时文件传递 prompt（避免命令行长度限制）
- Prompt 要求 AI 根据 `git log` 变更历史生成结构化的 release notes

### 流程总结

```
make publish
  ├── bump-version
  ├── release (build)
  ├── 获取 release notes（优先级）:
  │   ├── 1. NOTE 环境变量（手动指定）
  │   ├── 2. CHANGELOG.md 已有当前版本条目
  │   └── 3. AI 自动生成（新增，从 git log 提取变更摘要）
  ├── 写入 CHANGELOG.md（如有 NOTE 或 AI 生成）
  ├── git commit + tag + push
  └── cargo publish
```

## 不改动的部分

- `bump-version`、`release`、`release-note`、`publish-check` 等其他目标不变
- `J_AI_EXTRACT` 已存在，直接复用
- `push` 目标不变
