# v12.10.85


### 新功能

- **编辑器无边框模式**: MarkdownEditor 新增 `set_show_border()` 接口，允许 Notebook 等外层 UI 关闭编辑器自带边框，实现与外部 panel 的视觉无缝衔接
- **BlockCache 块级缓存**: 编辑器渲染器引入基于 pulldown-cmark 解析器的 `BlockCache`，替代原有的朴素 ``` toggle 扫描，准确识别代码块、表格、列表项内嵌套的围栏代码块和 blockquote 内的代码块

### 改进

- **Notebook 面板圆角边框**: 标题栏、笔记列表等面板统一使用 `Rounded` 圆角边框风格，编辑区无边框设计避免双重边框
- **Notebook 鼠标拖选**: 鼠标拖拽事件现在能正确转发给编辑器，支持在编辑区内用鼠标拖选文本；松开鼠标时同步提交选区
- **代码块识别准确性**: 使用 CommonMark/GFM 规范解析器替代 `|...|` 启发式扫描，修复段落夹在两个代码块之间被误判为代码块内容、列表/引用内代码块漏识别等问题
- **表格识别准确性**: 表格行判定改为基于解析器结果，避免非表格的 `|...|` 内容被误匹配为表格
- **缓存宽度感知**: BlockCache 构建时记录折行宽度，宽度变化时自动重建，确保表格高度计算与实际渲染一致

### Bug 修复

- **表格光标跳行**: 修复表格与紧邻段落之间缺少空行时，光标 visual 移动跳过段落行的测试与实际问题
- **未配对围栏行渲染**: 修复未闭合的 ``` 行在某些上下文中被吞掉导致空白屏的问题，回退到普通文本渲染

# v12.10.84


### 改进

- **Notebook TUI 布局优化**: 将标题栏与状态栏合并为统一的顶部栏，移除独立的底部状态栏，为笔记列表腾出更多显示空间
- **Notebook 提示消息改进**: 将操作提示（成功/错误消息）合并显示在顶部标题栏中，取代原先独立的状态栏区域

### 内部变更

- **版本升级**: 版本号从 v12.10.83 升级至 v12.10.84

# v12.10.83


### Bug 修复

- **代码块鼠标点击列偏移**: 修复在代码块内容行中鼠标点击时光标位置偏右 2 个字符的问题（未扣除 `│ ` 装饰列），现在点击 `vscode` 的 `v` 能正确定位，不再变成 `c`
- **Visual 选区复制闭合性**: 修复 character-wise visual mode 复制时选区右端不包含光标字符的问题，现在屏幕高亮范围与复制到剪贴板的内容完全一致

### 新功能

- **编辑器鼠标拖选复制（基于渲染 spans）**: 鼠标拖选高亮和复制现在基于渲染坐标而非源码坐标，跨表格单元格、代码块边框、链接语法拖选时，复制内容 = 屏幕上看到的可见文字，自动跳过边框、padding、行号等装饰元素。选中后按 `y` 或 `c` 复制，按 `Esc` 取消
- **Notebook 括号粘贴（Bracketed Paste）**: Notebook 编辑器和输入框支持终端 bracketed paste，粘贴多行文本不再被逐字符处理，大幅提升粘贴体验
- **Notebook 快捷键**: 目录树新增 `a` 新建、`r` 改名、`d` 删除、`s` 刷新、`o` 打开目录、`[`/`]` 调整面板比例、`q` 退出等快捷键
- **Reader 按 Enter 停止**: `j read` 服务现在按 Enter 即可停止，不再需要 Ctrl-C

### 改进

- **选区工具函数公共化**: `is_selectable_line`、`extract_content_from_line`、`spans_to_char_offset` 等函数从 chat UI 迁移至 `tui/components/selection.rs`，编辑器和聊天界面共用同一套选区逻辑
- **Notebook 重命名错误回显**: 重命名遇到目标文件已存在时，错误信息回显到状态栏，用户可继续修改输入而不会退出重命名模式
- **tokio 特性扩展**: 新增 `io-std` 和 `io-util` feature，支持异步 stdin 读取

