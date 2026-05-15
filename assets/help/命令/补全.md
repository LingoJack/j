---
name: Shell 补全
order: 32
---

## Shell 补全

启用 Tab 自动补全，提升命令行操作效率。

### 启用补全

```bash
# Zsh（默认）
eval "$(j completion zsh)"    # 加入 ~/.zshrc

# Bash
eval "$(j completion bash)"   # 加入 ~/.bashrc

# Fish
j completion fish             # 按 fish 机制生成
```

### 命令说明

| 命令 | 说明 |
|------|------|
| `j completion` | 生成 zsh 补全脚本（默认） |
| `j completion zsh` | 生成 zsh 补全脚本 |
| `j completion bash` | 生成 bash 补全脚本 |
| `j completion fish` | 生成 fish 补全脚本 |

### 设置步骤

1. 在 shell 配置文件（如 `~/.zshrc`）中添加对应 `eval` 语句
2. 重新加载配置：`source ~/.zshrc`
3. 输入 `j ` 后按 Tab 即可触发补全
