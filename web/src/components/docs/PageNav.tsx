import type { Language } from '../../types'

interface PageNavProps {
  lang: Language
  activeSection: string
  onNavigate: (section: string) => void
}

// Flatten doc tree to get ordered sections
function getFlatSections(lang: Language): string[] {
  const sections: string[] = []
  const tree = {
    en: {
      gettingStarted: ['installation', 'quickStart', 'dataDirectory'],
      coreFeatures: ['alias', 'report', 'todo', 'script'],
      aiFeatures: ['aiChat', 'agentMode', 'tools', 'skills', 'hooks'],
      advanced: ['browser', 'remote', 'permissions']
    },
    zh: {
      gettingStarted: ['installation', 'quickStart', 'dataDirectory'],
      coreFeatures: ['alias', 'report', 'todo', 'script'],
      aiFeatures: ['aiChat', 'agentMode', 'tools', 'skills', 'hooks'],
      advanced: ['browser', 'remote', 'permissions']
    }
  }
  
  const order = ['gettingStarted', 'coreFeatures', 'aiFeatures', 'advanced']
  order.forEach(category => {
    sections.push(...tree[lang][category as keyof typeof tree[typeof lang]])
  })
  
  return sections
}

// Section titles
const sectionTitles: Record<Language, Record<string, string>> = {
  en: {
    installation: 'Installation',
    quickStart: 'Quick Start',
    dataDirectory: 'Data Directory',
    alias: 'Alias Management',
    report: 'Daily Reports',
    todo: 'Todo Management',
    script: 'Script System',
    aiChat: 'AI Chat',
    agentMode: 'Agent Mode',
    tools: 'AI Tools',
    skills: 'Skill System',
    hooks: 'Hook System',
    browser: 'Browser Automation',
    remote: 'Remote Control',
    permissions: 'Permissions'
  },
  zh: {
    installation: '安装',
    quickStart: '快速上手',
    dataDirectory: '数据目录',
    alias: '别名管理',
    report: '日报系统',
    todo: '待办管理',
    script: '脚本系统',
    aiChat: 'AI 对话',
    agentMode: 'Agent 模式',
    tools: 'AI 工具',
    skills: 'Skill 技能',
    hooks: 'Hook 系统',
    browser: '浏览器自动化',
    remote: '远程控制',
    permissions: '权限配置'
  }
}

export function PageNav({ lang, activeSection, onNavigate }: PageNavProps) {
  const sections = getFlatSections(lang)
  const titles = sectionTitles[lang]
  const currentIndex = sections.indexOf(activeSection)
  
  const prevSection = currentIndex > 0 ? sections[currentIndex - 1] : null
  const nextSection = currentIndex < sections.length - 1 ? sections[currentIndex + 1] : null
  
  const navLabels = {
    en: { prev: 'Previous', next: 'Next' },
    zh: { prev: '上一页', next: '下一页' }
  }
  
  const labels = navLabels[lang]
  
  return (
    <div className="flex items-center justify-between py-8 mt-8 border-t border-stone-200">
      {/* Previous */}
      <div className="flex-1">
        {prevSection && (
          <button
            onClick={() => onNavigate(prevSection)}
            className="group flex flex-col items-start text-left hover:bg-stone-100 rounded-lg p-3 -ml-3 transition-colors"
          >
            <span className="text-xs text-stone-400 mb-1 flex items-center gap-1">
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2}>
                <path strokeLinecap="round" strokeLinejoin="round" d="M15 19l-7-7 7-7" />
              </svg>
              {labels.prev}
            </span>
            <span className="text-sm font-medium text-stone-700 group-hover:text-stone-900 transition-colors">
              {titles[prevSection]}
            </span>
          </button>
        )}
      </div>
      
      {/* Next */}
      <div className="flex-1 flex justify-end">
        {nextSection && (
          <button
            onClick={() => onNavigate(nextSection)}
            className="group flex flex-col items-end text-right hover:bg-stone-100 rounded-lg p-3 -mr-3 transition-colors"
          >
            <span className="text-xs text-stone-400 mb-1 flex items-center gap-1">
              {labels.next}
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2}>
                <path strokeLinecap="round" strokeLinejoin="round" d="M9 5l7 7-7 7" />
              </svg>
            </span>
            <span className="text-sm font-medium text-stone-700 group-hover:text-stone-900 transition-colors">
              {titles[nextSection]}
            </span>
          </button>
        )}
      </div>
    </div>
  )
}
