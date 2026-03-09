# 关键设计决策

> 本文档记录 `j` 项目的重要设计决策和技术选型。

---

## 1. clap try_parse + fallback

Java 版手动 split 命令字符串，Rust 版利用 `Cli::try_parse()` 尝试解析：

- 成功 → 匹配到子命令 → dispatch
- 失败 → 不是内置命令 → 作为别名打开

这使得 `j chrome` 虽然不是子命令，但能正确 fallback 到别名查找。

---

## 2. 配置文件直接 serde 序列化

Java 版用 `commons-configuration2` 逐 key 读写。Rust 版将整个 YAML 结构映射为 `YamlConfig` struct，任何修改直接序列化整个结构写回文件。

**优点**：简单可靠，避免部分更新导致的不一致。

---

## 3. 交互模式命令解析独立于 clap

交互模式不走 `Cli::try_parse()`（需要完整的 argv），而是自己实现了 `parse_interactive_command()` 函数，将输入行 split 后手动匹配到 `SubCmd` 枚举。

**优点**：共享同一套 dispatch 逻辑。

---

## 4. UTF-8 安全的模糊匹配

`fuzzy.rs` 中的 `get_match_intervals()` 使用 `char_indices()` 映射确保切片始终在 char boundary 上，避免中文等多字节字符导致 panic。

---

## 5. 全局常量集中管理

`constants.rs` 统一维护所有魔法字符串，任何新增的 section、配置 key、版本号等应先在 `constants.rs` 中定义，再在各模块中引用。

---

## 6. CLI 工具智能识别

`open.rs` 中的 `is_cli_executable()` 函数自动判断 path 别名指向的是 CLI 可执行文件还是 GUI 应用：

- **CLI 可执行文件** → `Command::new()` 在当前终端执行，支持管道
- **GUI 应用**（`.app`）→ 系统 `open` 命令打开新窗口

**判断规则**：
1. URL（http/https 开头）→ 非 CLI
2. `.app` 结尾或包含 `.app/` → macOS GUI 应用
3. 文件存在 + 普通文件 + 可执行权限 → CLI 工具

---

## 7. 日报系统默认路径 + git 远程同步

**默认路径机制**：
- 日报文件默认存储在 `~/.jdata/report/week_report.md`
- 首次使用自动创建目录和文件
- 支持自定义路径（优先级高于默认）

**git 远程同步**：
- `reportctl push`：自动 git add + commit + push
- `reportctl pull`：智能判断三种场景（clone/fetch+reset/pull --rebase）
- 自动同步 remote origin URL

---

## 8. 交互模式三态命令解析

`parse_interactive_command()` 返回三态枚举 `ParseResult`：

```rust
enum ParseResult {
  Matched(SubCmd),  // 成功解析为内置命令
  Handled,          // 是内置命令但参数不足，已打印 usage
  NotFound,         // 不是内置命令 → fallback 到别名查找
}
```

解决了原来 `None` 一值两义导致的 bug。

---

## 9. Markdown 终端渲染（嵌入二进制 + fallback）

**渲染引擎**：`ask`（Go 编写，基于 `go-term-markdown`）

**嵌入策略**：
- 编译时通过 `include_bytes!` 嵌入二进制
- 首次调用 `md!` 宏时自动释放到 `~/.jdata/bin/ask`
- 通过文件大小校验版本

**渲染策略（两级 fallback）**：
- 优先：调用嵌入的 `ask` 二进制
- fallback：非 macOS ARM64 平台退化到 `termimad` crate

---

## 10. 交互模式历史隐私保护

`auto_add_history` 改为 `false`，手动控制历史记录：

- `report <content>` 命令**不记入历史**——日报内容属于隐私
- 其他所有命令正常记录历史

---

## 11. TUI 多行编辑器（vim 模式）

基于 ratatui + tui-textarea 的全屏编辑器：

**vim 模式支持**：
- NORMAL 模式：默认进入，支持 hjkl/w/e/b/gg/G/yy/dd/cc/u/Ctrl+R
- INSERT 模式：i/a/o/O 进入
- VISUAL 模式：v 进入选择
- COMMAND 模式：`:wq`/`:x`/`:q`/`:q!`
- SEARCH 模式：`/pattern` 搜索，`n`/`N` 跳转

