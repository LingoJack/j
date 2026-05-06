## 文件工具

### Read

读取文件内容，支持图片格式。

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| path | string | 是 | 文件路径（绝对或相对路径） |
| offset | uint | 否 | 起始行号（0-based，默认从头开始） |
| limit | uint | 否 | 读取行数（默认读取全部） |

**支持的图片格式：** PNG、JPG、GIF、WEBP、BMP

```json
{"path": "src/main.rs"}
{"path": "screenshot.png"}
{"path": "large.log", "offset": 100, "limit": 50}
```

### Write

写入文件，自动创建父目录。会覆盖已存在的文件。

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| path | string | 是 | 文件路径 |
| content | string | 是 | 写入内容 |

```json
{"path": "src/new_file.rs", "content": "fn main() {}\n"}
```

### Edit

字符串替换编辑文件。`old_string` 必须唯一匹配文件中的内容。

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| path | string | 是 | 文件路径 |
| old_string | string | 是 | 要替换的原始字符串（必须唯一匹配） |
| new_string | string | 否 | 替换后的字符串（空则删除） |

```json
{"path": "src/main.rs", "old_string": "fn main() {}", "new_string": "fn main() { println!(\"Hello\"); }"}
```

### Glob

按模式查找文件。

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| pattern | string | 是 | Glob 模式（如 `**/*.rs`、`src/**/*.ts`） |
| path | string | 否 | 搜索目录（默认当前目录） |
| excludePattern | string | 否 | 排除模式（如 `**/node_modules/**`） |
| limit | uint | 否 | 最大结果数（默认 100） |

```json
{"pattern": "**/*.rs"}
{"pattern": "src/**/*.tsx", "excludePattern": "**/node_modules/**"}
```

### Grep

正则搜索文件内容。

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| pattern | string | 是 | 正则表达式 |
| path | string | 否 | 搜索路径（默认当前目录） |
| glob | string | 否 | 文件过滤（如 `*.rs`） |
| type | string | 否 | 文件类型（js/py/rust/go/java） |
| output_mode | string | 否 | 输出模式：content/files_with_matches/count |
| context | uint | 否 | 上下文行数 |
| head_limit | uint | 否 | 最大结果数 |
| ignore_case | bool | 否 | 忽略大小写（默认 false） |

```json
{"pattern": "fn\\s+\\w+", "type": "rust"}
{"pattern": "TODO", "glob": "*.rs", "output_mode": "count"}
```

## 执行工具

### Bash

执行 shell 命令（仅 macOS / Linux）。

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| command | string | 是 | Shell 命令（在 bash -c 中执行） |
| cwd | string | 否 | 工作目录 |
| timeout | uint | 否 | 超时秒数（默认 120，最大 600） |
| run_in_background | bool | 否 | 后台执行（返回 task_id） |

**注意事项：**
- 不支持交互式命令
- 构建命令建议 timeout 设为 300-600
- 后台任务使用 `TaskOutput` 获取结果

```json
{"command": "cargo build --release", "timeout": 300}
{"command": "npm run dev", "run_in_background": true}
```

### PowerShell

执行 PowerShell 命令（仅 Windows）。

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| command | string | 是 | PowerShell 命令（在 powershell.exe -NoProfile -Command 中执行） |
| description | string | 否 | 命令描述 |
| cwd | string | 否 | 工作目录 |
| timeout | uint | 否 | 超时秒数（默认 120，最大 600） |
| run_in_background | bool | 否 | 后台执行（返回 task_id） |

**注意事项：**
- 不支持交互式命令
- 构建命令建议 timeout 设为 300-600
- 后台任务使用 `TaskOutput` 获取结果

```json
{"command": "cargo build --release", "timeout": 300}
{"command": "npm run dev", "run_in_background": true}
```

### TaskOutput

获取后台任务输出。

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| task_id | string | 是 | 后台任务 ID |
| block | bool | 否 | 是否等待完成（默认 true） |
| timeout | uint | 否 | 等待超时毫秒（默认 30000，最大 600000） |

## 网络工具

### WebFetch

获取网页内容，自动转换为 Markdown 或纯文本。

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| url | string | 是 | 完整 URL（http:// 或 https://） |
| extract_mode | string | 否 | 输出格式：markdown/text |
| max_chars | uint | 否 | 最大字符数（默认 50000） |
| headers | object | 否 | 自定义请求头 |
| authorization | string | 否 | Authorization 头 |

```json
{"url": "https://docs.rs/serde"}
{"url": "https://api.github.com/repos/rust-lang/rust", "headers": {"Accept": "application/vnd.github.v3+json"}}
```

### WebSearch

使用 Exa 搜索网络（需要 `EXA_API_KEY` 环境变量）。

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| query | string | 是 | 搜索关键词 |
| count | uint | 否 | 结果数量（1-10，默认 5） |
| type | string | 否 | 搜索类型：auto/keyword/neural |

## 交互工具

### Ask

向用户请求结构化输入，支持单选/多选。

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| questions | array | 是 | 问题列表（1-4 个） |

**Question 结构：**

| 字段 | 类型 | 描述 |
|------|------|------|
| header | string | 短标签 |
| question | string | 完整问题文本 |
| options | array | 选项列表（2-4 个） |
| multi_select | bool | 是否多选 |

```json
{
  "questions": [{
    "header": "风格",
    "question": "选择代码风格",
    "options": ["简洁", "详细", "标准"],
    "multi_select": false
  }]
}
```

