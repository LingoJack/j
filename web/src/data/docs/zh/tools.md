## 可用工具

| 工具 | 描述 |
|------|------|
| `Read` | 读取文件内容 |
| `Write` | 写入文件 |
| `Edit` | 字符串替换编辑文件 |
| `Glob` | 按模式查找文件 |
| `Grep` | 搜索文件内容 |
| `Bash` | 执行 shell 命令 |
| `WebFetch` | 获取网页内容 |
| `WebSearch` | 搜索网络 |
| `Ask` | 向用户请求输入 |
| `TaskOutput` | 获取后台任务输出 |
| `Task` | 管理任务 |
| `TodoWrite` | 写入待办事项 |
| `TodoRead` | 读取待办事项 |
| `Compact` | 压缩对话上下文 |
| `RegisterHook` | 注册钩子 |
| `ComputerUse` | macOS 桌面控制 |
| `EnterPlanMode` | 进入计划模式 |
| `ExitPlanMode` | 退出计划模式 |
| `LoadSkill` | 加载技能 |

## 权限配置

```yaml
# ~/.jdata/agent/data/agent_config.yaml
tools:
  - name: Read
    permission: allow
  - name: Bash
    permission: ask  # 需要用户确认
  - name: Write
    permission: deny
```

## 上下文引用

| 引用 | 描述 |
|------|------|
| `@file:路径` | 包含文件内容 |
| `@dir:路径` | 包含目录结构 |
| `@url:url` | 包含网页内容 |
| `@grep:模式` | 包含搜索结果 |
