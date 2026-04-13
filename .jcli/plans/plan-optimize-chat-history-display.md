# Plan: 对话记录展示优化方案

## 一、现状分析

### 1.1 当前架构概览

对话记录展示涉及以下核心模块：

| 文件 | 职责 |
|------|------|
| `render_cache.rs` | 消息渲染行缓存 + 增量构建 (P0/P1/P2优化) |
| `ui/chat.rs` | 主绘制入口 (`draw_messages`) |
| `markdown/parser.rs` | Markdown → Line 转换 |
| `app/ui_state.rs` | UI 状态管理 (`MsgLinesCache`, `PerMsgCache`) |
| `handler/tui_loop.rs` | TUI 事件循环 + 渲染节流 |

### 1.2 当前渲染流程

```
session.messages → build_message_lines_incremental → per_msg_lines + streaming_lines → draw_messages
```

**已有优化措施：**
- P0：消息级缓存，历史消息内容未变时直接复用
- P1：流式消息增量段落渲染（只解析最后一个不完整段落）
- P2：避免扁平 Vec 组装，直接索引缓存
- 渲染节流：30fps (~33ms) + 流式内容 200字节/150ms 触发

### 1.3 发现的问题点

1. **视觉层次不够清晰**
   - 用户消息与 AI 消息的气泡样式区分度有限
   - 工具调用/结果消息在折叠模式下信息密度高，难以快速定位

2. **长消息处理体验欠佳**
   - 代码块超长时只显示，缺少「折叠/展开」交互
   - Diff 内容无语法高亮区分（仅有颜色）
   - 工具结果限制 100 行，但无「查看完整内容」入口

3. **时间信息缺失**
   - 消息无时间戳显示，无法追溯对话时序
   - 流式响应无进度/耗时指示

4. **工具消息视觉冗余**
   - `tool_call_request` 和 `tool_result` 分离显示，占用双倍空间
   - 折叠模式下参数预览截断策略简单（60字符硬截断）

5. **消息导航效率低**
   - Browse 模式只能逐条滚动，无「跳转到首/尾/指定消息」快捷键
   - 无消息搜索/过滤功能

---

## 二、优化方案

### 2.1 视觉层次增强

#### 2.1.1 消息时间戳显示

**改动范围：** `render_cache.rs`, `storage.rs`

**方案：**
- 在消息气泡顶部/底部显示相对时间（如「2分钟前」）或绝对时间（如「14:32」）
- 时间戳使用 `text_dim` 颜色，不干扰主要内容
- 仅在消息间隔 > 30秒时显示时间戳，避免视觉冗余

```rust
// storage.rs: Message 结构体添加 timestamp 字段
pub struct Message {
    pub role: String,
    pub content: String,
    pub timestamp: Option<i64>,  // Unix timestamp (秒)
    // ...
}

// render_cache.rs: 渲染时间戳行
fn render_timestamp_line(timestamp: i64, theme: &Theme) -> Line<'static> {
    let elapsed = Utc::now().timestamp() - timestamp;
    let text = if elapsed < 60 {
        "刚刚".to_string()
    } else if elapsed < 3600 {
        format!("{}分钟前", elapsed / 60)
    } else {
        format!("{}", Local.timestamp(timestamp).format("%H:%M"))
    };
    Line::from(Span::styled(format!("  {}", text), Style::default().fg(theme.text_dim)))
}
```

#### 2.1.2 消息类型图标增强

**改动范围：** `tools/classification.rs`, `render_cache.rs`

**方案：**
- 扩展 `ToolCategory` 图标集，增加更多语义化图标
- 为 user/assistant/system 消息添加角色图标前缀
- 选中消息时图标高亮放大

| 消息类型 | 当前图标 | 建议图标 |
|---------|---------|---------|
| User | 无 | `👤` 或 `🗣️` |
| Assistant | "Sprite" 标签 | `🤖` + "Sprite" |
| Tool Call | 分类图标 | 保持现状 |
| Tool Result | `🔧` | 根据成功/失败状态区分 (`✅`/`❌`) |
| System | "sys" 标签 | `⚙️` |

