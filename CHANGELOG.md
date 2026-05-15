# v12.10.60


### 新功能

- **帮助页面滚动**: Chat TUI 帮助页面现支持键盘滚动（↑/↓/j/k/PageUp/PageDown），方便查看较长内容
- **帮助页面鼠标选区**: 帮助页面支持鼠标选择文本，选中后按 `c` 可复制到剪贴板

### 改进

- **选区复制体验优化**: 统一了帮助模式和聊天模式的选区复制逻辑，有选区时按 `c` 即可复制
- **帮助页面渲染缓存**: 添加帮助内容缓存机制，提升选区操作性能
</result

# v12.10.59


### 新功能

- **帮助页目录树导航**: 帮助页面从平铺 Tab 布局改为左侧目录树 + 右侧内容预览的双栏布局，支持展开/折叠目录、鼠标滚轮滚动、拖拽调整面板宽度
- **帮助页鼠标选区复制**: 右侧内容区支持鼠标拖拽选择文字，Ctrl+C 复制选中内容到剪贴板
- **帮助页默认展开所有目录**: 进入帮助页时自动展开全部目录，无需逐个点击展开
- **标题栏多行换行**: Teammate/SubAgent 状态栏支持多行自动换行显示，不再截断，按行宽自动排列

### 改进

- **代码块渲染撑满宽度并自动折行**: 代码块围栏宽度从按内容最大宽度对齐改为撑满可用屏幕宽度，超长代码行自动折行而非截断（IR 渲染器和 Editor 渲染器均已更新）
- **引用块样式统一**: 引用块（blockquote）渲染风格与 thinking block 对齐，背景色改为主背景色，竖线样式从 `▎` 改为 `|`，增加前导缩进
- **帮助文档按目录重组**: 帮助文档从平铺单文件拆分为按主题分组的目录结构（别名/对话/日报/待办/笔记/脚本/钩子/工具/命令），每个文件聚焦单一主题，更易于浏览
- **Chat 帮助弹窗布局优化**: 移除边框组件，改用内边距布局，减少视觉噪音
- **帮助页启用鼠标捕获**: 启用鼠标事件监听，支持点击选择、拖拽选区、滚轮滚动等交互
</result

# v12.10.58


### 新功能
- **文件加密文档**: 新增 `j lock` / `j unlock` 文件加密功能的帮助文档（中英文），涵盖 AES-256-GCM 加密原理、命令用法、批量操作及注意事项

### 改进
- **代码块渲染**: 代码块围栏宽度改为根据代码内容动态计算，不再固定占满整行；边框和背景统一使用 `text_dim` / `bg_primary` 主题色，移除独立的 `code_border` / `code_bg` 配色
- **超时计算**: Background Task 的轮询超时逻辑从 `Instant::now() + Duration` 改为 `start.elapsed() >= timeout`，避免循环中

# v12.10.57


### Bug 修复
- **安装脚本错误处理**: 修复 PowerShell 和 Shell 安装脚本中 `exit 1` 导致的非预期退出问题，改为使用 `return` 优雅退出函数
- **Windows 重复渲染**: 在 `chat.rs` 和 `config.rs` 中添加 `Clear` widget，解决 Windows 上 crossterm 差异缓冲区不清理旧内容导致的界面重复渲染问题

### 改进
- **安装脚本版本回退**: 移除安装脚本的内置版本回退机制（`FALLBACK_VERSION`），所有网络获取失败时直接报错并提示用户指定版本安装
</result

# v12.10.56


### 改进
- **更新备用方案**: 更新失败时的备用方案从仅支持 curl 扩展为 Windows PowerShell 方案，解决 Windows 上 rustls 导致的 TLS/证书/连接错误；同时扩展错误识别范围，覆盖 decoding、IoError、certificate、ssl、TLS、connection 等多种失败场景
- **下载验证**: 备用下载完成后增加文件存在性和非空校验，避免空文件或丢失文件导致后续解压失败
- **欢迎页 ASCII Art**: 修正 J-CLI ASCII Art 字符对齐，调整间距使 logo 显示更规整

### Bug 修复
- **Windows 更新失败**: 修复 Windows 平台因 rustls TLS 兼容性问题导致 `j update` 无法正常更新的问题，改用系统原生 TLS 的 PowerShell Invoke-WebRequest 作为下载后备方案

