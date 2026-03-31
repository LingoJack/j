import { useState } from 'react'
import { Link } from 'react-router-dom'

type Lang = 'en' | 'zh'

const i18n = {
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
        { icon: '⚡', title: 'Alias Management', description: 'Register apps, URLs, and scripts. Access them with j <alias>. Supports categorization and combination.' },
        { icon: '📝', title: 'Daily Reports', description: 'Quick write, view, and search daily reports with automatic week management. Git sync supported.' },
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
        { icon: '⚡', title: '别名管理', description: '注册应用、URL、脚本，通过 j <别名> 快速访问，支持分类标记和组合使用。' },
        { icon: '📝', title: '日报系统', description: '快速写入、查看、搜索日报，自动周数管理，支持 Git 同步。' },
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
            { title: '分类标记', desc: '用 note 命令标记别名类型（browser/editor/vpn/script/outer_url）。', example: 'j note chrome browser' },
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

// Copy button component
function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false)
  
  const handleCopy = async () => {
    await navigator.clipboard.writeText(text)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }
  
  return (
    <button
      onClick={handleCopy}
      className="absolute right-3 top-1/2 -translate-y-1/2 px-3 py-1.5 text-xs font-medium 
                 text-stone-600 hover:text-stone-900 bg-white hover:bg-stone-50 
                 rounded border border-stone-300 hover:border-stone-400 transition-colors shadow-sm"
    >
      {copied ? 'Copied!' : 'Copy'}
    </button>
  )
}

// Code block component
function CodeBlock({ children, showCopy = true }: { children: string; showCopy?: boolean }) {
  return (
    <div className="relative group">
      <pre className="bg-[#faf9f6] text-stone-800 rounded-lg p-4 text-sm overflow-x-auto font-mono border border-stone-200">
        <code>{children}</code>
      </pre>
      {showCopy && <CopyButton text={children} />}
    </div>
  )
}

// Feature card component
function FeatureCard({ icon, title, description }: { icon: string; title: string; description: string }) {
  return (
    <div className="p-6 bg-white rounded-lg border border-stone-200 hover:border-stone-300 transition-colors">
      <div className="text-2xl mb-3">{icon}</div>
      <h3 className="text-base font-medium text-stone-900 mb-2">{title}</h3>
      <p className="text-stone-600 text-sm leading-relaxed">{description}</p>
    </div>
  )
}

// Command example component
function CommandExample({ cmd, description }: { cmd: string; description: string }) {
  return (
    <div className="py-3 border-b border-stone-200 last:border-0">
      <code className="text-stone-700 font-mono text-sm block mb-1">
        {cmd}
      </code>
      <span className="text-stone-500 text-sm">{description}</span>
    </div>
  )
}

// Tip card component for best practices
function TipCard({ title, desc, example }: { title: string; desc: string; example: string }) {
  return (
    <div className="p-5 bg-white rounded-lg border border-stone-200 hover:border-stone-300 transition-colors">
      <h4 className="font-medium text-stone-900 mb-2">{title}</h4>
      <p className="text-stone-600 text-sm mb-3 leading-relaxed">{desc}</p>
      <div className="bg-[#faf9f6] rounded px-3 py-2 border border-stone-200">
        <code className="text-stone-700 text-xs font-mono">{example}</code>
      </div>
    </div>
  )
}

// Section component
function Section({ id, children, className = '' }: { id?: string; children: React.ReactNode; className?: string }) {
  return (
    <section id={id} className={`py-16 md:py-24 px-6 ${className}`}>
      <div className="max-w-4xl mx-auto">
        {children}
      </div>
    </section>
  )
}

