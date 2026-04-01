import type { Language } from '../../types'

// Document tree structure for sidebar navigation
export const docTree: Record<Language, Record<string, { title: string; children: Record<string, string> }>> = {
  en: {
    gettingStarted: {
      title: 'Getting Started',
      children: {
        installation: 'Installation',
        quickStart: 'Quick Start',
        dataDirectory: 'Data Directory'
      }
    },
    coreFeatures: {
      title: 'Core Features',
      children: {
        alias: 'Alias Management',
        report: 'Daily Reports',
        todo: 'Todo Management',
        script: 'Script System'
      }
    },
    aiFeatures: {
      title: 'AI Features',
      children: {
        aiChat: 'AI Chat',
        agentMode: 'Agent Mode',
        tools: 'AI Tools',
        skills: 'Skill System',
        hooks: 'Hook System'
      }
    },
    advanced: {
      title: 'Advanced',
      children: {
        browser: 'Browser Automation',
        remote: 'Remote Control',
        permissions: 'Permissions'
      }
    }
  },
  zh: {
    gettingStarted: {
      title: '快速开始',
      children: {
        installation: '安装',
        quickStart: '快速上手',
        dataDirectory: '数据目录'
      }
    },
    coreFeatures: {
      title: '核心功能',
      children: {
        alias: '别名管理',
        report: '日报系统',
        todo: '待办管理',
        script: '脚本系统'
      }
    },
    aiFeatures: {
      title: 'AI 功能',
      children: {
        aiChat: 'AI 对话',
        agentMode: 'Agent 模式',
        tools: 'AI 工具',
        skills: 'Skill 技能',
        hooks: 'Hook 系统'
      }
    },
    advanced: {
      title: '进阶功能',
      children: {
        browser: '浏览器自动化',
        remote: '远程控制',
        permissions: '权限配置'
      }
    }
  }
}

// Navigation i18n
export const docNavI18n = {
  en: {
    back: '← Back to Home',
    github: 'GitHub',
    menu: 'Menu'
  },
  zh: {
    back: '← 返回首页',
    github: 'GitHub',
    menu: '菜单'
  }
}

// Section titles for PageNav
export const sectionTitles: Record<Language, Record<string, string>> = {
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

// Get ordered sections for navigation
export function getOrderedSections(): string[] {
  return [
    'installation', 'quickStart', 'dataDirectory',
    'alias', 'report', 'todo', 'script',
    'aiChat', 'agentMode', 'tools', 'skills', 'hooks',
    'browser', 'remote', 'permissions'
  ]
}

// Import all markdown files as raw strings
const mdFilesEn = import.meta.glob<{ default: string }>(
  './en/*.md',
  { eager: true, query: '?raw' }
)

const mdFilesZh = import.meta.glob<{ default: string }>(
  './zh/*.md',
  { eager: true, query: '?raw' }
)

// Build content map from imported files
function buildContentMap(): Record<Language, Record<string, string>> {
  const contentMap: Record<Language, Record<string, string>> = {
    en: {},
    zh: {}
  }

  // Process English files
  for (const [path, module] of Object.entries(mdFilesEn)) {
    const match = path.match(/\.\/en\/(\w+)\.md$/)
    if (match && module?.default) {
      contentMap.en[match[1]] = module.default
    }
  }

  // Process Chinese files
  for (const [path, module] of Object.entries(mdFilesZh)) {
    const match = path.match(/\.\/zh\/(\w+)\.md$/)
    if (match && module?.default) {
      contentMap.zh[match[1]] = module.default
    }
  }

  return contentMap
}

export const docContent = buildContentMap()

// Get document content by language and section
export function getDocContent(lang: Language, section: string): string {
  return docContent[lang]?.[section] || docContent.en[section] || ''
}

// Get section title
export function getSectionTitle(lang: Language, section: string): string {
  return sectionTitles[lang]?.[section] || sectionTitles.en[section] || section
}
