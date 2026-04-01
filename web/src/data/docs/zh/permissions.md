## 权限级别

| 级别 | 描述 |
|------|------|
| `allow` | 始终允许 |
| `ask` | 请求确认 |
| `deny` | 始终拒绝 |

## 配置

```yaml
# ~/.jdata/agent/data/agent_config.yaml
permissions:
  # 读取操作 - 始终允许
  - tool: Read
    permission: allow
  
  # 写入操作 - 请求确认
  - tool: Write
    permission: ask
  
  # Shell 命令 - 请求确认
  - tool: Bash
    permission: ask
    rules:
      - pattern: "ls *"        # 允许 ls 命令
        permission: allow
      - pattern: "rm *"        # rm 始终询问
        permission: ask
  
  # 网络访问 - 始终允许
  - tool: WebFetch
    permission: allow
  - tool: WebSearch
    permission: allow
```

## 细粒度规则

```yaml
permissions:
  - tool: Bash
    permission: ask
    rules:
      # 允许特定模式
      - pattern: "git status"
        permission: allow
      - pattern: "cargo build"
        permission: allow
      
      # 拒绝危险模式
      - pattern: "rm -rf /*"
        permission: deny
```
