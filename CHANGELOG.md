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