# v12.10.55


### 新功能

- **文件索引缓存系统 (FileIndex)**: 新增后台文件索引模块，使用 `notify` crate 监控文件变化并维护内存路径缓存。`@` 弹窗和文件弹窗从每帧 WalkBuilder 扫描改为内存模糊搜索，大幅提升响应速度
- **欢迎界面可切换模式**: 新增 `welcome_quote` 配置项，允许在欢迎界面切换诗句引言和 J-CLI ASCII Art 两种显示风格，可在全局设置面板中开关
- **带行号文本智能换行**: 工具结果渲染器现在能检测带行号前缀的文本（如 Read 工具输出），续行自动保留 `│` 符号对齐，新增 `wrap_text_with_prefix`、`line_number_prefix_width`、`line_number_continuation_prefix` 等工具函数
- **j-agent 模板资源外置**: 将系统提示词、teammate/sub-agent prompt、记忆/灵魂模板等从内联字符串提取为独立 `.md` 文件，放置于 `j-agent/assets/` 目录，便于维护和定制
- **Makefile `commit` 命令**: 新增 `make commit` 伪目标，自动基于变更文件生成 commit message 并提交，无需 AI 参与
- **PPT 一键导出工具链**: 新增 `make ppt-build`/`ppt-render`/`ppt-deps`/`ppt-clean` 命令，支持将 HTML 演示文稿渲染为高清 PNG 并打包为 .pptx 文件

### 改进

- **j-agent 模块结构扁平化**: 将 `context/mod.rs`、`infra/mod.rs`、`permission/mod.rs`、`storage/mod.rs`、`teammate/mod.rs`、`tools/mod.rs` 统一重命名为对应的 `.rs` 文件，遵循弃用 `mod.rs` 的项目规范
- **版本号同步管理**: `bump-version` 和 `set-version` 目标现在同步更新 j-cli 和 j-agent 两个 crate 的版本号，确保版本一致性
- **j-agent 独立版本号**: j-agent 版本号从 `0.1.0` 独立演进，与主项目保持同步
- **欢迎页 ASCII Art 微调**: 修正了启动界面 J-CLI ASCII Art 的字符对齐问题

### Bug 修复

- **文件弹窗性能问题**: 修复 `@` 弹窗和 `file:` 弹窗每帧触发 WalkBuilder 全目录扫描导致的卡顿，改为后台缓存 + 内存过滤

# v12.10.54


### Bug 修复
- **Windows CI 编译**: 修复 `j-agent` 中 `core-graphics` 和 `nix` 被无条件声明为依赖导致 Windows CI 编译失败的问题，改为平台条件依赖（`cfg(unix)` / `cfg(target_os = "macos")`）
</result

# v12.10.53


### 改进

- **PPT 答辩演示文稿**: 新增 Function Calling 原理讲解页面（3.5 节），含 LLM 输入结构、Function Calling 四步流程图和代码示例
- **PPT 答辩演示文稿**: 重写命令执行三态页面，新增真实 npm create 卡死案例作为问题背景，增强叙事说服力
- **PPT 答辩演示文稿**: 合并 TodoWrite + system-reminder 两页为单页，聚焦 system-reminder 状态感知注入的两种机制（占位符替换 + 动态消息注入）
- **PPT 答辩演示文稿**: 移除"窗口膨胀现象"页面和 Hook 系统事件页面，精简内容结构
- **PPT 答辩演示文稿**: 总结页从四列卡片布局改为两列布局，标题从"四大贡献"改为"四项主要工作"
- **PPT 答辩演示文稿**: 多处页面排版微调——精简列表文案、调整间距与图片裁切方式、Grep 工具描述更新为"模板匹配上下文"
- **PNG 渲染脚本**: 渲染分辨率从 1280x720@2x 调整为 1920x1080@1.33x，匹配 html-ppt skill 的设计尺寸
- **README**: 修正 j-gui 仓库链接为正确地址
</result

# v12.10.52


