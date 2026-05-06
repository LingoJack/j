## 权限配置文件

权限配置位于项目目录 `.jcli/permissions.yaml`：

```yaml
permissions:
  # 完全放开（跳过所有工具确认）
  allow_all: false
  
  # 允许列表（匹配则跳过确认）
  allow:
    - Read
    - Grep
    - Glob
    - "Bash(cargo build:*)"
    - "Bash(git status:*)"
  
  # 拒绝列表（优先于 allow，匹配则直接拒绝执行）
  deny:
    - "Bash(rm -rf:*)"
    - "Bash(/.*sudo.*/)"    # 正则匹配
```

## 规则格式

| 格式 | 说明 | 示例 |
|------|------|------|
| `*` | 匹配所有工具 | `*` |
| `ToolName` | 匹配该工具所有调用 | `Read`, `Grep` |
| `ToolName(prefix:*)` | 前缀匹配 | `Bash(cargo build:*)` |
| `ToolName(path:/dir/*)` | 路径匹配 | `Write(path:/src/*)` |
| `ToolName(domain:example.com)` | 域名匹配 | `WebFetch(domain:docs.rs)` |
| `ToolName(/regex/)` | 正则匹配 | `Bash(/^cargo (build\|test)/)` |

## 匹配优先级

```
deny > allow > 默认需要确认
```

- `deny` 列表优先级最高，匹配则直接拒绝执行
- `allow` 列表匹配则跳过确认
- `allow_all: true` 跳过所有确认（但 deny 仍优先）

## 工具特定规则

### 平台差异

| 工具 | macOS / Linux | Windows |
|------|---------------|---------|
| Shell | `Bash(...)` | `PowerShell(...)` |
| Computer Use | `ComputerUse(...)` | 不可用 |

> **注意**：Windows 上使用 `PowerShell(...)` 规则匹配，而非 `Bash(...)`。

### Bash / PowerShell 命令匹配

macOS / Linux:

```yaml
allow:
  - "Bash(cargo:*)"        # cargo build, cargo test 等
  - "Bash(git status:*)"   # git status
  - "Bash(ls:*)"           # ls, ls -la 等

deny:
  - "Bash(rm -rf:*)"       # 阻止 rm -rf
  - "Bash(/.*sudo.*/)"     # 阻止所有 sudo 命令
```

Windows:

```yaml
allow:
  - "PowerShell(cargo:*)"        # cargo build, cargo test 等
  - "PowerShell(git status:*)"   # git status
  - "PowerShell(dir:*)"          # dir, dir /s 等

deny:
  - "PowerShell(Remove-Item -Recurse -Force:*)"  # 阻止递归强制删除
  - "PowerShell(/.*Format-.*/)"                   # 阻止格式化命令
```

### 文件路径匹配（Write/Edit/Read）

```yaml
# macOS / Linux
allow:
  - "Write(path:/src/*)"   # 允许写入 /src 目录
  - "Edit(path:/lib/*)"    # 允许编辑 /lib 目录

deny:
  - "Write(path:/etc/*)"   # 阻止写入 /etc

# Windows
allow:
  - "Write(path:C:\\Projects\\myapp\\src\\*)"  # 允许写入项目 src 目录
  - "Edit(path:C:\\Projects\\myapp\\lib\\*)"   # 允许编辑项目 lib 目录

deny:
  - "Write(path:C:\\Windows\\System32\\*)"     # 阻止写入系统目录
```

### URL 域名匹配（WebFetch）

```yaml
allow:
  - "WebFetch(domain:docs.rs)"
  - "WebFetch(domain:github.com)"
  - "WebFetch(domain:/.*\\.google\\.com$/)"  # 正则匹配所有 google 子域名
```
