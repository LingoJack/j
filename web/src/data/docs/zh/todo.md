## 命令

| 命令 | 描述 |
|------|------|
| `j todo` | 打开 TUI 待办管理器 |
| `j todo add <内容>` | 快速添加待办 |
| `j todo done <id>` | 标记待办完成 |
| `j todo list` | 列出待办（支持 --done/--undone） |

## 示例

```bash
# 快速添加
j todo add 买牛奶
j todo add 审查 PR

# 列出待办
j todo list              # 所有待办
j todo list --undone     # 仅未完成
j todo list --done       # 仅已完成

# 标记完成
j todo done 1
j todo done 1 --report   # 同时写入日报
```

## TUI 管理器

运行 `j todo` 打开交互式 TUI：

- **添加/编辑/删除**：交互式管理待办
- **优先级**：设置优先级
- **截止日期**：添加截止时间
- **分类**：按项目/上下文组织

## Markdown 集成

待办可以在日报中使用 Markdown 格式：

```markdown
- [x] 已完成的任务
- [ ] 待处理的任务
- [ ] 另一个待处理任务
```
