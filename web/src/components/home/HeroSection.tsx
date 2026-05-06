import { CodeBlock } from '../common/CodeBlock'
import type { I18nData } from '../../types'
import type { Platform } from '../../pages/Home'

interface HeroSectionProps {
  t: I18nData
  installCmd: string
  platform: Platform
  onPlatformChange: (platform: Platform) => void
}

export function HeroSection({ t, installCmd, platform, onPlatformChange }: HeroSectionProps) {
  return (
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
        
        {/* Platform selector */}
        <div className="flex gap-2 mb-4">
          <button
            onClick={() => onPlatformChange('unix')}
            className={`px-3 py-1.5 text-xs font-medium rounded-lg transition-colors ${
              platform === 'unix'
                ? 'bg-stone-900 text-white'
                : 'bg-stone-100 text-stone-600 hover:bg-stone-200'
            }`}
          >
            macOS / Linux
          </button>
          <button
            onClick={() => onPlatformChange('windows')}
            className={`px-3 py-1.5 text-xs font-medium rounded-lg transition-colors ${
              platform === 'windows'
                ? 'bg-stone-900 text-white'
                : 'bg-stone-100 text-stone-600 hover:bg-stone-200'
            }`}
          >
            Windows
          </button>
        </div>
        
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
  )
}
