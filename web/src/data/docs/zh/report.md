## 命令

| 命令 | 描述 |
|------|------|
| `j report [内容]` | 写入日报（无内容时打开 TUI） |
| `j check [n]` | 查看最近 n 行（默认 10 行） |
| `j search <关键词>` | 模糊搜索日报 |

## 示例

```bash
# 快速写入
j report "完成用户认证模块"
j report "团队会议" "讨论冲刺计划"

# 查看日报
j check          # 查看最近 10 行
j check 20       # 查看最近 20 行

# 搜索
j search 认证
j search "用户模块" -fuzzy
```

## TUI 编辑器

不带参数运行 `j report` 会打开 TUI 编辑器：

- **多行编辑**：支持更长的内容和格式化
- **历史建议**：从历史记录自动补全
- **Tab 补全**：快速插入常用短语

## Git 同步

```bash
# 初始化 Git 同步
cd ~/.jdata/report
git init
git remote add origin <你的仓库>

# 日常工作流
j report "完成功能"
j reportctl push   # 同步到远程
```
