# Plan: Compact UI Message 显示

## 需求

compact（自动/手动）完成后，在 UI 聊天界面以 Agent 工具 result 嵌套边框风格显示压缩结果。仅 UI 显示，不进入 LLM context。

## 方案

### 核心思路

`ui_messages` 是纯 UI 管道，与 LLM `messages` 完全独立。compact 完成后向 `ui_messages` 推一条专用 role 的消息即可。

### 1. 新增 `ROLE_COMPACT_UI` 常量

`constants.rs`:
```rust
pub const ROLE_COMPACT_UI: &str = "_compact_ui";
```
- `to_openai_messages` 的 `filter_map` 中不匹配该 role → 自动丢弃，不会发给 LLM
- 渲染层新增专属分支

### 2. 扩展 `CompactResult`

`compact.rs`:
```rust
pub struct CompactResult {
    pub messages_before: usize,
    pub messages_after: usize,     // 新增
    pub transcript_path: String,   // 新增
    pub summary_preview: String,   // 新增：摘要前 N 字符
}
```
- 移除现有的 `messages` 中 system 消息推送（compact.rs:486-492）
- 让 `auto_compact()` 填充完整的统计信息

### 3. 在 `agent_loop.rs` 调用处向 `ui_messages` 推送

3 处 auto_compact 调用点（L200-233 自动触发, L741-750 手动触发 fallback, L913-922 手动触发主路径），Ok 分支中：

```rust
if let Ok(result) = compact::auto_compact(...).await {
    let compact_ui_msg = ChatMessage {
        role: ROLE_COMPACT_UI.to_string(),
        content: serde_json::json!({
            "messages_before": result.messages_before,
            "messages_after": result.messages_after,
            "transcript_path": result.transcript_path,
            "summary_preview": result.summary_preview,
            "trigger": "auto",  // 或 "manual"
        }).to_string(),
        ..Default::default()
    };
    push_ui(&ui_messages, compact_ui_msg);
}
```

### 4. 渲染层新增 `ROLE_COMPACT_UI` 分支

`render/cache.rs` `build_message_lines_incremental` 中：

```rust
ROLE_COMPACT_UI => {
    render_compact_ui_msg(&m.content, bubble_max_width, &mut tmp_lines, t);
}
```

新增 `render_compact_ui_msg()` 函数，参考 `render_agent_result_nested` 嵌套边框风格：
- 顶边框 `╭───╮`
- `📦 上下文压缩完成` 标题（高亮色）
- `压缩前 N 条 → 压缩后 M 条` 统计
- 摘要预览（最多 10 行）
- Transcript 路径（灰色）
- 底边框 `╰───╯`

### 改动清单

| 文件 | 改动 |
|------|------|
| `constants.rs` | 新增 `ROLE_COMPACT_UI` 常量 |
| `compact.rs` | 扩展 `CompactResult`；移除 system 消息推送；返回完整统计 |
| `agent_loop.rs` | 3 处 auto_compact 调用 Ok 分支，向 `ui_messages` 推 compact UI 消息 |
| `render/cache.rs` | 新增 `ROLE_COMPACT_UI` 渲染分支 + `render_compact_ui_msg()` 函数 |