### 新功能
- **文件索引缓存**: 引入 FileIndex 后台缓存机制，替代 @ 弹窗和文件弹窗每帧 WalkBuilder 扫描，大幅提升文件搜索性能；支持 notify 文件监控自动刷新
- **HTML PPT Skill**: 新增 html-ppt skill，包含 36 个主题、20+ 动画效果、15 个完整 deck 模板和 30+ 单页模板，支持一键生成专业 HTML 演示文稿
- **欢迎诗句配置**: 新增 `welcome_quote` 全局配置项，关闭时显示 J-CLI ASCII Art 渐变色 Logo
- **PPT 导出命令**: Makefile 新增 `ppt-serve`、`ppt-build`、`ppt-render` 等命令，支持 HTML PPT 一键导出为 .pptx（图片版）

### 改进
- **工具结果渲染**: 带行号的文本（如 Read 输出）自动检测，续行保留 │ 符号实现视觉对齐
- **oneshot Markdown 重绘**: 用光标位置保存替代行数计算，流式输出重绘更精确
- **search 命令**: 支持多词搜索，`-f/--fuzzy` 标志位置更灵活
- **浏览器命令**: 支持多词搜索、`--engine` 标志指定搜索引擎
- **笔记保存**: 新文件始终保存，自动创建父目录
- **Windows 安装脚本**: 自动关闭占用进程，重命名策略处理文件占用
- **Windows 更新**: 使用 tar 替代 PowerShell 解压 zip（解决 Deflate64 兼容问题），旧版本重命名备份 + 延迟清理
- **Release workflow**: Windows zip 使用 7z 创建，确保标准 Deflate 压缩
- **j-agent 资源整理**: 模板文件统一迁移至 j-agent/assets 目录
</

# v12.10.51


### 新功能

- **浏览器搜索支持 --engine 参数**: `j open <browser_alias> <keywords>` 支持 `--engine <google|bing|baidu>` 指定搜索引擎，多个关键词自动拼接为搜索词，同时保留 URL 别名和直接 URL 的优先匹配
- **核心引擎拆分为独立 crate (j-agent)**: 将 Chat 核心（Agent、LLM 客户端、工具系统、权限管理、上下文管理等）提取为独立的 `j-agent` 库，不依赖 ratatui/crossterm，为后续 GUI（Tauri）复用奠定基础
- **HTML PPT 技能包**: 新增 `html-ppt` skill，包含 36 套主题、20 种动画特效、30+ 页面模板、14 套完整演示文稿模板，支持 presenter 模式和键盘导航

### 改进

- **搜索命令参数优化**: `j search` 支持多关键词搜索（空格分隔自动拼接），`-f`/`--fuzzy` 改为标准 flag 参数
- **Oneshot 模式 Markdown 重绘重构**: 用终端光标行号（`crossterm::cursor::position`）替代手动行计数实现流式文本的 Markdown 重绘，消除自动换行计算偏差导致的显示错位
- **Windows 更新安装可靠性增强**: 安装脚本（install.ps1）和 `j update` 均支持 exe 被占用时的重命名策略（先重命名为 `.bak` 再替换），安装失败时自动恢复备份
- **Windows zip 解压策略**: `j update` 优先使用系统自带 `tar` 命令解压，回退到 PowerShell `Expand-Archive`，兼容 7z 创建的 zip 格式
- **self_update 依赖特性**: Windows 平台新增 `compression-zip-deflate` feature，提升 zip 解压兼容性
- **ToolCategory/ToolStatus 颜色解耦**: 工具分类和状态的颜色方法通过扩展 trait（`ToolCategoryColor`/`ToolStatusColor`）注入，核心库不再依赖 ratatui
- **Hook 帮助文档注入机制**: RegisterHookTool 的帮助内容改用 `OnceLock` + 注入函数，核心库不再依赖 `rust-embed` 资源系统

### Bug 修复

- **新笔记编辑后即使内容未变也强制保存**: 修复新建笔记在编辑器中保存时，因"内容未变化"判断导致文件未被写入的问题；同时自动创建不存在的父目录
- **Windows 更新失败时手动安装提示**: 错误提示区分 Unix/Windows 平台，分别显示 `curl` 和 `irm` 安装命令
- **Windows 卸载时处理进程占用**: 卸载脚本先尝试关闭 `j.exe` 进程，文件被占用时使用重命名策略替代直接删除
</result

# v12.10.48


### 改进

