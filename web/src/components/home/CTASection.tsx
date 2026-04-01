import { Section } from '../common/Section'
import { CopyButton } from '../common/CopyButton'
import type { I18nData } from '../../types'

interface CTASectionProps {
  t: I18nData
  installCmd: string
}

export function CTASection({ t, installCmd }: CTASectionProps) {
  return (
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
            <pre className="bg-[#faf9f6] text-stone-800 rounded-lg p-4 text-sm overflow-x-auto font-mono text-left border border-stone-200 max-w-full">
              <code className="block whitespace-pre-wrap break-words sm:whitespace-pre">{installCmd}</code>
            </pre>
            <CopyButton text={installCmd} />
          </div>
        </div>
      </div>
    </Section>
  )
}
