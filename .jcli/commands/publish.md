---
name: publish
description: 发布
---
使用 make publish 发布 jcli 的新的版本，注意要有清晰的 release notes

## 工作流

Release notes 统一由 `CHANGELOG.md` 管理。每个版本用 `# v12.10.12` 一级标题标记，新版本追加到文件顶部。

## 用法

### 1. 手动指定 Release Notes（推荐）

```bash
NOTE='### 新功能
- **功能名称**: 描述

### 改进
- **改进内容**: 描述

### Bug 修复
- **修复内容**: 描述' make publish
```

NOTE 通过环境变量传递，会自动在 `CHANGELOG.md` 顶部插入 `# v版本号` 标题 + NOTE 内容，同时用作 git tag message。

### 2. 从 CHANGELOG.md 读取

```bash
make publish
```

不传 `NOTE` 时，从 `CHANGELOG.md` 提取第一个 `# v` 段落作为 release notes。适合提前编辑好 CHANGELOG.md 再发布的场景。

### 3. 预览当前 release notes

```bash
make release-note
```

打印 CHANGELOG.md 中最新版本段落的内容。

## 注意事项

- 每次发布会自动递增版本号 patch 位（如 12.10.11 -> 12.10.12）
- Release Notes 会写入 `CHANGELOG.md` 和 git annotated tag message
- GitHub Release 页面会自动读取 tag message 显示（已设置 `generate_release_notes: false`）
- NOTE 参数使用单引号包裹，避免 shell 对特殊字符的解析
- 确保 `cargo publish` 有 crates.io 的 API token 配置
