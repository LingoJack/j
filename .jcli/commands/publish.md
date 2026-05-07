---
name: publish
description: 发布
---
使用 make publish 发布 jcli 的新的版本，注意要有清晰的 release notes

## 用法

### 1. 手动指定 Release Notes（推荐）

```bash
make publish NOTE='## jcli vX.Y.Z

### 新功能
- **功能名称**: 描述

### 改进
- **改进内容**: 描述

### Bug 修复
- **修复内容**: 描述

### 文档
- **文档变更**: 描述'
```

### 2. 自动 AI 生成 Release Notes

```bash
make publish
```

不传 `NOTE` 参数时，会自动调用 `j ai` 根据上次 tag 以来的 git log 生成中文发布说明。但如果 commit message 是自动时间戳格式（如"更新: YYYY-MM-DD HH:MM:SS"），AI 生成的质量可能不高，建议手动指定。

## 注意事项

- 每次发布会自动递增版本号 patch 位（如 12.10.7 -> 12.10.8）
- Release Notes 会被写入 git annotated tag 的 message 中
- 确保 `cargo publish` 有 crates.io 的 API token 配置
- NOTE 参数使用单引号包裹，避免 shell 对特殊字符的解析