## 任务工具

### Task

管理任务（create/get/list/update）。

**操作类型：**

| action | 描述 |
|--------|------|
| create | 创建任务（需要 title） |
| get | 获取任务详情（需要 taskId） |
| list | 列出所有任务 |
| update | 更新任务状态 |

**任务状态流转：** `pending` → `in_progress` → `completed`

```json
{"action": "create", "title": "实现用户认证", "description": "添加登录/注册功能"}
{"action": "update", "taskId": 1, "status": "in_progress"}
{"action": "list", "ready": true}
```

### TodoWrite

管理待办事项列表。

| 参数 | 类型 | 描述 |
|------|------|------|
| todos | array | 待办列表 |
| merge | bool | 是否合并更新 |

**Todo 结构：**

| 字段 | 类型 | 描述 |
|------|------|------|
| id | string/int | 待办 ID |
| content | string | 内容 |
| status | string | 状态：pending/in_progress/completed |

**规则：** 同时只能有一个 `in_progress` 项

### TodoRead

读取当前待办列表。返回所有待办项的 id、content 和 status。

## 计划工具

### EnterPlanMode

进入计划模式，只读工具可用，写工具被阻止。

**用途：** 在开始非平凡实现任务前，探索代码库并设计实现方案。

**适用场景：**
- 新功能实现涉及架构决策
- 存在多种有效方案需要用户选择
- 代码修改影响现有行为
- 多文件变更（超过 2-3 个文件）

### ExitPlanMode

退出计划模式，提交计划供用户审批。

| 参数 | 类型 | 描述 |
|------|------|------|
| allowedPrompts | array | 实现计划所需的提示权限 |

## 扩展工具

### LoadSkill

加载指定技能到上下文。

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| name | string | 是 | 技能名称 |
| arguments | string | 否 | 传递给技能的参数 |

**可用技能：**

| 技能 | 描述 |
|------|------|
| j-cli | 命令行工具工作流自动化 |
| skill-creator | 创建新技能指南 |
| swift-ios-app-gen | iOS 原生应用开发 |

### Agent

启动子代理处理复杂多步任务。子代理使用全新上下文，可以使用除 Agent 外的所有工具。

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| prompt | string | 是 | 子代理任务描述 |
| description | string | 否 | 简短描述（3-5 词） |
| run_in_background | bool | 否 | 后台运行 |

### RegisterHook

注册、列出、移除会话级钩子。

**操作：**

| action | 描述 |
|--------|------|
| register | 注册钩子（需要 event + command） |
| list | 列出所有钩子 |
| remove | 移除钩子（需要 event + index） |
| help | 查看协议文档 |

## 会话工具

### Compact

压缩对话上下文，释放上下文窗口。

**适用场景：**
- 对话变长需要继续高效工作
- 多次尝试解决问题失败后需要清理思路

## ComputerUse 工具

macOS 桌面控制工具，支持截图、点击、输入等操作（仅 macOS）。

### 截图操作

#### screenshot

截取屏幕并返回带有 SoM (Set-of-Mark) 标注的图片。

| 参数 | 类型 | 描述 |
|------|------|------|
| som | bool | 启用 SoM 标注（默认 true） |
| app | string | 目标应用名称 |

**SoM 标注：**
- 为每个可交互元素绘制编号边框
- 返回元素索引供后续点击引用
- 支持通过 `element` 参数点击编号元素

### 鼠标操作

#### click

点击指定位置或元素。

| 参数 | 类型 | 描述 |
|------|------|------|
| x, y | number | 坐标点（逻辑点） |
| element | uint | SoM 元素编号 |

#### double_click

双击指定位置。

#### right_click

右键点击指定位置。

#### drag

拖拽操作。

| 参数 | 类型 | 描述 |
|------|------|------|
| start_x, start_y | number | 起始坐标 |
| end_x, end_y | number | 结束坐标 |
| start_element, end_element | uint | SoM 元素编号 |
| duration_ms | uint | 拖拽持续时间 |

#### scroll

滚动操作。

| 参数 | 类型 | 描述 |
|------|------|------|
| dx, dy | int | 滚动量（负值向上） |

### 键盘操作

#### type

输入文本。

| 参数 | 类型 | 描述 |
|------|------|------|
| text | string | 要输入的文本 |
| delay_ms | uint | 按键间隔毫秒 |

#### key

按下单个按键。

| 参数 | 类型 | 描述 |
|------|------|------|
| key | string | 按键名称（如 enter、tab、escape） |

#### keys

按键组合。

| 参数 | 类型 | 描述 |
|------|------|------|
| keys | array | 按键列表（如 `["cmd", "c"]`） |

**支持的按键：**
- 字母：a-z
- 数字：0-9
- 特殊：enter、tab、space、escape、delete、backspace
- 方向：up、down、left、right
- 功能：f1-f12、home、end、pageup、pagedown
- 修饰：cmd、shift、alt/option、ctrl

### 辅助操作

#### find_element

查找元素。

| 参数 | 类型 | 描述 |
|------|------|------|
| query | string | 搜索查询 |

#### ax_tree

查询无障碍树。

| 参数 | 类型 | 描述 |
|------|------|------|
| app | string | 目标应用 |
| depth | uint | 树深度限制 |
| role | string | 角色过滤（如 AXButton） |
| clickable | bool | 仅显示可点击元素 |

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
