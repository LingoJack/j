---
name: 权限配置
order: 4
parent: tools
---

## .jcli/ 权限配置

在项目根目录创建 `.jcli/` 目录，在其中放置 `permissions.yaml` 文件，可细粒度控制工具的自动执行权限。程序会从当前目录向上查找 `.jcli/` 目录。

### 配置示例

```yaml
# .jcli/permissions.yaml
# allow_all: true  # 完全放开

allow:
  - "Bash(cargo build:*)"   # Bash 命令前缀匹配
  - "Bash(cargo test:*)"
  - "Read"                   # 工具级别放行
  - "Glob"
  - "Write(path:/Users/jack/projects/*)"  # 文件路径前缀匹配
  - "WebFetch(domain:docs.rs)"            # URL 域名匹配

deny:
  - "Bash(rm -rf:*)"        # 黑名单（优先于 allow）
  - "Bash(sudo:*)"
```

### 规则说明

- `deny` 优先于 `allow`：即使某条规则同时命中 allow 和 deny，deny 生效
- 无 `.jcli/` 目录时保持默认行为（需确认的工具弹确认框）
- `allow_all: true` 可完全放开所有工具权限（危险，慎用）

### 匹配模式

| 模式 | 说明 | 示例 |
|------|------|------|
| `ToolName` | 工具级别放行 | `"Read"`、`"Glob"` |
| `Bash(prefix:*)` | Bash 命令前缀匹配 | `"Bash(cargo build:*)"` |
| `Write(path:prefix*)` | 文件路径前缀匹配 | `"Write(path:/Users/jack/projects/*)"` |
| `WebFetch(domain:name)` | URL 域名匹配 | `"WebFetch(domain:docs.rs)"` |