# v12.10.82



### Bug 修复

- **编辑器末行显示**: 修复编辑区 `content_height` 计算错误导致内容最末一行被 `~` 占位符覆盖的问题，现在根据是否显示命令栏动态计算保留行数
- **表格内滚动与光标定位**: 修复光标在表格块内部卡住、无法正常上下移动以及视口滚动异常的严重问题，重新设计了 wrap engine 对表格块的高度膨胀机制，使视觉坐标与实际渲染行一一对应

### 改进

- **鼠标滚轮滚动**: 简化滚轮交互为光标驱动模式——滚轮直接移动光标 3 个视觉行，由渲染循环自动追踪视口，消除了之前独立视口滚动与光标同步之间的状态冲突和底部抖动
- **表格渲染架构**: 新增 `measure_table_height` / `measure_cell_wrap_lines` 无分配高度测量，将表格真实渲染高度注入 `WrapEngine` 前缀和数组，移除了旧的 `visual_map` 重分布及渲染范围手动外扩等补丁逻辑，视口定位和滚动计算全部统一到单一视觉坐标系

# v12.10.81


### 新功能

- **`j read` 命令**: 在浏览器中预览 Markdown 文件。启动本地 HTTP 服务（仅绑定 127.0.0.1），自动打开浏览器展示渲染后的文档内容，支持 `--port` 指定端口和 `--no-open` 禁止自动打开浏览器
- **目录侧边栏导航**: Reader 页面右侧自动提取 Markdown 标题生成可点击的目录导航，支持滚动高亮当前标题、平滑滚动定位，可收起/展开
- **~ 路径展开**: `j read` 命令支持 `~` 和 `~/...` 路径自动展开为用户 home 目录
- **纯文本兜底**: 非 Markdown 文件自动以纯文本模式预览，前端展示等宽字体原文
- **文件大小保护**: 预览文件限制为 5 MiB 以内，超大文件给出明确提示

### 改进

- **Markdown IR 支持 JSON 序列化**: 为所有 IR 类型（`ParsedDocument`、`Block`、`Inline`、`TableData` 等）添加 `Serialize` 实现，采用 adjacently-tagged 格式，便于前后端数据交互
- **交互式 Shell 补全**: `read` / `rd` 命令支持 Tab 补全，包括 `--port`、`--no-open` 参数提示和文件路径补全
- **Reader SPA 嵌入二进制**: 使用 `rust-embed` 将前端构建产物编译期嵌入，单二进制分发无需额外文件
- **新增 `build-reader-web` Makefile 目标**: 一键构建 Reader SPA 前端资源

### Bug 修复

- **soft_break 渲染修复**: `soft_break` 元素正确渲染为换行而非空格
</result

# v12.10.80



### 新功能

- **交互模式使用技巧提示**: 交互模式输入空行时随机展示一条使用技巧（内置 21 条），帮助用户发现更多功能
- **编辑器 :w 保存不退出**: 编辑器命令 `:w` 现在仅保存不退出，`:wq` / `:x` 保存并退出；新增 `EditorAction::Save` 动作区分保存与提交
- **Notebook 命令面板新建笔记**: 命令面板（/）新增"新建笔记"选项，无需记忆快捷键

### 改进

- **Notebook 鼠标交互优化**: 单击笔记自动保存当前修改并加载编辑器内容；单击目录即可展开/折叠（不再需要双击）；活跃面板边框以青色高亮区分焦点
- **编辑器滚轮滚动重构**: 滚轮改为移动光标而非直接操控视口偏移，消除视口与光标状态不同步的问题；点击和拖拽时自动解除滚动锁定；光标脱离可视区域时自动归位至视口中央
- **Notebook 帮助栏精简**: 目录树和编辑器焦点下的帮助栏文案大幅简化，信息更清晰

# v12.10.79


