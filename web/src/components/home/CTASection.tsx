import { useState } from 'react'
import { Section } from '../common/Section'
import type { I18nData } from '../../types'

interface CTASectionProps {
  t: I18nData
  installCmd: string
}

export function CTASection({ t, installCmd }: CTASectionProps) {
  const [copied, setCopied] = useState(false)
  
  const handleCopy = async () => {
    await navigator.clipboard.writeText(installCmd)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }
  
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
          <div className="relative group">
            <pre className="bg-[#faf9f6] text-stone-800 rounded-lg p-4 text-sm overflow-x-auto font-mono text-left border border-stone-200 max-w-full">
              <code className="block whitespace-pre-wrap break-words sm:whitespace-pre">{installCmd}</code>
            </pre>
            <button
              onClick={handleCopy}
              className="absolute right-3 top-3 px-3 py-1.5 text-xs font-medium 
                       text-stone-600 hover:text-stone-900 bg-white hover:bg-stone-50 
                       rounded border border-stone-300 hover:border-stone-400 transition-colors shadow-sm
                       opacity-0 group-hover:opacity-100"
            >
              {copied ? 'Copied!' : 'Copy'}
            </button>
          </div>
        </div>
      </div>
    </Section>
  )
}
