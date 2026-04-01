import { useState } from 'react'
import { Link } from 'react-router-dom'

type Lang = 'en' | 'zh'

const i18n = {
  en: {
    nav: {
      back: '← Back to Home',
      github: 'GitHub'
    },
    hero: {
      title: 'Documentation',
      subtitle: 'Complete guide to jcli commands and features.'
    },
    sections: {
      alias: {
        title: 'Alias Management',
        commands: [
          {
            cmd: 'j set <name> <path>',
            desc: 'Register a new alias',
            examples: [
              'j set chrome "/Applications/Google Chrome.app"',
              'j set github https://github.com',
              'j set code ~/projects'
            ]
          },
          {
            cmd: 'j <alias>',
            desc: 'Open the registered alias',
            examples: [
              'j chrome',
              'j github',
              'j code'
            ]
          },
          {
            cmd: 'j <alias> <text>',
            desc: 'Search with browser alias',
            examples: [
              'j chrome React tutorial',
              'j chrome Rust docs baidu'
            ]
          },
          {
            cmd: 'j note <alias> <type>',
            desc: 'Mark alias type (browser/editor/vpn/script/inner_url/outer_url)',
            examples: [
              'j note chrome browser',
              'j note portal outer_url'
            ]
          },
          {
            cmd: 'j find <keyword> [types]',
            desc: 'Find aliases, optionally filter by types',
            examples: [
              'j find chrome',
              'j find chrome browser,vpn'
            ]
          },
          {
            cmd: 'j rn <old> <new>',
            desc: 'Rename an alias',
            examples: [
              'j rn chrome browser'
            ]
          },
          {
            cmd: 'j rm <alias>',
            desc: 'Remove an alias',
            examples: [
              'j rm chrome'
            ]
          },
          {
            cmd: 'j ls',
            desc: 'List all aliases',
            examples: []
          }
        ]
      },
      report: {
        title: 'Daily Reports',
        commands: [
          {
            cmd: 'j report [content...]',
            desc: 'Write to daily report (opens TUI if no content)',
            examples: [
              'j report',
              'j report Completed user module',
              'j report "Fixed bug #123"'
            ]
          },
          {
            cmd: 'j check [n]',
            desc: 'View recent n lines of report (default 5)',
            examples: [
              'j check',
              'j check 10'
            ]
          },
          {
            cmd: 'j search <scope> <keyword> [options]',
            desc: 'Search in reports (scope: all/week/month)',
            examples: [
              'j search all user',
              'j search week bug -fuzzy'
            ]
          },
          {
            cmd: 'j rctl new',
            desc: 'Create new week report',
            examples: []
          },
          {
            cmd: 'j rctl push',
            desc: 'Push reports to git',
            examples: []
          },
          {
            cmd: 'j rctl pull',
            desc: 'Pull reports from git',
            examples: []
          }
        ]
      },
      todo: {
        title: 'Todo Management',
        commands: [
          {
            cmd: 'j todo',
            desc: 'Open TUI todo manager',
            examples: []
          },
          {
            cmd: 'j todo add <content>',
            desc: 'Add a new todo item',
            examples: [
              'j todo add Review PR',
              'j todo add "Write documentation"'
            ]
          },
          {
            cmd: 'j todo list [options]',
            desc: 'List todos (options: --done, --undone)',
            examples: [
              'j todo list',
              'j todo list --undone'
            ]
          },
          {
            cmd: 'j todo done <id>',
            desc: 'Mark todo as done',
            examples: [
              'j todo done 1'
            ]
          },
          {
            cmd: 'j todo rm <id>',
            desc: 'Remove a todo item',
            examples: [
              'j todo rm 1'
            ]
          }
        ]
      },
      script: {
        title: 'Script System',
        commands: [
          {
            cmd: 'j concat <name> "<content>"',
            desc: 'Create a new script with content (auto-registers to script section)',
            examples: [
              'j concat open "open $1"',
              'j concat pull "j rctl pull"'
            ]
          },
          {
            cmd: 'j concat <name>',
            desc: 'Create script with TUI editor (no content provided)',
            examples: [
              'j concat deploy'
            ]
          },
          {
            cmd: 'j <script> [args...]',
            desc: 'Execute a script with arguments',
            examples: [
              'j open README.md',
              'j deploy staging'
            ]
          },
          {
            cmd: 'j <script> -w [args...]',
            desc: 'Execute script in new terminal window',
            examples: [
              'j open -w README.md'
            ]
          }
        ],
        examples: [
          {
            title: 'Script: open.sh',
            content: 'open $1',
            desc: 'Simple file opener'
          },
          {
            title: 'Script: ll.sh',
            content: 'j ls',
            desc: 'List all aliases'
          },
          {
            title: 'Script: pull.sh',
            content: 'j rctl pull',
            desc: 'Pull reports from git'
          },
          {
            title: 'Script: push.sh',
            content: 'j rctl push',
            desc: 'Push reports to git'
          }
        ]
      },
      chat: {
        title: 'AI Chat',
        commands: [
          {
            cmd: 'j chat',
            desc: 'Enter AI chat TUI',
            examples: []
          },
          {
            cmd: '@file:<path>',
            desc: 'Include local file as context',
            examples: [
              '@file:src/main.rs Explain this code'
            ]
          },
          {
            cmd: '/compact [focus:...]',
            desc: 'Compress conversation to free context window',
            examples: [
              '/compact',
              '/compact focus:current task'
            ]
          }
        ],
        features: [
          'Multi-model support (OpenAI, Anthropic, etc.)',
          'Streaming output',
          'Tool calling (Read, Write, Bash, WebFetch, etc.)',
          'Fine-grained permission control',
          'Remote control via mobile',
          'Agent mode for autonomous task execution'
        ]
      },
      browser: {
        title: 'Browser Automation',
        modes: [
          {
            name: 'Lite Mode',
            desc: 'Lightweight HTTP control for simple operations',
            features: ['Navigate URLs', 'Execute JavaScript', 'Fast and efficient']
          },
          {
            name: 'CDP Mode',
            desc: 'Full browser automation via Chrome DevTools Protocol',
            features: ['Screenshots', 'Click & Type', 'Scroll & Drag', 'Full DOM access']
          }
        ]
      }
    }
  },
  zh: {
    nav: {
      back: '← 返回首页',
      github: 'GitHub'
    },
    hero: {
      title: '文档',
      subtitle: 'jcli 命令和功能完整指南。'
    },
    sections: {
      alias: {
        title: '别名管理',
        commands: [
          {
            cmd: 'j set <别名> <路径>',
            desc: '注册新别名',
            examples: [
              'j set chrome "/Applications/Google Chrome.app"',
              'j set github https://github.com',
              'j set code ~/projects'
            ]
          },
          {
            cmd: 'j <别名>',
            desc: '打开注册的别名',
            examples: [
              'j chrome',
              'j github',
              'j code'
            ]
          },
          {
            cmd: 'j <别名> <文本>',
            desc: '用浏览器别名搜索',
            examples: [
              'j chrome React 教程',
              'j chrome Rust 文档 baidu'
            ]
          },
          {
            cmd: 'j note <别名> <类型>',
            desc: '标记别名类型（browser/editor/vpn/script/inner_url/outer_url）',
            examples: [
              'j note chrome browser',
              'j note portal outer_url'
            ]
          },
          {
            cmd: 'j find <关键字> [类型]',
            desc: '查找别名，可按类型过滤',
            examples: [
              'j find chrome',
              'j find chrome browser,vpn'
            ]
          },
          {
            cmd: 'j rn <旧名> <新名>',
            desc: '重命名别名',
            examples: [
              'j rn chrome browser'
            ]
          },
          {
            cmd: 'j rm <别名>',
            desc: '删除别名',
            examples: [
              'j rm chrome'
            ]
          },
          {
            cmd: 'j ls',
            desc: '列出所有别名',
            examples: []
          }
        ]
      },
      report: {
        title: '日报系统',
        commands: [
          {
            cmd: 'j report [内容...]',
            desc: '写入日报（无内容时打开 TUI）',
            examples: [
              'j report',
              'j report 完成用户模块',
              'j report "修复 bug #123"'
            ]
          },
          {
            cmd: 'j check [n]',
            desc: '查看最近 n 行日报（默认 5 行）',
            examples: [
              'j check',
              'j check 10'
            ]
          },
          {
            cmd: 'j search <范围> <关键字> [选项]',
            desc: '搜索日报（范围：all/week/month）',
            examples: [
              'j search all 用户',
              'j search week bug -fuzzy'
            ]
          },
          {
            cmd: 'j rctl new',
            desc: '创建新周报',
            examples: []
          },
          {
            cmd: 'j rctl push',
            desc: '推送日报到 git',
            examples: []
          },
          {
            cmd: 'j rctl pull',
            desc: '从 git 拉取日报',
            examples: []
          }
        ]
      },
      todo: {
        title: '待办管理',
        commands: [
          {
            cmd: 'j todo',
            desc: '打开 TUI 待办管理器',
            examples: []
          },
          {
            cmd: 'j todo add <内容>',
            desc: '添加新待办',
            examples: [
              'j todo add 审查代码',
              'j todo add "编写文档"'
            ]
          },
          {
            cmd: 'j todo list [选项]',
            desc: '列出待办（选项：--done, --undone）',
            examples: [
              'j todo list',
              'j todo list --undone'
            ]
          },
          {
            cmd: 'j todo done <id>',
            desc: '标记待办为完成',
            examples: [
              'j todo done 1'
            ]
          },
          {
            cmd: 'j todo rm <id>',
            desc: '删除待办',
            examples: [
              'j todo rm 1'
            ]
          }
        ]
      },
      script: {
        title: '脚本系统',
        commands: [
          {
            cmd: 'j concat <名称> "<内容>"',
            desc: '创建脚本并指定内容（自动注册到 script 分类）',
            examples: [
              'j concat open "open $1"',
              'j concat pull "j rctl pull"'
            ]
          },
          {
            cmd: 'j concat <名称>',
            desc: '使用 TUI 编辑器创建脚本（不提供内容时）',
            examples: [
              'j concat deploy'
            ]
          },
          {
            cmd: 'j <脚本> [参数...]',
            desc: '执行脚本并传递参数',
            examples: [
              'j open README.md',
              'j deploy staging'
            ]
          },
          {
            cmd: 'j <脚本> -w [参数...]',
            desc: '在新终端窗口执行脚本',
            examples: [
              'j open -w README.md'
            ]
          }
        ],
        examples: [
          {
            title: '脚本: open.sh',
            content: 'open $1',
            desc: '简单文件打开器'
          },
          {
            title: '脚本: ll.sh',
            content: 'j ls',
            desc: '列出所有别名'
          },
          {
            title: '脚本: pull.sh',
            content: 'j rctl pull',
            desc: '从 git 拉取日报'
          },
          {
            title: '脚本: push.sh',
            content: 'j rctl push',
            desc: '推送日报到 git'
          }
        ]
      },
      chat: {
        title: 'AI 对话',
        commands: [
          {
            cmd: 'j chat',
            desc: '进入 AI 对话 TUI',
            examples: []
          },
          {
            cmd: '@file:<路径>',
            desc: '将本地文件作为上下文',
            examples: [
              '@file:src/main.rs 解释这段代码'
            ]
          },
          {
            cmd: '/compact [focus:...]',
            desc: '压缩对话以释放上下文窗口',
            examples: [
              '/compact',
              '/compact focus:当前任务'
            ]
          }
        ],
        features: [
          '多模型支持（OpenAI、Anthropic 等）',
          '流式输出',
          '工具调用（Read、Write、Bash、WebFetch 等）',
          '细粒度权限控制',
          '手机远程控制',
          'Agent 模式自主执行任务'
        ]
      },
      browser: {
        title: '浏览器自动化',
        modes: [
          {
            name: 'Lite 模式',
            desc: '轻量级 HTTP 控制，适合简单操作',
            features: ['导航 URL', '执行 JavaScript', '快速高效']
          },
          {
            name: 'CDP 模式',
            desc: '通过 Chrome DevTools Protocol 完整控制浏览器',
            features: ['截图', '点击与输入', '滚动与拖拽', '完整 DOM 访问']
          }
        ]
      }
    }
  }
}

