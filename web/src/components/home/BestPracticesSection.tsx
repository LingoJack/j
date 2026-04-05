import { Link } from 'react-router-dom'
import { Section } from '../common/Section'
import { TipCard } from '../common/TipCard'
import type { Language, I18nData } from '../../types'

interface BestPracticesSectionProps {
  lang: Language
  t: I18nData
}

export function BestPracticesSection({ lang, t }: BestPracticesSectionProps) {
  return (
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
            <div className="grid sm:grid-cols-2 gap-1">
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
  )
}