---

### 2.2 长消息交互优化

#### 2.2.1 代码块折叠/展开

**改动范围：** `render_cache.rs`, `app/ui_state.rs`, `handler/browse.rs`

**方案：**
- 新增 `code_block_expand_state: HashMap<(msg_idx, block_idx), bool>` 状态
- 代码块超过 10 行时默认折叠，显示 `▼ 展开 (N 行)` 指示器
- Browse 模式下 Enter 键切换当前消息的代码块展开状态
- 折叠状态只显示代码块首 3 行 + `... (共 N 行)` 提示

```rust
// ui_state.rs
pub struct CodeBlockState {
    pub msg_idx: usize,
    pub block_idx: usize,
    pub expanded: bool,
}

// render_cache.rs
fn render_code_block_fenced(
    content: &str,
    lang: &str,
    max_lines: usize,
    expanded: bool,
    theme: &Theme,
) -> Vec<Line<'static>> {
    let lines = content.lines().collect::<Vec<_>>();
    let total = lines.len();
    
    if expanded || total <= max_lines {
        // 全量渲染
    } else {
        // 折叠渲染：首 3 行 + 展开提示
        let mut result = render_first_3_lines(&lines, lang, theme);
        result.push(Line::from(Span::styled(
            format!("  ▼ 展开 (共 {} 行)", total),
            Style::default().fg(theme.text_dim).add_modifier(Modifier::ITALIC),
        )));
        result
    }
}
```

#### 2.2.2 工具结果「查看完整」入口

**改动范围：** `render_cache.rs`, `handler/browse.rs`

**方案：**
- 工具结果超过限制时，底部显示 `(查看完整内容)` 可点击提示
- Browse 模式选中工具结果消息时，按 `v` 键打开外部编辑器/pager 查看完整内容
- 完整内容临时写入 `/tmp/j-tool-result-{id}.txt`，用 `bat` 或 `less` 打开

---

### 2.3 工具消息聚合显示

#### 2.3.1 Tool Call + Tool Result 合并渲染

**改动范围：** `render_cache.rs`

**方案：**
- 相邻的 `tool_call_request` + `tool_result` 消息合并为一个视觉单元
- 单元顶部显示工具名 + 参数摘要 + 状态图标
- 展开模式下内部显示完整参数 + 结果

**视觉结构：**
```
┌─ 🔧 Bash ─────────────────────────────── ✅ ─┐
│  $ ls -la                                    │  ← 折叠模式：命令 + 状态
└──────────────────────────────────────────────┘

┌─ 🔧 Bash ─────────────────────────────── ✅ ─┐
│  命令:                                       │
│    $ ls -la                                  │  ← 展开模式
│  输出 (32行):                                │
│    total 48                                  │
│    drwxr-xr-x ...                            │
│    ...                                       │
│  ▼ 展开完整输出                               │
└──────────────────────────────────────────────┘
```

**实现要点：**
- `build_message_lines_incremental` 中识别相邻 tool_call/tool_result 配对
- 引入 `ToolPairCache` 结构聚合渲染

---

### 2.4 消息导航增强

#### 2.4.1 Browse 模式快捷键扩展

**改动范围：** `handler/browse.rs`, `ui/hint.rs`

**新增快捷键：**

| 按键 | 功能 |
|-----|------|
| `g` | 跳转到首条消息 |
| `G` | 跳转到最后一条消息 |
| `/` | 进入搜索模式（模糊匹配消息内容） |
| `n` | 下一个搜索匹配 |
| `N` | 上一个搜索匹配 |
| `v` | 查看当前消息完整内容（外部 pager） |
| `e` | 展开/折叠当前消息所有代码块 |

#### 2.4.2 消息搜索功能

**改动范围：** `app/ui_state.rs`, `handler/browse.rs`