### 新功能

- **代码块边框样式配置**: 新增 `code_block_border_style` 配置项，支持 `rounded`（圆角 ╭╮╰╯，默认）和 `plain`（直角 ┌┐└┘）两种代码块围栏边框风格。可通过 `j config setting code_block_border_style plain` 切换
- **Config 命令值自动补全**: `/config setting` 命令的第三个参数现在会根据 key 名自动提示合法的 value 候选值（如 `color_mode` 提示 `auto/truecolor/ansi256/ansi16/none`），且 setting 段的 key 补全会包含所有已知可配置项，不再局限于配置文件中已存在的 key

### 改进

- **代码块右侧内边距**: 代码块右侧增加 2 字符内边距，防止竖线紧贴屏幕右边缘
- **编辑器鼠标交互**: 鼠标点击和拖拽时

### 维护

- **版本升级**: 版本号从 v12.10.78 升级至 v12.10.79，涉及 j-cli、j-agent 及安装脚本（install.sh / install.ps1）的版本同步更新

# v12.10.78


### 新功能

- **交互模式使用技巧提示**: 交互模式输入空行时随机展示一条使用技巧，帮助用户发现更多功能
- **编辑器 :w 保存不退出**: 编辑器命令 `:w` 现在仅保存不退出，`:wq` / `:x` 保存并退出

### 改进

- **Notebook 交互优化**: 单击笔记自动保存当前修改并加载编辑器内容；单击目录展开/折叠（不再需要双击）；活跃面板边框高亮（青色）
- **编辑器滚轮滚动重构**: 滚轮改为移动光标而非直接操控视口偏移，消除视口与光标状态冲突；点击和拖拽时自动解除滚动锁定
- **Notebook 命令面板**: 新增"新建笔记"选项，简化各模式下的帮助栏文案
- **Notebook 编辑器集成重构**: 将 Normal 模式按键处理从 input.rs 移至 handler.rs，独立 `handle_tree_focus_key` 和 `handle_editor_focus_key` 函数职责更清晰

### Bug 修复

- **编辑器无法滚动到底部**: 修复 `scroll_viewport_down` 使用上一帧渲染行数计算滚动上限，导致表格等内容底部无法滚动到的问题
- **编辑器鼠标点击后闪烁**: 修复点击后 `scroll_locked` 未解除、光标行源码/渲染模式切换引发渲染行数抖动导致视口闪烁的问题；重构渲染流程为三阶段避免反馈循环

# v12.10.77


### 新功能

- **交互模式显示工作目录**: 每轮输入前显示当前工作目录（HOME 缩写为 `~`，超长路径中间省略）
- **编辑器 Insert 模式命令面板**: Insert 模式下输入 `/` 弹出命令面板，支持快速插入图片模板 `![]()` 等内容，面板定位在光标下方而非固定底部
- **fp 预设脚本**: 新增 `j fp <文件名>` 预设命令，在当前目录 3 层深度内查找文件

### 改进

- **编辑器滚动优化**: 使用实际渲染行数计算滚动上限（修复表格等内容无法滚到底部的问题）；鼠标滚轮在边界处不再回退移动光标（修复底部抖动）
- **编辑器点击空白区域定位**: 点击内容末尾的 `~` 空白行时自动定位到最后一行末尾
- **update 模块拆分**: 将 1100+ 行的 `update.rs` 拆分为 10 个子模块（github_update、fallback、permission、restart 等），提升可维护性
- **TUI 主循环拆分**: 将 1400+ 行的 `tui_loop.rs` 拆分为 dispatch、mouse、terminal_setup、ws_handler 子模块
- **工具渲染拆分**: 将 `tool_call_render.rs`（1600+ 行）和 `tool_result_render.rs`（1500+ 行）按工具类型拆分为独立子模块
- **编辑器模块拆分**: 将 `editor.rs` 拆分为 api、commands、render、selection 子模块

### Bug 修复

