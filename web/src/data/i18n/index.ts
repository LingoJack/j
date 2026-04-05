import type { I18nData, Language } from '../../types'

export const i18n: Record<Language, I18nData> = {
  en: {
    nav: {
      features: 'Features',
      quickStart: 'Quick Start',
      github: 'GitHub'
    },
    hero: {
      badge: 'CLI Tool',
      title: 'Your command line,',
      titleHighlight: 'simple, but works.',
      subtitle: 'Alias management, daily reports, todo notes, AI chat, and browser automation.',
      subtitleExtra: 'One command to boost your productivity.',
      getStarted: 'Get Started',
      viewGithub: 'View on GitHub →'
    },
    features: {
      title: 'Core Features',
      subtitle: 'One tool, many capabilities. From daily task management to AI assistance.',
      list: [
        { icon: '→', title: 'Alias Management', description: 'Register apps, URLs, and scripts. Access them with j <alias>. Supports categorization and combination.' },
        { icon: '■', title: 'Daily Reports', description: 'Quick write, view, and search daily reports with automatic week management. Git sync supported.' },
        { icon: '✓', title: 'Todo & Notes', description: 'Built-in TUI todo manager with Markdown checkbox support. Links to daily reports on completion.' },
        { icon: '◉', title: 'AI Chat', description: 'TUI AI chat with multi-model support, streaming output, tool calling, and remote control.' },
        { icon: '◐', title: 'Browser Automation', description: 'Lite mode for lightweight HTTP control, CDP mode for full browser automation with screenshots.' },
        { icon: '◈', title: 'Script System', description: 'Create and register scripts as aliases with environment variable injection and new window execution.' }
      ]
    },
    quickStart: {
      title: 'Quick Start',
      subtitle: 'Get up and running in minutes.',
      installation: 'Installation',
      oneLineInstall: 'One-line install (recommended)',
      cratesInstall: 'Or install from crates.io',
      usageExamples: 'Usage Examples',
      examples: [
        { cmd: 'j set chrome "/Applications/Google Chrome.app"', description: 'Register app alias' },
        { cmd: 'j set github https://github.com', description: 'Register URL alias' },
        { cmd: 'j chrome', description: 'Open Chrome' },
        { cmd: 'j chrome "search query"', description: 'Search with Chrome' },
        { cmd: 'j report "Completed feature"', description: 'Write to daily report' },
        { cmd: 'j todo add Buy milk', description: 'Quick add todo' },
        { cmd: 'j chat', description: 'Enter AI chat' }
      ]
    },
    more: {
      title: 'And More',
      list: [
        { title: 'Interactive Mode', desc: 'REPL with Tab completion and history suggestions.' },
        { title: 'Permission Control', desc: 'Fine-grained tool permissions for sensitive operations.' },
        { title: 'Remote Control', desc: 'Connect mobile for remote AI chat control.' },
        { title: 'Agent Mode', desc: 'Autonomous multi-step reasoning with tool calling.' },
        { title: 'Context Compact', desc: 'Three-layer compression for context management.' },
        { title: 'Multiple Themes', desc: 'Dark, Light, Dracula, Gruvbox, Monokai, Nord.' }
      ]
    },
    bestPractices: {
      title: 'Best Practices',
      subtitle: 'Practical tips to maximize your productivity with jcli.',
      categories: [
        {
          title: 'Alias Management',
          tips: [
            { title: 'Categorize aliases', desc: 'Use sections like browser, editor, inner_url, outer_url to organize aliases by type.', example: 'j set browser:chrome "/Applications/Google Chrome.app"' },
            { title: 'Quick search', desc: 'Register a browser and search directly with text. No need to open browser first.', example: 'j chrome "search query"' },
            { title: 'Combine with VPN', desc: 'Mark URLs as outer_url to auto-connect VPN when opening.', example: 'j set outer_url:work-portal https://internal.company.com' },
            { title: 'Script as alias', desc: 'Register frequently used scripts as aliases with environment variables.', example: 'j set script:deploy ~/scripts/deploy.sh' }
          ]
        },
        {
          title: 'Daily Reports',
          tips: [
            { title: 'Write immediately', desc: 'Add notes right after completing tasks. Use quotes for multi-word content.', example: 'j report "Implemented user authentication feature"' },
            { title: 'Use TUI editor', desc: 'Run j report without arguments to open TUI for multi-line editing with history.', example: 'j report' },
            { title: 'Weekly sync', desc: 'Use reportctl to create weekly reports and sync across devices.', example: 'j reportctl new && j reportctl push' },
            { title: 'Search history', desc: 'Quickly search past reports with fuzzy matching.', example: 'j report search authentication' }
          ]
        },
        {
          title: 'Todo & Notes',
          tips: [
            { title: 'Quick capture', desc: 'Add todos on the fly without interrupting your workflow.', example: 'j todo add Review pull request' },
            { title: 'Link to reports', desc: 'Completed todos can automatically write to daily reports.', example: 'j todo done 1 --report' },
            { title: 'Use TUI manager', desc: 'Interactive TUI for managing complex todo lists.', example: 'j todo' },
            { title: 'Markdown support', desc: 'Write todos with Markdown checkboxes in report files.', example: '- [x] Completed task' }
          ]
        },
        {
          title: 'AI Chat Workflow',
          tips: [
            { title: 'Context files', desc: 'Use @file: to include local files as context for AI.', example: '@file:src/main.rs Explain this code' },
            { title: 'Web search', desc: 'Enable web search to let AI fetch latest information.', example: 'What are the new features in React 19?' },
            { title: 'Tool permissions', desc: 'Configure fine-grained permissions for sensitive operations.', example: 'Allow: Read, Bash, WebFetch' },
            { title: 'Compact context', desc: 'Use /compact to compress conversation and free context window.', example: '/compact focus:current task' }
          ]
        },
        {
          title: 'Script Execution',
          tips: [
            { title: 'Create script', desc: 'Create executable script with content, auto-registered to script section.', example: 'j concat open "open $1"' },
            { title: 'TUI editor', desc: 'Open TUI editor to write script when no content provided.', example: 'j concat deploy' },
            { title: 'New window', desc: 'Execute script in new terminal window without blocking.', example: 'j open -w README.md' },
            { title: 'Pass arguments', desc: 'Pass arguments to script, referenced as $1, $2, etc.', example: 'j open README.md' }
          ]
        }
      ]
    },
    tech: {
      title: 'Built with Rust'
    },
    cta: {
      title: 'Ready to get started?',
      subtitle: 'One command to begin your productivity journey.'
    },
    footer: {
      license: 'MIT License'
    }
  },
  zh: {
    nav: {
      features: '功能',
      quickStart: '快速开始',
      github: 'GitHub'
    },
    hero: {
      badge: '命令行工具',
      title: '你的命令行，',
      titleHighlight: '简洁高效。',
      subtitle: '别名管理、日报系统、待办备忘、AI 对话、浏览器自动化。',
      subtitleExtra: '一个命令，效率翻倍。',
      getStarted: '快速开始',
      viewGithub: '查看源码 →'
    },
    features: {
      title: '核心功能',
      subtitle: '一个工具，多种能力。从日常任务管理到 AI 辅助。',
      list: [
        { icon: '→', title: '别名管理', description: '注册应用、URL、脚本，通过 j <别名> 快速访问，支持分类标记和组合使用。' },
        { icon: '■', title: '日报系统', description: '快速写入、查看、搜索日报，自动周数管理，支持 Git 同步。' },
        { icon: '✓', title: '待办备忘', description: '内置 TUI 待办管理，支持 Markdown checkbox，完成时可联动写入日报。' },
        { icon: '◉', title: 'AI 对话', description: 'TUI AI 对话，多模型支持、流式输出、工具调用，支持远程控制。' },
        { icon: '◐', title: '浏览器自动化', description: 'Lite 模式轻量级 HTTP 控制，CDP 模式完整浏览器自动化，支持截图。' },
        { icon: '◈', title: '脚本系统', description: '创建脚本并注册为别名，支持环境变量注入、新窗口执行。' }
      ]
    },
    quickStart: {
      title: '快速开始',
      subtitle: '几分钟即可上手。',
      installation: '安装',
      oneLineInstall: '一键安装（推荐）',
      cratesInstall: '或从 crates.io 安装',
      usageExamples: '使用示例',
      examples: [
        { cmd: 'j set chrome "/Applications/Google Chrome.app"', description: '注册应用别名' },
        { cmd: 'j set github https://github.com', description: '注册 URL 别名' },
        { cmd: 'j chrome', description: '打开 Chrome' },
        { cmd: 'j chrome "搜索内容"', description: '用 Chrome 搜索' },
        { cmd: 'j report "完成功能开发"', description: '写入日报' },
        { cmd: 'j todo add 买牛奶', description: '快速添加待办' },
        { cmd: 'j chat', description: '进入 AI 对话' }
      ]
    },
    more: {
      title: '更多特性',
      list: [
        { title: '交互模式', desc: '带 Tab 补全和历史建议的 REPL 环境。' },
        { title: '权限控制', desc: '细粒度的工具权限配置，敏感操作需确认。' },
        { title: '远程控制', desc: '扫码连接手机，远程操作 AI 对话。' },
        { title: 'Agent 模式', desc: '自主多步推理，自动调用工具完成复杂任务。' },
        { title: '对话压缩', desc: '三层压缩机制，智能管理上下文窗口。' },
        { title: '多主题支持', desc: 'Dark、Light、Dracula、Gruvbox、Monokai、Nord。' }
      ]
    },
    bestPractices: {
      title: '最佳实践',
      subtitle: '高效使用 jcli 的实用技巧。',
      categories: [
        {
          title: '别名管理',
          tips: [
            { title: '设置常用别名', desc: '将常用的应用、目录、网址设置为别名，快速访问。', example: 'j set code /Users/you/projects' },
            { title: '分类标记', desc: '用 note 命令标记别名类型，便于分类查找。', example: 'j note chrome browser' },
            { title: '快速查找', desc: '在指定分类中查找别名，支持多个分类逗号分隔。', example: 'j find chrome browser,vpn' },
            { title: '重命名别名', desc: '重命名已有别名，保持路径不变。', example: 'j rn chrome browser' }
          ]
        },
        {
          title: '浏览器与搜索',
          tips: [
            { title: '快速搜索', desc: '浏览器别名后接搜索文本，自动使用搜索引擎。', example: 'j chrome React 教程' },
            { title: '打开内网地址', desc: '将内网地址设为 inner_url，直接通过别名打开。', example: 'j set wiki https://wiki.company.com' },
            { title: '联动 VPN', desc: '将外网地址标记为 outer_url，打开时自动连接 VPN。', example: 'j note portal outer_url' },
            { title: '指定搜索引擎', desc: '搜索时可指定搜索引擎（google/bing/baidu）。', example: 'j chrome Rust 文档 baidu' }
          ]
        },
        {
          title: '日报系统',
          tips: [
            { title: '快速写入', desc: '直接写入日报内容，支持多参数拼接。', example: 'j report 完成用户模块开发' },
            { title: 'TUI 编辑', desc: '无参数运行 report 打开 TUI 编辑器，支持多行编辑。', example: 'j report' },
            { title: '查看日报', desc: '查看最近 N 行日报内容。', example: 'j check 10' },
            { title: '搜索历史', desc: '在日报中搜索关键字，支持模糊匹配。', example: 'j search all 用户 -fuzzy' }
          ]
        },
        {
          title: '待办备忘',
          tips: [
            { title: '快速添加', desc: '命令行直接添加待办事项。', example: 'j todo add 完成代码审查' },
            { title: '列出待办', desc: '以 Markdown 格式输出待办列表，可过滤已完成/未完成。', example: 'j todo list --undone' },
            { title: 'TUI 管理', desc: '无参数运行进入 TUI 界面，交互式管理待办。', example: 'j todo' }
          ]
        },
        {
          title: '脚本执行',
          tips: [
            { title: '创建脚本', desc: '创建可执行脚本，自动保存到数据目录的 scripts 文件夹。', example: 'j concat open "open $1"' },
            { title: 'TUI 编辑', desc: '不提供内容时打开 TUI 编辑器编写脚本。', example: 'j concat open' },
            { title: '新窗口执行', desc: '脚本可在新终端窗口中执行，不阻塞当前终端。', example: 'j open -w README.md' },
            { title: '传递参数', desc: '执行脚本时可传递参数，脚本内用 $1、$2 等引用。', example: 'j open README.md' }
          ]
        }
      ]
    },
    tech: {
      title: '基于 Rust 构建'
    },
    cta: {
      title: '准备好了吗？',
      subtitle: '一行命令，开启高效之旅。'
    },
    footer: {
      license: 'MIT 许可证'
    }
  }
}