**日报 TUI 特性**：
- 自动预加载最后 3 行历史上下文
- 自动预填日期前缀行
- 提交后原样写入文件

---

## 12. 脚本新窗口执行（`-w` 标志）

**跨平台实现**：
- macOS：`osascript` + AppleScript
- Windows：`start cmd /c`
- Linux：`gnome-terminal`/`xterm`/`konsole` fallback

**设计要点**：
- `-w` 标志从参数列表中过滤后再传递给脚本
- 新窗口执行是非阻塞的
- 不强制追加等待按键逻辑

---

## 13. 文件路径补全增强

**交互模式**：根据别名类型智能选择补全策略

| 别名类型 | 后续参数补全 |
|----------|-------------|
| 编辑器别名 | 文件路径 |
| 浏览器别名 | 别名 + 文件路径 |
| 其他别名 | 文件路径 + 别名 |

**快捷模式**：`j completion [zsh|bash]` 动态生成补全脚本

---

## 14. 脚本环境变量注入

**命名规则**：`J_<别名大写>`，`-` 转 `_`

**注入方式**：

| 场景 | 注入方式 |
|------|----------|
| 当前终端脚本执行 | `Command::env()` |
| 新窗口脚本执行 | `export` 语句拼接 |
| 交互模式 shell 命令 | `Command::env()` + 进程级 `set_var` |

**注意**：路径含空格时，脚本中必须用双引号包裹变量。

---

## 15. AI 对话 Agent 循环

支持多轮工具调用，最大 10 轮（可通过 `max_tool_rounds` 配置）：

1. 构建 API 请求
2. 流式/整体接收响应
3. 收到 tool_calls → 暂停，等待用户确认
4. 执行工具 → 将结果加入上下文
5. 继续 API 请求

**配置项**：
- `max_tool_rounds`：最大工具调用轮数（默认 10）
- `tool_confirm_timeout`：工具确认超时秒数（默认 0，禁用自动执行）

**兼容性处理**：
- 流式响应中 tool_calls 反序列化失败时，自动 fallback 到非流式请求（适配 Gemini 等平台）

---

## 16. Skill 技能系统

**存储位置**：`~/.jdata/agent/skills/<skill_name>/SKILL.md`

**懒加载策略**：
- 系统提示词中仅包含技能名称和描述摘要
- AI 判断需要时调用 `load_skill` 加载完整指令
- 用户也可用 `@skill_name` 提示

**模板占位符**：
- `{{.skills}}` → 技能摘要
- `{{.tools}}` → 工具摘要
- `{{.style}}` → 回复风格
- `{{.memory}}` → 记忆内容
- `{{.soul}}` → 灵魂/人格设定

**引用文件支持**：
- 技能目录下的 `references/` 子目录会被自动加载
- 所有文件内容会被拼接追加到技能正文后

---

## 17. 帮助文档 Tab 分配策略

`j help` 命令使用 TUI 界面展示帮助内容，通过 `## ` 标题将 `assets/help.md` 分配到不同 Tab 页。

**实现机制**：

```rust
// src/command/help/app.rs
const TAB_DEFS: &[TabDef] = &[
    TabDef {
        name: "快速上手",
        heading_keywords: &["快速上手"],
    },
    TabDef {
        name: "别名 & 打开",
        heading_keywords: &["别名管理", "分类标记", "列表", "打开"],
    },
    // ...
];
```

**分配逻辑**：
1. 按 `## ` 标题将 `help.md` 切分为多个 section
2. 遍历每个 section，检查标题是否包含某 Tab 的 `heading_keywords`
3. 匹配上则将该 section 内容追加到对应 Tab

**设计要点**：
- 一个 Tab 可匹配多个 section（如 "别名 & 打开" 匹配 4 个 section）
- 一个 section 只分配到一个 Tab（首次匹配优先）
- 关键词应选择标题中的唯一标识词，避免歧义
- 新增 `## ` section 时需同步更新 `TAB_DEFS`
