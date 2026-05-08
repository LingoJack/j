# Plan: 引入 CHANGELOG.md 管理 Release Notes

## 方案

### 1. 创建 `CHANGELOG.md`

简洁格式，每个版本用 `# v12.10.8` 做一级标题，内容直接是 release notes。新版本追加到**文件顶部**。

示例：
```markdown
# v12.10.11

### Bug 修复
- **修复 GitHub Release 页面不显示 release notes 的问题**: ...

### 改进
- **Makefile publish 支持 NOTE 参数**: ...

# v12.10.10

### Bug 修复
- ...
```

先回填 v12.10.8 ~ v12.10.11 的 release notes（从 git tag message 提取）。

### 2. 修改 `Makefile` 的 `publish` 目标

发布流程：
1. `bump-version` + `release` + `git add .`
2. 读取 release notes：
   - 有 `NOTE` 参数 → 追加到 CHANGELOG.md 顶部 + 用作 tag message
   - 无 `NOTE` → 从 CHANGELOG.md 提取第一个 `# v` 段落（到下一个 `# v` 之前）
3. commit + tag + push + cargo publish

提取逻辑用 awk：从文件开头读到下一个 `# v` 行之前。

### 3. 更新 `.jcli/commands/publish.md`

更新文档，说明 CHANGELOG.md 工作流。

## 涉及文件

| 文件 | 操作 |
|------|------|
| `CHANGELOG.md` | **新建** — 回填 v12.10.8 ~ v12.10.11 |
| `Makefile` | **修改** — `publish` 目标 |
| `.jcli/commands/publish.md` | **修改** — 更新文档 |
