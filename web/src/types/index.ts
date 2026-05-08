export type Language = 'en' | 'zh'

export interface FeatureItem {
  icon: string
  title: string
  description: string
}

export interface TipItem {
  title: string
  desc: string
  example: string
}

export interface Category {
  title: string
  tips: TipItem[]
}

export interface CommandExample {
  cmd: string
  description: string
}

export interface MoreFeature {
  title: string
  desc: string
}

export interface ScreenshotItem {
  src: string
  alt: string
  caption: string
  label: string
}

export interface I18nData {
  nav: {
    features: string
    quickStart: string
    github: string
  }
  hero: {
    badge: string
    title: string
    titleHighlight: string
    subtitle: string
    subtitleExtra: string
    getStarted: string
    viewGithub: string
  }
  features: {
    title: string
    subtitle: string
    list: FeatureItem[]
  }
  quickStart: {
    title: string
    subtitle: string
    installation: string
    oneLineInstall: string
    cratesInstall: string
    usageExamples: string
    examples: CommandExample[]
  }
  more: {
    title: string
    list: MoreFeature[]
  }
  screenshots: {
    title: string
    subtitle: string
    list: ScreenshotItem[]
  }
  bestPractices: {
    title: string
    subtitle: string
    categories: Category[]
  }
  tech: {
    title: string
  }
  cta: {
    title: string
    subtitle: string
  }
  footer: {
    license: string
  }
}

export interface DocSection {
  title: string
  content: string
}

export interface DocCategory {
  title: string
  children: Record<string, string>
}

export type DocTree = Record<string, DocCategory>