**方案：**
- Browse 模式按 `/` 进入搜索子模式，底部显示搜索输入框
- 实时高亮匹配消息，`n/N` 在匹配结果间跳转
- 搜索范围：消息 content + tool_name + tool_arguments

---

### 2.5 流式响应进度指示

**改动范围：** `render_cache.rs`

**方案：**
- 流式响应期间显示已接收字节数 + 耗时
- 使用思考指示器动画区域展示进度

```
Sprite
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
◍ 正在思考... (已接收 1.2KB, 耗时 3s)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

---

## 三、实施计划

### Phase 1: 基础视觉增强 (优先级 P0)

| 任务 | 预估工时 | 改动文件 |
|-----|---------|---------|
| 消息时间戳显示 | 2h | `storage.rs`, `render_cache.rs` |
| 消息角色图标 | 1h | `render_cache.rs` |
| 工具结果状态图标区分 | 1h | `render_cache.rs`, `classification.rs` |

### Phase 2: 长消息交互 (优先级 P1)

| 任务 | 预估工时 | 改动文件 |
|-----|---------|---------|
| 代码块折叠/展开 | 4h | `ui_state.rs`, `render_cache.rs`, `browse.rs` |
| 工具结果完整查看 | 2h | `render_cache.rs`, `browse.rs` |

### Phase 3: 工具消息聚合 (优先级 P2)

| 任务 | 预估工时 | 改动文件 |
|-----|---------|---------|
| Tool Call + Result 合并渲染 | 3h | `render_cache.rs` |

### Phase 4: 消息导航增强 (优先级 P2)

| 任务 | 预估工时 | 改动文件 |
|-----|---------|---------|
| Browse 快捷键扩展 | 2h | `browse.rs`, `hint.rs` |
| 消息搜索功能 | 4h | `ui_state.rs`, `browse.rs` |

### Phase 5: 流式进度指示 (优先级 P3)

| 任务 | 预估工时 | 改动文件 |
|-----|---------|---------|
| 流式响应进度显示 | 2h | `render_cache.rs` |

---

## 四、风险与注意事项

1. **缓存一致性**
   - 新增状态（折叠、搜索）需正确同步到 `MsgLinesCache`
   - 状态变更时需标记 `msg_lines_cache = None` 触发重建

2. **性能影响**
   - 消息聚合渲染需避免 O(n²) 配对搜索
   - 时间戳计算需缓存结果，避免每帧重新计算

3. **向后兼容**
   - `Message` 结构体新增 `timestamp` 字段需处理旧数据缺失情况
   - 旧 session 文件加载时自动填充默认时间戳

4. **终端兼容性**
   - 外部 pager 调用需正确处理终端 raw mode 切换
   - 参考 `tui_loop.rs` 中 markdown 编辑器的暂停/恢复模式

---

## 五、建议实施顺序

基于用户价值与实现复杂度，建议按以下顺序实施：

1. **消息时间戳** — 用户高频需求，实现简单
2. **工具结果状态图标区分** — 即时视觉反馈，实现简单
3. **Browse 快捷键扩展 (g/G)** — 导航效率提升，实现简单
4. **代码块折叠/展开** — 长消息体验核心改进
5. **工具消息聚合** — 视觉整洁度提升
6. **消息搜索** — 高级导航需求
7. **流式进度指示** — 锦上添花

---

## 六、用户确认项

请确认以下问题后开始实施：

1. **时间戳格式偏好？**
   - A: 相对时间（如「2分钟前」）
   - B: 绝对时间（如「14:32」）
   - C: 混合（近期用相对，超过 1 小时用绝对）

2. **代码块折叠触发阈值？**
   - A: 超过 10 行折叠
   - B: 超过 20 行折叠
   - C: 用户可配置

3. **工具消息聚合是否需要？**
   - A: 是，合并显示更整洁
   - B: 否，保持独立显示

4. **优先实施哪些功能？**
   - 请从 Phase 1-5 中选择优先项