// Command block component
function CommandBlock({ cmd, desc, examples }: { cmd: string; desc: string; examples: string[] }) {
  return (
    <div className="mb-6">
      <div className="bg-[#faf9f6] rounded-lg p-4 border border-stone-200 mb-2">
        <code className="text-stone-800 font-mono text-sm">{cmd}</code>
      </div>
      <p className="text-stone-600 text-sm mb-2">{desc}</p>
      {examples.length > 0 && (
        <div className="pl-4 border-l-2 border-stone-200">
          {examples.map((ex, i) => (
            <code key={i} className="block text-stone-500 text-xs font-mono mb-1">
              $ {ex}
            </code>
          ))}
        </div>
      )}
    </div>
  )
}

// Script example component
function ScriptExample({ title, content, desc }: { title: string; content: string; desc: string }) {
  return (
    <div className="bg-white rounded-lg border border-stone-200 p-4">
      <div className="flex items-center justify-between mb-2">
        <span className="text-sm font-medium text-stone-900">{title}</span>
        <span className="text-xs text-stone-400">{desc}</span>
      </div>
      <div className="bg-[#faf9f6] rounded px-3 py-2 border border-stone-200">
        <code className="text-stone-700 text-xs font-mono">{content}</code>
      </div>
    </div>
  )
}

