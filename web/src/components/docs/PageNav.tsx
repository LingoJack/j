import type { Language } from '../../types'
import { getOrderedSections, sectionTitles } from '../../data/docs'

interface PageNavProps {
  lang: Language
  activeSection: string
  onNavigate: (section: string) => void
}

const navLabels = {
  en: { prev: 'Previous', next: 'Next' },
  zh: { prev: '上一页', next: '下一页' }
}

export function PageNav({ lang, activeSection, onNavigate }: PageNavProps) {
  const sections = getOrderedSections()
  const titles = sectionTitles[lang]
  const labels = navLabels[lang]
  const currentIndex = sections.indexOf(activeSection)
  
  const prevSection = currentIndex > 0 ? sections[currentIndex - 1] : null
  const nextSection = currentIndex < sections.length - 1 ? sections[currentIndex + 1] : null
  
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