export default function App() {
  const [lang, setLang] = useState<Lang>('zh')  // 默认中文
  const t = i18n[lang]
  
  const installCmd = 'curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh'

  return (
    <div className="min-h-screen bg-[#faf9f6] text-stone-800">
      {/* Navigation */}
      <nav className="fixed top-0 left-0 right-0 z-50 bg-[#faf9f6]/90 backdrop-blur-sm border-b border-stone-200/50">
        <div className="max-w-4xl mx-auto px-6 py-4 flex items-center justify-between">
          <a href="#" className="flex items-center gap-2">
            <span className="text-2xl font-bold text-stone-900">j</span>
            <span className="text-stone-400 text-sm hidden sm:inline">CLI tool</span>
          </a>
          <div className="flex items-center gap-5">
            {/* Language Switcher */}
            <div className="relative group">
              <button className="text-stone-500 hover:text-stone-900 transition-colors text-sm flex items-center gap-0.5 whitespace-nowrap">
                {lang === 'en' ? 'EN' : '中文'}
                <svg className="w-3 h-3 ml-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2}>
                  <path strokeLinecap="round" strokeLinejoin="round" d="M19 9l-7 7-7-7" />
                </svg>
              </button>
              <div className="opacity-0 invisible group-hover:opacity-100 group-hover:visible absolute top-full left-0 mt-1 bg-white rounded shadow-lg py-1 z-50 transition-all">
                <button
                  onClick={() => setLang('en')}
                  className={`block w-full text-left px-3 py-1.5 text-sm hover:bg-stone-50 whitespace-nowrap ${lang === 'en' ? 'text-stone-900 font-medium' : 'text-stone-500'}`}
                >
                  EN
                </button>
                <button
                  onClick={() => setLang('zh')}
                  className={`block w-full text-left px-3 py-1.5 text-sm hover:bg-stone-50 whitespace-nowrap ${lang === 'zh' ? 'text-stone-900 font-medium' : 'text-stone-500'}`}
                >
                  中文
                </button>
              </div>
            </div>
            <a href="#features" className="text-stone-500 hover:text-stone-900 transition-colors text-sm whitespace-nowrap">
              {t.nav.features}
            </a>
            <a href="#best-practices" className="text-stone-500 hover:text-stone-900 transition-colors text-sm whitespace-nowrap">
              {lang === 'en' ? 'Best Practices' : '最佳实践'}
            </a>
            <a href="#quick-start" className="text-stone-500 hover:text-stone-900 transition-colors text-sm whitespace-nowrap">
              {t.nav.quickStart}
            </a>
            <a 
              href="https://github.com/LingoJack/j" 
              target="_blank" 
              rel="noopener noreferrer"
              className="flex items-center gap-2 text-stone-500 hover:text-stone-900 transition-colors"
            >
              <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 24 24">
                <path fillRule="evenodd" clipRule="evenodd" d="M12 2C6.477 2 2 6.477 2 12c0 4.42 2.87 8.17 6.84 9.5.5.08.66-.23.66-.5v-1.69c-2.77.6-3.36-1.34-3.36-1.34-.46-1.16-1.11-1.47-1.11-1.47-.91-.62.07-.6.07-.6 1 .07 1.53 1.03 1.53 1.03.87 1.52 2.34 1.07 2.91.83.09-.65.35-1.09.63-1.34-2.22-.25-4.55-1.11-4.55-4.92 0-1.11.38-2 1.03-2.71-.1-.25-.45-1.29.1-2.64 0 0 .84-.27 2.75 1.02.79-.22 1.65-.33 2.5-.33.85 0 1.71.11 2.5.33 1.91-1.29 2.75-1.02 2.75-1.02.55 1.35.2 2.39.1 2.64.65.71 1.03 1.6 1.03 2.71 0 3.82-2.34 4.66-4.57 4.91.36.31.69.92.69 1.85v2.74c0 .27.16.59.67.5C19.14 20.16 22 16.42 22 12A10 10 0 0012 2z"/>
              </svg>
              <span className="text-sm">{t.nav.github}</span>
            </a>
          </div>
        </div>
      </nav>

      {/* Hero Section */}
      <section className="pt-32 pb-16 px-6">
        <div className="max-w-4xl mx-auto">
          <div className="mb-6">
            <span className="inline-block px-3 py-1 text-xs font-medium text-stone-600 bg-white rounded-full border border-stone-200">
              {t.hero.badge}
            </span>
          </div>
          
          <h1 className="text-4xl sm:text-5xl md:text-6xl font-light text-stone-900 mb-6 leading-tight tracking-tight">
            {t.hero.title}
            <br />
            <span className="text-stone-400">{t.hero.titleHighlight}</span>
          </h1>
          
          <p className="text-lg text-stone-600 mb-8 max-w-2xl leading-relaxed">
            {t.hero.subtitle}
            <br className="hidden sm:block" />
            {t.hero.subtitleExtra}
          </p>
          
          <div className="max-w-lg mb-8">
            <CodeBlock>{installCmd}</CodeBlock>
          </div>
          
          <div className="flex flex-wrap items-center gap-4">
            <a 
              href="#quick-start"
              className="px-5 py-2.5 bg-stone-900 text-white rounded-lg font-medium text-sm hover:bg-stone-800 transition-colors"
            >
              {t.hero.getStarted}
            </a>
            <a 
              href="https://github.com/LingoJack/j"
              target="_blank"
              rel="noopener noreferrer"
              className="px-5 py-2.5 text-stone-600 hover:text-stone-900 font-medium text-sm transition-colors"
            >
              {t.hero.viewGithub}
            </a>
          </div>
        </div>
      </section>

      {/* Features Section */}
      <Section id="features" className="bg-white border-y border-stone-200">
        <div className="mb-12">
          <h2 className="text-3xl sm:text-4xl font-light text-stone-900 mb-4">
            {t.features.title}
          </h2>
          <p className="text-stone-500 max-w-lg">
            {t.features.subtitle}
          </p>
        </div>
        
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
          {t.features.list.map((feature, index) => (
            <FeatureCard key={index} {...feature} />
          ))}
        </div>
      </Section>

      {/* Quick Start Section */}
      <Section id="quick-start">
        <div className="mb-12">
          <h2 className="text-3xl sm:text-4xl font-light text-stone-900 mb-4">
            {t.quickStart.title}
          </h2>
          <p className="text-stone-500 max-w-lg">
            {t.quickStart.subtitle}
          </p>
        </div>
        
        <div className="grid md:grid-cols-2 gap-8">
          {/* Installation */}
          <div className="space-y-4">
            <h3 className="text-xs font-medium text-stone-400 uppercase tracking-wider mb-4">
              {t.quickStart.installation}
            </h3>
            <div className="space-y-3">
              <div>
                <p className="text-xs text-stone-400 mb-2">{t.quickStart.oneLineInstall}</p>
                <CodeBlock>{installCmd}</CodeBlock>
              </div>
              <div>
                <p className="text-xs text-stone-400 mb-2">{t.quickStart.cratesInstall}</p>
                <CodeBlock showCopy={true}>cargo install j-cli</CodeBlock>
              </div>
            </div>
          </div>
          
          {/* Usage */}
          <div>
            <h3 className="text-xs font-medium text-stone-400 uppercase tracking-wider mb-4">
              {t.quickStart.usageExamples}
            </h3>
            <div className="space-y-0">
              {t.quickStart.examples.map((item, index) => (
                <CommandExample key={index} {...item} />
              ))}
            </div>
          </div>
        </div>
      </Section>

      {/* More Features */}
      <Section className="bg-white border-y border-stone-200">
        <div className="text-center mb-12">
          <h2 className="text-3xl sm:text-4xl font-light text-stone-900 mb-4">
            {t.more.title}
          </h2>
        </div>
        
        <div className="grid sm:grid-cols-2 lg:grid-cols-3 gap-8">
          {t.more.list.map((item, i) => (
            <div key={i} className="text-center">
              <h3 className="font-medium text-stone-900 mb-2">{item.title}</h3>
              <p className="text-stone-500 text-sm">{item.desc}</p>
            </div>
          ))}
        </div>
      </Section>

      {/* Best Practices Section */}
      <Section id="best-practices">
        <div className="mb-12">
          <h2 className="text-3xl sm:text-4xl font-light text-stone-900 mb-4">
            {t.bestPractices.title}
          </h2>
          <p className="text-stone-500 max-w-lg">
            {t.bestPractices.subtitle}
          </p>
        </div>
        
        <div className="space-y-12">
          {t.bestPractices.categories.map((category, idx) => (
            <div key={idx}>
              <h3 className="text-lg font-medium text-stone-900 mb-4 pb-2 border-b border-stone-200">
                {category.title}
              </h3>
              <div className="grid sm:grid-cols-2 gap-4">
                {category.tips.map((tip, tipIdx) => (
                  <TipCard key={tipIdx} {...tip} />
                ))}
              </div>
            </div>
          ))}
        </div>
        
        {/* Link to docs */}
        <div className="mt-12 text-center">
          <Link 
            to="/docs" 
            className="inline-flex items-center gap-2 text-stone-600 hover:text-stone-900 transition-colors group"
          >
            <span>{lang === 'en' ? 'View full documentation' : '查看完整文档'}</span>
            <svg className="w-4 h-4 group-hover:translate-x-1 transition-transform" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M9 5l7 7-7 7" />
            </svg>
          </Link>
        </div>
      </Section>

      {/* Tech Stack */}
      <Section>
        <div className="text-center">
          <h2 className="text-2xl font-light text-stone-900 mb-8">
            {t.tech.title}
          </h2>
          <div className="flex flex-wrap justify-center gap-3">
            {['clap', 'ratatui', 'async-openai', 'serde', 'tokio'].map((tech) => (
              <span 
                key={tech}
                className="px-4 py-1.5 text-sm text-stone-600 bg-white rounded-full border border-stone-200"
              >
                {tech}
              </span>
            ))}
          </div>
        </div>
      </Section>

      {/* CTA Section */}
      <Section className="bg-stone-900 text-white">
        <div className="text-center">
          <h2 className="text-2xl sm:text-3xl font-light mb-4">
            {t.cta.title}
          </h2>
          <p className="text-stone-400 mb-8 max-w-md mx-auto">
            {t.cta.subtitle}
          </p>
          <div className="max-w-lg mx-auto">
            <div className="relative">
              <pre className="bg-[#faf9f6] text-stone-800 rounded-lg p-4 text-sm overflow-x-auto font-mono text-left border border-stone-200">
                <code>{installCmd}</code>
              </pre>
              <CopyButton text={installCmd} />
            </div>
          </div>
        </div>
      </Section>

      {/* Footer */}
      <footer className="border-t border-stone-200 py-8 px-6 bg-[#faf9f6]">
        <div className="max-w-4xl mx-auto flex flex-col sm:flex-row items-center justify-between gap-4">
          <div className="flex items-center gap-2">
            <span className="text-lg font-bold text-stone-900">j</span>
            <span className="text-stone-400 text-sm">
              {t.footer.license}
            </span>
          </div>
          <div className="flex items-center gap-6">
            <a 
              href="https://github.com/LingoJack/j" 
              target="_blank" 
              rel="noopener noreferrer"
              className="text-stone-400 hover:text-stone-900 transition-colors text-sm"
            >
              GitHub
            </a>
            <a 
              href="https://crates.io/crates/j-cli" 
              target="_blank" 
              rel="noopener noreferrer"
              className="text-stone-400 hover:text-stone-900 transition-colors text-sm"
            >
              crates.io
            </a>
          </div>
        </div>
      </footer>
    </div>
  )
}