- **编辑器滚动上限计算**: 修复 `update_scroll_from_visual` 使用 wrap_engine 视觉行数导致表格等内容底部无法滚动到的问题

# v12.10.76



### 改进

- **编辑器鼠标点击空白区域**: 点击编辑器内容下方的空白区域（`~` 填充行）时，光标现在会定位到最后一行的末尾，与主流编辑器行为一致
- **编辑器滚动行为优化**: 当滚动到达文档顶部或底部时，继续滚动会移动光标而非无效停留在原位，支持光标随滚轮自然上下移动

# v12.10.75


### Bug 修复

- **编辑器滚动修复**: 修复表格等元素渲染后底部内容无法滚动到的问题，改用实际渲染行数（`rendered_line_count`）替代 `wrap_engine` 的视觉行数来计算滚动上限，确保鼠标滚轮和光标移动时的滚动范围正确

# v12.10.74


### 新功能

- **交互模式显示当前工作目录**: 每轮交互前展示当前工作路径，HOME 目录缩写为 `~`，过长路径自动中间省略（`...`），方便确认执行上下文

### 改进

- **模块拆分提升可维护性**: 将 `update`、`tui_loop`、`tool_call_render`、`tool_result_render`、`editor` 五个大型单文件模块按职责拆分为独立子模块，降低单文件复杂度，便于后续维护和扩展

# v12.10.73


### 新功能

- **Insert 模式斜杠命令面板**: Markdown 编辑器 Insert 模式下输入 `/` 弹出命令面板，支持快速插入图片 `![]()` 等内容；面板跟随光标位置渲染，无匹配项时自动关闭避免干扰正常输入
- **工具结果分类摘要全覆盖**: 为 Write、Edit、Glob、Grep、WebFetch、WebSearch、Browser、TaskOutput、LoadSkill 等所有工具添加专用结果摘要函数，TUI 中工具调用结果展示更直观
- **TaskOutput 结构化渲染**: 为 TaskOutput 工具结果新增专用渲染器，解析 JSON 输出并以带状态图标、命令高亮、输出折叠的格式展示任务执行结果
- **配置页鼠标选区复制**: 配置页（Config 模式）支持鼠标拖拽选区和复制文本，新增行缓存与内容区域坐标映射，与 Help 模式选区体验一致
- **`~/` 路径补全**: 文件弹窗和 `@` 弹窗支持 `~` 路径展开，输入 `~/` 即可浏览用户主目录下的文件
- **`j md` 无参数快捷编辑**: 无参数调用 `j md` 时自动创建并编辑临时笔记 `temp_note_{N}.md`
- **YAML 语法高亮**: 为 YAML 文件新增完整语法高亮支持，包括键名、文档分隔符、列表指示符、锚点/别名、合并键、块标量指示符及类型标签的独立着色

### 改进

- **流式 Markdown 重绘机制重构**: 从基于 DSR 光标定位改为基于行数跟踪，使用 Unicode 宽度计算实际渲染行数，提升流式输出重绘的稳定性，移除对 `crossterm::cursor::position` 的依赖
- **YAML 关键字字典扩展**: 补充 `True`/`False`/`TRUE`/`FALSE`/`Null`/`NULL`/`~` 等常见布尔与空值变体

### Bug 修复

- **表格底部滚动问题**: 修复 TUI 编辑器中无法通过鼠标滚轮滚动到表格底部的问题，新增 `visual_map` 映射机制和渲染范围自动扩展逻辑，确保表格等块级元素可完整滚动浏览

# v12.10.72


### 改进

- **流式 Markdown 重绘机制重构**: 从基于 DSR 光标定位的行号保存方式改为基于行数跟踪，使用 Unicode 宽度计算实际渲染行数，提升流式输出重绘的稳定性

### Bug 修复

- **表格底部滚动问题**: 修复 TUI 编辑器中无法通过鼠标滚轮滚动到表格底部的问题，新增 `visual_map` 映射机制和渲染范围自动扩展逻辑

