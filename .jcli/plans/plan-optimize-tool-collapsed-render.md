# 优化工具缩略模式渲染

## 问题

部分工具在 TUI 缩略模式（collapsed）下直接显示原始 JSON 或无意义的通用摘要（如 "N 行, N 字符"），用户难以快速理解工具执行了什么操作。

涉及两处代码：
1. **tool_call 缩略** (`tool_call_render.rs`): `extract_tool_description_from_args()` 的 `_ => None` 分支会直接显示原始 JSON 截断
2. **tool_result 缩略** (`classification.rs`): `get_result_summary_for_tool()` 的 `_ => get_generic_summary()` 分支只显示 "N 行, N 字符"

## 具体缺失的工具覆盖

### tool_call 端 (`extract_tool_description_from_args`) 缺失：
- `LOAD_TOOL` → 显示原始 JSON
- `SESSION` → 显示原始 JSON

### tool_result 端 (`get_result_summary_for_tool`) 缺失（均走通用摘要）：
| 工具 | 当前显示 | 优化后显示 |
|------|---------|-----------|
| Write | "N 行, N 字符" | "写入文件: path (N 字符)" |
| Edit | "N 行, N 字符" | "编辑文件: path (N 处修改)" |
| Glob | "N 行, N 字符" | "搜索: pattern → N 个匹配" |
| Grep | "N 行, N 字符" | "搜索: pattern → N 处匹配" |
| WebFetch | "N 行, N 字符" | "url (N 行)" |
| WebSearch | "N 行, N 字符" | "query → N 条结果" |
| Browser | "N 行, N 字符" | "url" |
| Ask | "N 行, N 字符" | "用户已回答" |
| TaskOutput | "N 行, N 字符" | "获取任务 task_id 输出" |
| RegisterHook | "N 行, N 字符" | "钩子已注册" |
| LoadSkill | "N 行, N 字符" | "技能已加载: name" |
| SendMessage | "N 行, N 字符" | "消息已发送" |
| WorkDone | "N 行, N 字符" | "工作完成" |
| Plan | "N 行, N 字符" | "计划模式已进入/已退出" |
| Worktree | "N 行, N 字符" | "工作树已进入/已退出" |
| LoadTool | "N 行, N 字符" | "工具已加载: name" |
| Session | "N 行, N 字符" | "会话操作" |
| ComputerUse | "N 行, N 字符" | "计算机操作: action" |
| IgnoreMessage | "N 行, N 字符" | "消息已忽略" |

## 修改方案

### 文件 1: `j-agent/src/tools/classification.rs`

在 `get_result_summary_for_tool()` 的 match 中增加以下工具的专属摘要函数：

```rust
// 新增的 match 分支
tool_names::WRITE => get_write_summary(content, tool_args),
tool_names::EDIT => get_edit_summary(content, tool_args),
tool_names::GLOB => get_glob_summary(content, tool_args),
tool_names::GREP => get_grep_summary(content, tool_args),
tool_names::WEB_FETCH => get_web_fetch_summary(content, tool_args),
tool_names::WEB_SEARCH => get_web_search_summary(content, tool_args),
tool_names::BROWSER => get_browser_summary(content, tool_args),
tool_names::ASK => "用户已回答".to_string(),
tool_names::TASK_OUTPUT => get_task_output_summary(content, tool_args),
tool_names::REGISTER_HOOK => "钩子已注册".to_string(),
tool_names::LOAD_SKILL => get_load_skill_result_summary(content, tool_args),
tool_names::SEND_MESSAGE => "消息已发送".to_string(),
tool_names::WORK_DONE => get_work_done_result_summary(content, tool_args),
tool_names::ENTER_PLAN_MODE | tool_names::EXIT_PLAN_MODE => get_plan_result_summary(content),
tool_names::ENTER_WORKTREE | tool_names::EXIT_WORKTREE => get_worktree_result_summary(content),
tool_names::LOAD_TOOL => get_load_tool_result_summary(content, tool_args),
tool_names::SESSION => "会话操作完成".to_string(),
tool_names::IGNORE_MESSAGE => "消息已忽略".to_string(),
#[cfg(target_os = "macos")]
tool_names::COMPUTER_USE => get_computer_use_result_summary(content, tool_args),
```

每个新增的 `get_*_summary` 函数实现：
- 优先从 `tool_args` 提取关键参数（如 path、pattern、url）
- 结合 `content` 的行数/大小信息
- 输出简洁的人类可读摘要

### 文件 2: `src/command/chat/render/cache/tool_call_render.rs`

在 `extract_tool_description_from_args()` 中补充：

```rust
tool_names::LOAD_TOOL => parsed
    .get("name")
    .and_then(|v| v.as_str())
    .map(|s| format!("加载工具: {}", s)),
tool_names::SESSION => parsed
    .get("action")
    .and_then(|v| v.as_str())
    .map(|s| format!("会话: {}", s))
    .or_else(|| Some("会话操作".to_string())),
```

## 影响范围

- 仅修改缩略模式显示的文本内容，不影响展开模式
- 不影响工具执行逻辑
- 向后兼容，所有未覆盖的工具仍走 `_ => get_generic_summary()` 兜底