- **发布流程**: Makefile 的 `publish` 和 `publish-check` 目标现在支持先发布 `j-agent` 子包到 crates.io，再发布主包 `j-cli`，确保依赖顺序正确
- **依赖声明**: `j-agent` 在主包 `Cargo.toml` 中补充了 `version = "0.1.0"` 字段，满足 crates.io 发布要求

### 内部变更

- **版本号**: 从 `12.10.47` 升级至 `12.10.48`
</result

# v12.10.47


### 新功能

- **文件加密解密**: 新增 `j lock` / `j unlock` 命令，使用 AES-256-GCM 对文件进行对称加密，支持单文件或目录批量加密
- **j-agent 核心库**: 创建独立 workspace crate，将 chat 引擎核心模块（agent、storage、tools、infra 等）从 j-cli 抽离，消除代码重复
- **Session 工具**: 新增交互式进程会话工具，支持 stdin 写入、stdout 读取、quit 终止，配合 Bash/Powershell 的 interactive 模式
- **RegisterHook 工具**: LLM 可动态注册/管理 session 级 hook，支持 bash（shell 命令）和 llm（prompt 模板）两种类型
- **Shell 交互模式**: Bash 工具新增 interactive 参数，启动 PTY 子进程并返回 sid，支持 REPL/ssh/mysql 等交互式程序
- **远程控制面板**: 大幅增强远程 Web UI，新增配置管理（模型/会话/全局/工具/技能/Hooks）、文件浏览器、浏览器自动化面板、终端面板、归档管理、侧边栏导航等组件
- **BackgroundManager 增强**: 支持线程类任务（is_thread_running）、PTY writer、adopt_process 接管已运行进程

### 改进

- **远程协议扩展**: SessionSync 增加 context_tokens、message_count、auto_approve 字段；新增 ModelList/ThemeList/ConfigState 等广播消息
- **Hook 系统**: HookKind 扩展为 Shell/Llm/Builtin 三种类型，支持 prompt 模板调用 LLM API
- **代码架构**: j-cli 通过 re-export j-agent 模块消除重复，保持 TUI 依赖分离

### Bug 修复

- **Clippy 清理**: 清除所有 clippy 警告，j-cli 和 j-agent 均达到 0 error 0 warning
- **空测试声明**: 移除 retry/tool_processor/chat_error 中空的 `mod tests` 声明
- **编译修复**: browser_cdp 模块编译问题修复

### 重构

- **命名规范**: j-cli-core 重命名为 j-agent，更准确反映其定位
- **文档清理**: 删除过时的重构文档，格式化 j-agent 代码
</result

# v12.10.44


### 新功能

- **文件加密解密命令**: 新增 `j lock` 和 `j unlock` 命令，使用 AES-256-GCM 对称加密文件内容，支持单文件或目录批量加密/解密
- **Session 工具**: 新增交互式会话管理工具，支持 PTY 进程的 stdin 写入、stdout 读取、quit 终止操作，适用于 REPL、SSH、mysql 等交互式程序
- **LoadTool 延迟加载机制**: 新增工具延迟加载功能，默认将 Task、RegisterHook、ComputerUse、Browser 设为 deferred 状态，模型可按需动态加载
- **Shell 工具 interactive 模式**: Bash 工具新增 `interactive` 参数，可启动 PTY 交互式会话并返回 sid，配合 Session 工具使用

### 改进

- **后台任务与交互式会话分离**: 区分后台任务和交互式会话，生成独立的上下文摘要，避免误用 TaskOutput 操作交互式进程
- **远程 UI 风格优化**: 重构 remote UI 为 Inter + stone 色系简约风格
- **远程配置管理**: 远程界面支持配置管理、归档操作与权限审批功能
</result

# v12.10.41



### 新功能

- **LoadTool 会话级持久化**: LoadTool 加载的 deferred 工具状态现在按会话持久化保存，会话恢复后自动还原已加载的工具，不再丢失运行时状态
- **工具描述公共缩进去除**: 工具描述文本自动移除多余的公共缩进，使发送给 LLM 的描述信息更整洁

### 改进