### 内部改进

- **测试用例修正**: `FileIndex::fuzzy_match` 测试改用公开方法并修正预期值

# v12.10.71


### 新功能

- **工具结果分类摘要全覆盖**: 为 Write、Edit、Glob、Grep、WebFetch、WebSearch、Browser、TaskOutput、LoadSkill、LoadTool、PlanMode、Worktree、ComputerUse 等所有工具添加专用的结果摘要函数，TUI 中工具调用结果展示更直观
- **工具调用描述补全**: 新增 `extract_tool_description_from_args` 对 LoadTool、Session 工具的描述提取，折叠视图下显示更有意义的工具调用标签
- **TaskOutput 结构化渲染**: 为 TaskOutput 工具结果新增专用渲染器，解析 JSON 输出并以带状态图标、命令高亮、输出折叠的结构化格式展示任务执行结果
- **配置页鼠标选区复制**: 配置页（Config 模式）支持鼠标拖拽选区和复制文本，新增行缓存、内容区域 inner rect 和滚动偏移记录，与 Help 模式选区体验一致
- **`j md` 无参数快捷编辑**: 无参数调用 `j md` 时自动创建并编辑临时笔记 `temp_note_{N}.md`（N 自增），与 `j notebook` 无参数进入 TUI 列表的行为区分开

### 改进

- **版本升级至 v12.10.71**: 同步更新 Cargo.toml、install.sh、install.ps1 及 j-agent 版本号

# v12.10.70


### 新功能

- **`~/` 路径补全**: 文件弹窗和 `@` 弹窗现在支持 `~` 路径展开，输入 `~/` 即可浏览用户主目录下的文件，不再返回空结果

### 改进

- **版本升级至 v12.10.70**: 同步更新 Cargo.toml、install.sh、install.ps1 及 j-agent 版本号

# v12.10.69


### 新功能

- **工具结果结构化渲染**: 为 Glob、Grep、WebSearch、WebFetch、Write、Edit、Task、SendMessage 等工具添加了专用的结果摘要渲染函数，在 TUI 中展示更直观的工具调用结果
- **工具调用描述补全**: 为所有工具补全了 `get_result_summary` 和 `get_tool_call_description` 提取逻辑，涵盖 Write、Edit、Glob、Grep、WebFetch、WebSearch、Browser、RegisterHook、LoadSkill、SendMessage、PlanMode、Worktree 等工具
- **配置页 Model Tab 左右分栏布局**: 将 Model 配置页从水平 Provider 标签页改为左侧 Provider 列表 + 右侧配置字段的双栏布局，支持点击选择 Provider，新增 `ModelToggleLevel` 操作在 Provider 列表和字段间切换焦点
- **折行引擎支持代码块边框适配**: WrapEngine 新增 `rebuild_cache_with_code_blocks` 方法，代码块内容行自动减去边框宽度进行折行，避免溢出

### 改进

- **Shell 工具统一**: 移除独立的 PowerShell 工具 (`powershell.rs`)，统一使用 `ShellTool`，工具名从 `Bash` 改为 `Shell`，消除 Windows/Unix 双分支维护
- **配置页逐行渲染**: 将配置页从单个 `Paragraph` 整体渲染改为逐行渲染（`render_block_lines`），解决部分终端软换行导致相邻行被污染的问题
- **更新机制增强**: 修正更新源仓库名称为 `jcli`，扩展网络错误回退匹配（TLS、证书、连接错误等），Windows 使用 PowerShell 作为备用下载方案
- **终端状态恢复重构**: 提取 `try_enable_keyboard_enhancement` 和 `restore_terminal_state` 为独立函数，`PopKeyboardEnhancementFlags` 失败不再短路后续终端恢复步骤
- **代码导入统一**: 统一使用 `use` 导入替代内联完整路径引用，提升可读性
- **YAML 语法高亮增强**: 为 YAML 文件新增完整的语法高亮支持，包括键名（key）、文档分隔符（`---`/`...`）、列表指示符（`-`）、锚点（`&anchor`）、别名（`*alias`）、合并键（`<<`）、块标量指示符（`|`/`>`）及类型标签（`!!str` 等）的独立着色
- **YAML 关键字字典扩展**: 补充 `True`/`False`/`TRUE`/`FALSE`/`Null`/`NULL`/`~` 等常见布尔与空值变体，提升关键字识别覆盖率

