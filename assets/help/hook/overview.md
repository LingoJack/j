---
name: Hook 概述
order: 1
---

## 概念与来源

Hook 允许在对话关键节点注入自定义逻辑。对用户可配置部分，支持三级来源：

1. **用户级**：`~/.jdata/agent/hooks/<hook_name>/HOOK.yaml`（或 `HOOK.yml`）— 全局生效
2. **项目级**：`.jcli/hooks/<hook_name>/HOOK.yaml`（或 `HOOK.yml`）— 项目目录下生效
3. **Session 级**：通过 `register_hook` 工具由 AI 动态注册 — 仅当前会话

> 运行时实际还存在**内置 hook**，执行顺序是：内置 -> 用户级 -> 项目级 -> Session 级。
> 同一事件按链式执行，前一个 hook 的输出会成为后一个 hook 的输入。

## 事件生命周期总览

下面按一条消息从「用户输入」到「AI 回复完成」的完整流程，标注所有 Hook 事件的触发位置：

```
[会话启动] ──→ session_start
[会话退出] ──→ session_end

一轮对话的完整流程（循环）：

  用户输入
    │
    ▼
  ┌─────────────────────┐
  │  pre_send_message   │  ← 可修改/拦截用户消息
  └────────┬────────────┘
           ▼
  ┌─────────────────────┐
  │  post_send_message  │  ← 仅通知，不可修改
  └────────┬────────────┘
           ▼
  ┌─────────────────────┐
  │  pre_llm_request    │  ← 可修改 system_prompt / messages
  └────────┬────────────┘
           ▼
       LLM 请求
           │
           ▼
  ┌─────────────────────┐
  │  post_llm_response  │  ← 可修改 AI 回复内容
  └────────┬────────────┘
           │
           ├── AI 回复中包含工具调用？
           │       │
           │       ▼
           │   ┌──────────────────────────┐
           │   │  pre_tool_execution      │  ← 可修改/跳过工具参数
           │   └────────────┬─────────────┘
           │                ▼
           │           执行工具
           │                │
           │        ┌───────┴────────┐
           │        ▼                ▼
           │   成功:                失败:
           │   post_tool_execution   post_tool_execution_failure
           │   (可修改结果)          (可修改错误信息)
           │        │                │
           │        └───────┬────────┘
           │                ▼
           │       工具结果返回 LLM → 回到 pre_llm_request
           │       (LLM 基于工具结果继续生成)
           │
           ├── AI 回复中不再有工具调用？
           │       │
           │       ▼
           │   ┌─────────────────────┐
           │   │  stop               │  ← LLM 即将结束回复
           │   └────────┬────────────┘
           │            ▼
           │       回到顶部，等待下一轮用户输入
           │
           └── 上下文接近上限时触发压缩：
                   │
                   ├── micro_compact（轮次级压缩）
                   │       │
                   │   pre_micro_compact → 执行压缩 → post_micro_compact
                   │
                   └── auto_compact（全量压缩）
                           │
                       pre_auto_compact → 执行压缩 → post_auto_compact
```