- **工具配置页左右分栏布局**: Tools 配置页改为左侧工具列表 + 右侧详情面板的分栏布局，选中工具的启用/defer 选项在右侧面板显示，信息层次更清晰
- **defer 状态区分配置与运行时**: 工具列表中 defer 标签现在区分「defer」（未加载）和「defer·已加载」（本会话已通过 LoadTool 加载）两种状态，便于用户了解工具实际可用性
- **配置变更双写同步**: 在 UI 中修改工具的 deferred 状态时同步更新 agent_config（持久化）和 deferred_tools（运行时），保存时不再从运行时状态回写配置，避免会话级 LoadTool 操作被意外持久化

### Bug 修复

- **Makefile commit 提取**: 修复 `

# v12.10.40


### 改进

- **ToolRegistry 清理**: 移除 `ToolRegistry` 中冗余的 `deferred_tools` 字段及 `set_deferred_tools()` 方法，deferred 工具管理已由 `LoadTool` 和 `Arc<Mutex<Vec<String>>>` 共享引用独立承担，无需在 registry 层重复存储
- **子 agent 行为明确化**: 补充子 agent 不支持动态 LoadTool 的设计说明，子 agent 仅继承父 agent 的 deferred 快照作为初始工具过滤，避免后续维护者误判为缺失功能
</result

# v12.10.39


### 新功能
- **Deferred Tools**: 新增工具延迟加载机制，允许将工具标记为 "deferred" 状态，需通过 LoadTool 显式加载后才对 Agent 可用；Tools Tab UI 改造为层级导航模式，支持 Tab 键在工具列表层级与选项层级（启用/defer）之间切换

### 改进
- **Markdown 折行**: 新增前缀感知的 span 折行模块，修复长标题、长列表项折行后续行缩进对齐问题
- **ANSI 清洗**: 改进终端文本清洗，剥离完整 ANSI/OSC 序列而非仅删除 ESC 字节，避免渲染时泄漏 `[31m` 等残片
- **oneshot 确认框**: 边框改为左右闭合样式（右侧加 `│`），交互框宽度从 20-56 扩展至 40-80，新增 Ctrl+C 退出支持

### Bug 修复
- **oneshot 消息丢失**: 修复 agent loop 在 break 'round 前未将 streaming 内容刷新到 context_messages，导致 oneshot persist 时丢失最终 AI 回复的问题
</result

# v12.10.38


  ### Bug 修复
  
  - **no_render 模式输出纯净化**: 在 `--no-render` 模式下跳过缩进和 AI 标签输出，避免重定向到文件时内容被多余格式污染
  
  ### 改进
  
  - **Ctrl+C 中断机制优化**: 重构 oneshot agent 模式的 Ctrl+C 处理逻辑，移除"二次 Ctrl+C 强制退出进程"的行为，改为优雅退回 REPL，不再直接 `process::exit(130)` 杀死进程
  - **工具执行支持中断**: 工具调用在子线程中执行，主线程轮询中断标志，用户按 Ctrl+C 可立即中断等待中的工具调用并退回 REPL
  - **交互式确认支持中断**: 工具确认弹窗和多选/单选界面中按 Ctrl+C 不再强制退出，改为清理已绘制内容后返回
  - **Makefile push 增强**: `make push` 在无本地变更时直接 push 已有 commits，而非报错退出
  - **Makefile 模板替换重构**: 将 `sed` 替换模板变量的方式替换为 `awk`，避免 shell 变量中特殊字符导致的问题
  </result

# v12.10.37


  ### 改进
  
  - **oneshot 交互框边框闭合**: 所有边框绘制函数（顶部、底部、内容行、空行、提示行、选项行）统一使用 `bw` 参数计算宽度，左右两侧 `│` 完全闭合对齐，解决之前边框右侧缺失的问题
  - **oneshot 交互框宽度适配**: 交互框宽度范围从 20-56 调整为 40-80，更好地利用终端空间
  - **oneshot AI 回复缩进**: Sprite 回复文本增加 2 空格缩进，每行换行后自动补缩进，视觉层次更清晰
  
  ### Bug 修复
  
  - **Ctrl+C 双次中断机制**: oneshot 模式下第一次 Ctrl+C 执行优雅取消并提示"清理中...再按 Ctrl+C 强制退出"，第二次 Ctrl+C 强制退出进程（之前仅设置取消标志，无退出提示，且无法强制退出）
  - **Ctrl+C 退出终端恢复**: 强制退出时主动调用 `disable_raw_mode()` 恢复终端状态，避免终端残留 raw mode 导致输入异常
  - **交互框 Ctrl+C 支持**: 单选、多选、工具确认等交互框内新增 Ctrl+C 处理，退出时恢复终端并显示中断提示
  </result

会话 ID: 651511ed86724-172db

# v12.10.36


### 改进

- **Makefile 重构**: 将内嵌的 AI prompt 模板提取为独立文件 (prompts/commit-message.md, prompts/release-notes.md)，提高可维护性和可读性

# v12.10.35


### 改进
- **AGENT.md 重命名为 AGENTS.md**: 将项目级指令文件从 `AGENT.md` 统一更名为 `AGENTS.md`（含 `.local` 变体），涉及搜索加载逻辑、帮助文档、系统提示词模板、lint 脚本及 TUI 配置界面等全链路更新
- **Makefile push 智能提示增强**: AI 生成 commit message 的 prompt 中新增"必须查看具体 diff 判断变更内容"的行为规则，避免 AI 仅凭文件名或 commit message 猜测变更
- **Makefile publish 输出修复**: 将 `j ai` 命令输出

# v12.10.32



### Bug 修复

- **publish AI 输出**: 修复 `make publish` 时 AI 生成的 release notes 内容不可见的问题，改用 `tee` 同时输出到终端和文件

# v12.10.31


### Bug 修复
- **publish 命令**: 修复 AI 生成 release notes 时输出不可见的问题，改用 `tee` 同时输出到终端和文件
</result

# v12.10.30


### 新功能
- **make publish**: 支持 AI 自动生成 release notes，基于 git log 自动提取变更摘要

### 改进
- **Makefile install**: 重构为从本地构建安装，提升开发迭代效率
- **AI 响应提取**: 改用 awk 解析 result 标签，提升稳定性
- **oneshot prompt**: 添加调试日志，便于排查 Makefile 传参问题

### Bug 修复
- **oneshot 消息持久化**: 修复 AI 回复丢失问题，确保流式输出结束后正确持久化到 context_messages
- **oneshot 空会话问题**: 修复 LLM 无响应或调用失败时用户消息丢失的问题，现在会正确保留用户输入记录
- **文本截断**: 修复 UTF-8 多字节字符截断导致的 panic，改用字符边界安全截断

# v12.10.28


### 改进
- **调试支持**: 为 push 和 publish 命令添加 AI 原始输出调试打印，便于排查 AI 交互问题
</result

# v12.10.27


### 新功能
- **make publish**: 支持 AI 自动生成 release notes

### 改进
- **AI 响应提取**: 优化解析逻辑，改用 awk 解析 result 标签
- **Makefile install**: 重构为本地构建安装

# v12.10.25

### Bug 修复
- **Markdown 长标题折行修复**: 修复标题文本超出终端宽度折行时丢失前缀符号和续行缩进的问题，现在折行后正确保留列表标记（如 `- `、`> `）和缩进对齐

### 改进
- **Markdown 渲染模块重构**: 提取通用换行逻辑到独立 wrap 模块，新增 wrap_with_prefix / wrap_preserve_prefix 等工具函数，减少 block.rs 中重复代码
- **全局配置绘制重构**: 将全局配置页面绘制逻辑拆分为独立子列表函数，改善代码组织和可读性
- **Makefile push 目标优化**: push-ai 作为默认 push 行为，AI prompt 构建改用临时文件传递，避免命令行参数长度限制

# v12.10.23

### Bug 修复
- **修复工具确认界面状态残留**: 拒绝/执行/允许并执行工具后，UI 状态（选中项、输入框内容、光标位置等）未重置，导致处理下一个待确认工具时显示异常

### 改进
- **安装脚本版本获取逻辑优化**: install.sh 和 install.ps1 改用跟随 releases/latest 重定向提取版本号，替代从页面内容解析，更可靠且避免 HTML 结构变化导致的解析失败
- **安装脚本 fallback 版本更新**: 内置 fallback 版本号更新至 v12.10.22

# v12.10.22

### 改进
- **文本清洗体系重构**: 新增 `sanitize_terminal_text()` / `sanitize_single_line_text()` / `needs_terminal_sanitization()` 三层 API，完整剥离 ANSI/OSC 转义序列与控制字符，替代原有的 `normalize_terminal_text` 逐字符替换方案
- **wrap_text 防 ANSI 残片**: `wrap_text()` 现在先剥离 ANSI 转义序列再换行，避免 `[31m` / `[0m` 等残片泄漏到 TUI 渲染结果中
- **Markdown 解析预处理增强**: Markdown 解析器预处理从 `normalize_terminal_text` 升级为 `sanitize_terminal_text`，增加对 ANSI 转义序列的完整剥离

### Bug 修复
- **TUI 渲染安全加固**: 全面对外部输入文本（工具名、参数预览、teammate 名称/角色/描述、subagent 错误消息、浏览过滤器、重试提示、title bar 工具描述等）使用 `sanitize_single_line_text` 清洗，防止 ANSI 码和控制字符泄漏到 TUI 界面导致显示异常

### 其他
- **Makefile install 重构**: `make install` 改为从 GitHub Releases 下载预编译二进制安装到 `/usr/local/bin`，不再本地编译；配套更新 `make uninstall`
- **install.sh**: 更新内置 fallback 版本号为 v12.10.21

# v12.10.21

### Bug 修复
- **修复仓库名变更导致的页面空白**: GitHub 仓库名从 j 改为 jcli 后，React Router basename 不匹配导致页面无法渲染

### 改进
- **全面更新仓库引用路径**: 将所有源码、配置、文档、安装脚本中的 LingoJack/j 引用更新为 LingoJack/jcli，涉及以下文件：
  - vite.config.ts: base path 从 /j/ 改为 /jcli/
  - web/index.html: 页面 URL、仓库 URL、SPA 重定向路径
  - web/src/data/i18n/index.ts: 12 处图片路径
  - web/src/pages/Home.tsx: 安装命令
  - web/src/pages/Docs.tsx: GitHub 链接
  - web/src/components/home/: Nav、Footer、HeroSection 的 GitHub 链接
  - web/src/data/docs/: 中英文安装文档
  - src/command/update.rs: 更新检查 API URL
  - src/constants.rs: 版本信息中的仓库 URL
  - Cargo.toml: repository 和 homepage 字段
  - README.md: 安装命令和仓库链接
  - install.sh / install.ps1: REPO 变量和下载 URL
  - assets/help/install.md: 安装命令
  - assets/skills/j-cli/: SKILL.md、commands.md、ensure_j.sh

# v12.10.20

### 新功能
- **j md 支持标准输入渲染**: 管道输入 Markdown 文本时自动渲染为 ANSI 彩色输出到标准输出，支持 `echo "# Hello" | j md`、`cat README.md | j md` 等管道用法，复用已有的 md_render 渲染能力

### 改进
- **Notebook 列表鼠标点击修复**: 滚动后点击列表项时正确累加 scroll offset，不再选中错误条目

# v12.10.19

### Bug 修复
- **修复 Notebook 列表鼠标点击偏移错误**: 滚动后点击列表项时未累加 scroll offset，导致点击到错误条目

### 改进
- **README 全面重写**: 更新功能定位描述（Agent 工作台、别名打开、脚本工作流等），新增 6 张功能截图及说明，添加 j-gui 引导入口
- **文档站点截图展示组件**: 新增 FeaturesWithScreenshots 和 ScreenshotsSection 组件，按功能分类展示终端截图，更新 i18n 内容
- **清理冗余文件**: 移除 README.old.md

# v12.10.18

### Bug 修复
- **修复 GitHub Release 页面不显示 release notes**: CI workflow 现在从 CHANGELOG.md 提取对应版本段落写入 release body
- **修复 Markdown 分类标题被 git tag 吞掉**: 添加 --cleanup=verbatim 保留 # 开头的行

### 改进
- **引入 CHANGELOG.md 管理 release notes**: 发布记录统一由 CHANGELOG.md 维护，make publish 自动读写
- **make publish 支持 NOTE 参数**: 通过环境变量传入 release notes，自动追加到 CHANGELOG.md 顶部

# v12.10.17

### Bug 修复
- **修复 GitHub Release 不渲染 Markdown 分类标题**: git tag 默认 strip # 开头的行，添加 --cleanup=verbatim 保留 Markdown 标题

# v12.10.16

### Bug 修复
- **修复 GitHub Release 不渲染 Markdown 的问题**: tag message 增加独立 subject 行，body 从分类标题开始完整渲染

# v12.10.15

### Bug 修复
- **修复 GitHub Release 不渲染 Markdown 的问题**: 提取 tag message 时跳过版本标题行，让 Release body 从分类标题开始，确保正确渲染

# v12.10.14

### 改进
- **引入 CHANGELOG.md 管理 release notes**: 发布记录统一由 CHANGELOG.md 维护，make publish 自动读写
- **修复 make publish 多行 NOTE 解析失败**: 改用环境变量传递 NOTE，避免 Make 变量展开问题
- **修复 GitHub Release 不渲染 Markdown**: tag message 统一从 CHANGELOG.md 提取，确保包含完整标题和分类
- **make release-note 改为预览 CHANGELOG.md**: 不再依赖 AI 生成，直接从文件读取最新段落

# v12.10.13

### 改进
- **引入 CHANGELOG.md 管理 release notes**: 发布记录统一由 CHANGELOG.md 维护，make publish 自动读写
- **修复 make publish 多行 NOTE 解析失败**: 改用环境变量传递 NOTE，避免 Make 变量展开问题
- **make release-note 改为预览 CHANGELOG.md**: 不再依赖 AI 生成，直接从文件读取最新段落

# v12.10.11

### Bug 修复
- **修复 GitHub Release 页面不显示 release notes 的问题**: 将 release workflow 的 generate_release_notes 改为 false，使 GitHub Release 使用 annotated tag 中手动编写的 release notes，而非被 GitHub 自动生成的 Full Changelog 链接覆盖

### 改进
- **Makefile publish 支持 NOTE 参数**: make publish 新增 NOTE 参数，支持手动传入 release notes
- **更新 publish command 文档**: 补充了 NOTE 参数的用法说明

# v12.10.9

### Bug 修复
- **修复构建命令被误杀的问题**: Shell 工具的交互式命令静默检测阈值从 10 秒调高到 180 秒，避免 cargo build --release、docker build 等编译阶段长时间无输出的合法命令被错误终止

### 改进
- **Makefile publish 支持 NOTE 参数**: make publish 新增 NOTE 参数，支持手动传入 release notes，不传则回退到 AI 自动生成
- **更新 publish command 文档**: 补充了 NOTE 参数的用法说明和使用示例

# v12.10.8

### 新功能
- **Windows 平台支持**: 新增 PowerShell 工具（PowerShellTool），Windows 下自动替代 ShellTool，实现跨平台命令执行
- **Windows 自动更新**: update 命令新增 Windows x64/ARM64 平台支持，Mac 和 Windows 分别走各自的权限提升逻辑
- **后台任务自动升级**: Shell 工具新增超时自动后台化机制，长时间运行的命令超过阈值后自动移交给 BackgroundManager，不杀进程、不丢失输出
- **交互式命令静默检测**: Shell 工具新增静默超时检测，疑似交互式命令在无输出时提前终止，避免挂起

### 改进
- **SubAgent/Teammate Metrics 统计**: SubAgent 和 Teammate 循环中新增 LLM 调用次数、输入/输出 token、工具调用次数的累加统计
- **配置文件锁重构**: 移除 fs2 依赖，改用基于 create_new() 的独立 .lock 文件互斥机制（LockFileGuard），跨平台无兼容问题
- **终端文本清洗增强**: normalize_terminal_text 函数扩展控制字符清理范围，移除 BEL、BS、ESC、DEL 等控制字符，避免 TUI 脏渲染
- **长时运行命令识别扩展**: shell_safety 新增 podman compose/podman-compose 识别，避免误杀容器编排命令
- **编辑器视口重构**: MarkdownEditor 内部拆分为 ViewportState、ThemeState、RenderMeta 等子结构，改善代码组织和可维护性

### 文档
- **README 重写**: 采用居中简洁设计风格，突出「AI 驱动的命令行工作台」产品定位
- **文档站点优化**: 代码块平台切换器从仿终端窗口样式改为简洁 tab 按钮风格
- **文档构建产物更新**: docs/ 目录下 JS/CSS 重新构建