### Bug 修复

- **Unicode 宽度折行偏移**: 使用 Unicode 宽度计算替代字符计数修复描述文本折行偏移问题，正确处理中文字符等双宽字符
- **dedent 字节越界**: 修复 `dedent` 函数在处理含非 ASCII 空白字符的文本时，因按字符计数与字节切片不一致导致的越界 panic，改为仅对 ASCII 空白字符计数并安全计算字节偏移
- **Provider 圆点颜色**: 修复模型提供商圆点（active indicator）颜色受选中状态影响的问题，圆点颜色现在独立显示
- **倒计时溢出**: 倒计时使用 `checked_add` 防止 `Instant` 溢出导致的 panic

# v12.10.68


### 改进
- **Model Provider 鼠标点击**: Model 配置页面左侧 Provider 列表现在支持鼠标点击选中，双击已选中的 Provider 可进入字段编辑。
- **Provider 选中高亮延伸**: Model Provider 列表中选中项的高亮背景色现在延伸到行尾，视觉反馈更加完整清晰。
</result

# v12.10.67


### 新功能
- **描述文本自动折行**: Skills 和 Commands 配置页面的描述文本现在支持自动折行显示，长描述不再被截断，而是根据窗口宽度智能换行。

### 改进
- **Model 布局优化**: 移除了 Model 配置页面右侧面板的边框，改为使用 padding 分隔，视觉更加简洁。
- **分隔线优化**: Global 配置页面的分隔线不再贴边显示，宽度与内容对齐。
- **Provider 列表宽度**: 调整了 Model 页面 Provider 列表的最小/最大宽度限制，适应更长的提供商名称。

### Bug 修复
- **Unicode 宽度计算**: 使用 Unicode 宽度计算替代字节长度，修复了中文字符等宽字符描述文本折行时偏移不正确的问题。
- **圆点颜色逻辑**: Model Provider 列表中的圆点颜色现在独立于选中状态显示，仅反映启用/禁用状态。

# v12.10.66


### 新功能

- **Model Tab 左右分栏布局**: Model 配置界面从原来的水平 Provider sub-tabs 切换模式改为左右分栏布局，左侧为 Provider 列表，右侧为选中 Provider 的配置字段详情，与 Tools Tab 风格统一
- **Model Tab 层级导航**: Tab 键现在用于在 Provider 列表和配置字段之间切换焦点层级，上下键在不同层级内导航，Enter 在字段层级进入编辑

### Bug 修复

- **dedent 函数字节越界**: 修复 `dedent` 函数在处理含非 ASCII 空白字符的文本时，因按字符计数与字节切片不一致导致的越界 panic 问题，改为仅对 ASCII 空白字符计数并安全计算字节偏移

### 改进

- **版本升级**: 版本号从 12.10.65 升级至 12.10.66

# v12.10.65


### 改进

- **安装脚本版本号集中管理**: `install.sh` 和 `install.ps1` 新增 `DEFAULT_VERSION` / `$DefaultVersion` 常量，替换所有硬编码的示例版本号 `v1.0.0`，确保帮助信息和错误提示中显示真实可用版本
- **Makefile 版本同步增强**: `bump-version` 和 `set-version` 目标新增自动同步安装脚本版本号的逻辑，发布新版本时安装脚本中的版本号自动更新
- **更新失败提示改进**: `update.rs` 在 Windows 回退安装失败时新增 GitHub Releases 页面链接提示，方便用户手动下载
</result

# v12.10.64


### 改进

