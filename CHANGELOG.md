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