// Section component
function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="py-12 border-b border-stone-200 last:border-0">
      <h2 className="text-2xl font-light text-stone-900 mb-6">{title}</h2>
      {children}
    </section>
  )
}

export default function Docs() {
  const [lang, setLang] = useState<Lang>('zh')  // 默认中文
  const [langMenuOpen, setLangMenuOpen] = useState(false)
  const t = i18n[lang]

  return (
    <div className="min-h-screen bg-[#faf9f6] text-stone-800">
      {/* Navigation */}
      <nav className="fixed top-0 left-0 right-0 z-50 bg-[#faf9f6]/90 backdrop-blur-sm border-b border-stone-200/50">
        <div className="max-w-4xl mx-auto px-6 py-4 flex items-center justify-between">
          <Link to="/" className="flex items-center gap-2">
            <span className="text-2xl font-bold text-stone-900">j</span>
            <span className="text-stone-400 text-sm hidden sm:inline">docs</span>
          </Link>
          <div className="flex items-center gap-5">
            {/* Language Switcher */}
            <div className="relative">
              <button 
                onClick={() => setLangMenuOpen(!langMenuOpen)}
                onBlur={() => setTimeout(() => setLangMenuOpen(false), 150)}
                className="text-stone-500 hover:text-stone-900 transition-colors text-sm flex items-center gap-0.5"
              >
                {lang === 'en' ? 'EN' : '中文'}
                <svg className={`w-3 h-3 ml-0.5 transition-transform ${langMenuOpen ? 'rotate-180' : ''}`} fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2}>
                  <path strokeLinecap="round" strokeLinejoin="round" d="M19 9l-7 7-7-7" />
                </svg>
              </button>
              {langMenuOpen && (
                <div className="absolute top-full left-0 mt-1 bg-white rounded shadow-lg py-1 z-50 whitespace-nowrap min-w-[60px]">
                  <button
                    onClick={() => { setLang('en'); setLangMenuOpen(false); }}
                    className={`block w-full text-left px-3 py-1.5 text-sm hover:bg-stone-50 ${lang === 'en' ? 'text-stone-900 font-medium' : 'text-stone-500'}`}
                  >
                    EN
                  </button>
                  <button
                    onClick={() => { setLang('zh'); setLangMenuOpen(false); }}
                    className={`block w-full text-left px-3 py-1.5 text-sm hover:bg-stone-50 ${lang === 'zh' ? 'text-stone-900 font-medium' : 'text-stone-500'}`}
                  >
                    中文
                  </button>
                </div>
              )}
            </div>
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

      {/* Hero */}
      <section className="pt-32 pb-12 px-6">
        <div className="max-w-4xl mx-auto">
          <h1 className="text-4xl sm:text-5xl font-light text-stone-900 mb-4">
            {t.hero.title}
          </h1>
          <p className="text-lg text-stone-600">
            {t.hero.subtitle}
          </p>
        </div>
      </section>

      {/* Content */}
      <main className="px-6 pb-16">
        <div className="max-w-4xl mx-auto">
          {/* Alias Management */}
          <Section title={t.sections.alias.title}>
            {t.sections.alias.commands.map((cmd, i) => (
              <CommandBlock key={i} {...cmd} />
            ))}
          </Section>

          {/* Daily Reports */}
          <Section title={t.sections.report.title}>
            {t.sections.report.commands.map((cmd, i) => (
              <CommandBlock key={i} {...cmd} />
            ))}
          </Section>

          {/* Todo */}
          <Section title={t.sections.todo.title}>
            {t.sections.todo.commands.map((cmd, i) => (
              <CommandBlock key={i} {...cmd} />
            ))}
          </Section>

          {/* Scripts */}
          <Section title={t.sections.script.title}>
            {t.sections.script.commands.map((cmd, i) => (
              <CommandBlock key={i} {...cmd} />
            ))}
            <h3 className="text-lg font-medium text-stone-900 mb-4 mt-8">
              {lang === 'en' ? 'Example Scripts' : '脚本示例'}
            </h3>
            <div className="grid sm:grid-cols-2 gap-4">
              {t.sections.script.examples.map((ex, i) => (
                <ScriptExample key={i} {...ex} />
              ))}
            </div>
          </Section>

          {/* AI Chat */}
          <Section title={t.sections.chat.title}>
            {t.sections.chat.commands.map((cmd, i) => (
              <CommandBlock key={i} {...cmd} />
            ))}
            <h3 className="text-lg font-medium text-stone-900 mb-4 mt-8">
              {lang === 'en' ? 'Features' : '功能特性'}
            </h3>
            <ul className="space-y-2">
              {t.sections.chat.features.map((feature, i) => (
                <li key={i} className="flex items-start gap-2 text-stone-600 text-sm">
                  <span className="text-stone-400 mt-1">•</span>
                  {feature}
                </li>
              ))}
            </ul>
          </Section>

          {/* Browser Automation */}
          <Section title={t.sections.browser.title}>
            <div className="grid sm:grid-cols-2 gap-6">
              {t.sections.browser.modes.map((mode, i) => (
                <div key={i} className="bg-white rounded-lg border border-stone-200 p-6">
                  <h3 className="text-lg font-medium text-stone-900 mb-2">{mode.name}</h3>
                  <p className="text-stone-600 text-sm mb-4">{mode.desc}</p>
                  <ul className="space-y-1">
                    {mode.features.map((f, j) => (
                      <li key={j} className="text-stone-500 text-sm flex items-center gap-2">
                        <span className="text-stone-400">✓</span>
                        {f}
                      </li>
                    ))}
                  </ul>
                </div>
              ))}
            </div>
          </Section>
        </div>
      </main>

      {/* Footer */}
      <footer className="border-t border-stone-200 py-8 px-6 bg-[#faf9f6]">
        <div className="max-w-4xl mx-auto flex items-center justify-between">
          <Link to="/" className="text-stone-500 hover:text-stone-900 transition-colors text-sm">
            {t.nav.back}
          </Link>
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