- **Shell 工具统一**: 将 Unix (Bash) 和 Windows (PowerShell) 的 Shell 工具合并为统一的 `Shell` 工具入口，按平台条件编译分发到 `shell/unix.rs` 和 `shell/windows.rs`，消除了工具名分叉（`Bash` / `PowerShell` → 统一 `Shell`），简化权限规则、渲染和分类链路的维护

- **终端状态恢复**: 提取 `restore_terminal_state()` 和 `try_enable_keyboard_enhancement()` 为独立函数，消除 `TerminalGuard::Drop`、`restore_terminal()` 和主循环退出三处的重复代码；keyboard enhancement 失败时不再无条件设置 flag，并确保 `PopKeyboardEnhancementFlags` 失败不会短路后续的鼠标捕获和备用屏幕恢复

### Bug 修复

- **配置页渲染污染**: 配置页改用逐行渲染 `render_block_lines()` 替代单 `Paragraph` widget，解决部分终端在软换行时内容溢出污染相邻行的问题

- **倒计时溢出**: 倒计时校准使用 `checked_add` 代替直接 `start + Duration`，防止极长倒计时场景下 `Instant` 加法溢出导致的 panic

- **更新源仓库名称**: 修正 `self_update` 配置中 `repo_name` 从 `"j"` 改为 `"jcli"`，使自动更新指向正确的 GitHub 仓库

- **更新网络错误回退**: 补充 `ReqwestError` 和 `sending request` 到更新失败的回退匹配列表，修复网络不通或代理场景下无法正确降级的问题
</result

# v12.10.63


### 改进

- **代码块折行适配边框宽度**: 代码块内容行折行时自动减去左右边框（4 字符），避免内容溢出边框导致显示错乱。重构 `WrapEngine` 支持按行设置有效折行宽度，修复折行片段重复渲染问题。

- **列表样式增强**: 有序列表序号使用主题色加粗显示，无序列表项目符号颜色统一调整为绿色系以提升视觉区分度。

# v12.10.62


### 新功能

- **Read 工具语法高亮**: Read 工具结果支持根据文件扩展名自动进行语法高亮显示，支持 Rust、Go、Python、JavaScript、TypeScript、Java、C/C++、Bash、JSON、YAML、TOML、Markdown、SQL 等 20+ 种语言

- **Glob 工具结果树形渲染**: Glob 结果改为树形文件列表，区分文件和目录的颜色显示，自动提取公共前缀简化路径，显示文件数量统计

- **Grep 工具结果结构化渲染**: Grep 结果支持三种输出模式的结构化渲染：content 模式显示文件路径+行号+匹配内容，count 模式显示文件匹配数统计，files_with_matches 模式显示文件列表，并自动汇总匹配数和文件数

### 改进

- **工具调用展开渲染优化**: Glob/Grep/Read/Write/Edit 工具调用在展开模式下只显示关键参数，路径过长时自动截断首尾，Edit 工具显示 old/new 字符串摘要（行数+预览）

- **WebSearch 结果结构化显示**: WebSearch 结果改为序号+标题+URL+摘要的结构化布局，标题高亮显示，末尾显示结果数统计

- **WebFetch 结果 Markdown 渲染**: WebFetch 结果自动检测是否为 Markdown 内容，包含标题/列表等标记时用 Markdown 渲染器渲染，否则纯文本折行显示

- **Task 结果状态图标显示**: Task 结果解析 JSON 数组，显示状态图标（●/◉/○）、任务 ID、标题，不同状态使用不同颜色区分

- **Write/Edit 结果路径高亮**: Write/Edit 结果高亮文件路径，失败时（找不到匹配、匹配不唯一）显示红色错误信息

- **SendMessage 结果发送确认**: SendMessage 结果显示发送目标（@target 高亮）和消息预览

# v12.10.61



### 改进

- **版本升级**: 版本号从 12.10.60 升级到 12.10.61

本版本为维护性版本更新，无新增功能或 Bug 修复。

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