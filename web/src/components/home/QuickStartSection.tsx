import { Section } from '../common/Section'
import { CodeBlock } from '../common/CodeBlock'
import { CommandExample } from '../common/CommandExample'
import type { I18nData } from '../../types'
import type { Platform } from '../../pages/Home'

interface QuickStartSectionProps {
  t: I18nData
  installCmd: string
  platform: Platform
  onPlatformChange: (platform: Platform) => void
}

export function QuickStartSection({ t, installCmd, platform, onPlatformChange }: QuickStartSectionProps) {
  return (
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
              {/* Platform selector */}
              <div className="flex gap-2 mb-3">
                <button
                  onClick={() => onPlatformChange('unix')}
                  className={`px-2.5 py-1 text-xs font-medium rounded transition-colors ${
                    platform === 'unix'
                      ? 'bg-stone-900 text-white'
                      : 'bg-stone-100 text-stone-500 hover:bg-stone-200'
                  }`}
                >
                  macOS / Linux
                </button>
                <button
                  onClick={() => onPlatformChange('windows')}
                  className={`px-2.5 py-1 text-xs font-medium rounded transition-colors ${
                    platform === 'windows'
                      ? 'bg-stone-900 text-white'
                      : 'bg-stone-100 text-stone-500 hover:bg-stone-200'
                  }`}
                >
                  Windows
                </button>
              </div>
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
  )
}
