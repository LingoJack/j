## 可用工具

| 工具 | 描述 |
|------|------|
| `Read` | 读取文件内容（支持图片：png/jpg/gif/webp/bmp） |
| `Write` | 写入文件（自动创建目录） |
| `Edit` | 字符串替换编辑文件（old_string 必须唯一匹配） |
| `Glob` | 按模式查找文件（支持 `**/*.rs` 等 glob 模式） |
| `Grep` | 正则搜索文件内容（支持 context、pagination） |
| `Bash` | 执行 shell 命令（支持后台执行） |
| `WebFetch` | 获取网页内容（转 Markdown 或纯文本） |
| `WebSearch` | Exa 搜索网络（需 EXA_API_KEY） |
| `Ask` | 向用户请求结构化输入（单选/多选） |
| `TaskOutput` | 获取后台任务输出 |
| `Task` | 管理任务（create/get/list/update） |
| `TodoWrite` | 写入待办事项（仅一个 in_progress） |
| `TodoRead` | 读取待办事项列表 |
| `Compact` | 压缩对话上下文（自动触发） |
| `RegisterHook` | 注册 session 级钩子 |
| `ComputerUse` | macOS 桌面控制（截图、点击、输入） |
| `EnterPlanMode` | 进入计划模式（只读工具） |
| `ExitPlanMode` | 退出计划模式（提交计划） |
| `LoadSkill` | 加载技能（按需注册） |
| `Agent` | 子代理执行复杂任务（防止递归） |
| `Browser` | 浏览器工具（Lite/CDP 模式） |

## 权限配置

权限配置位于 `.jcli/permissions.yaml`，支持三种规则：

```yaml
# .jcli/permissions.yaml
permissions:
  # 完全放开（跳过所有工具确认）
  allow_all: false
  
  # 允许列表（匹配则跳过确认，支持正则）
  allow:
    - Read
    - Grep
    - Glob
    - "Bash:ls.*"       # 正则匹配命令参数
    - "Bash:git status"
  
  # 拒绝列表（优先于 allow，匹配则直接拒绝）
  deny:
    - "Bash:rm -rf.*"   # 阻止危险命令
    - "Bash:.*sudo.*"   # 阻止 sudo 命令
```

### 规则匹配说明

- **简单匹配**：工具名（如 `Read`、`Bash`）
- **正则匹配**：`工具名:正则表达式`（如 `Bash:rm.*` 匹配 Bash 工具的 command 参数）
- **优先级**：deny > allow > 默认需要确认

## 上下文引用

| 引用 | 描述 |
|------|------|
| `@file:路径` | 包含文件内容（自动读取并注入上下文） |
| `@skill:名称` | 加载并激活指定 skill |
