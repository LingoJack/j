import { useState } from 'react'
import { Section } from '../common/Section'
import type { I18nData } from '../../types'

interface ScreenshotsSectionProps {
  t: I18nData
}

export function ScreenshotsSection({ t }: ScreenshotsSectionProps) {
  const screenshots = t.screenshots.list
  const [activeIndex, setActiveIndex] = useState(0)
  const current = screenshots[activeIndex]

  return (
    <Section className="bg-white border-y border-stone-200">
      <div className="text-center mb-10">
        <h2 className="text-3xl sm:text-4xl font-light text-stone-900 mb-4">
          {t.screenshots.title}
        </h2>
        <p className="text-stone-500 max-w-md mx-auto">
          {t.screenshots.subtitle}
        </p>
      </div>

      {/* Main screenshot display */}
      <div className="relative mb-6">
        <div className="bg-stone-100 rounded-xl border border-stone-200 overflow-hidden shadow-sm">
          <div className="aspect-[16/10] flex items-center justify-center bg-stone-950 p-2">
            <img
              src={current.src}
              alt={current.alt}
              className="w-full h-full object-contain rounded"
            />
          </div>
        </div>
        {/* Navigation arrows */}
        <button
          onClick={() => setActiveIndex((prev) => (prev - 1 + screenshots.length) % screenshots.length)}
          className="absolute left-0 top-1/2 -translate-y-1/2 -translate-x-3 w-10 h-10 flex items-center justify-center
                     bg-white border border-stone-200 rounded-full shadow-sm
                     text-stone-500 hover:text-stone-900 hover:border-stone-300 transition-colors"
          aria-label="Previous"
        >
          <svg width="16" height="16" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M15 19l-7-7 7-7" />
          </svg>
        </button>
        <button
          onClick={() => setActiveIndex((prev) => (prev + 1) % screenshots.length)}
          className="absolute right-0 top-1/2 -translate-y-1/2 translate-x-3 w-10 h-10 flex items-center justify-center
                     bg-white border border-stone-200 rounded-full shadow-sm
                     text-stone-500 hover:text-stone-900 hover:border-stone-300 transition-colors"
          aria-label="Next"
        >
          <svg width="16" height="16" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M9 5l7 7-7 7" />
          </svg>
        </button>
      </div>

      {/* Caption */}
      <p className="text-center text-stone-600 text-sm leading-relaxed max-w-2xl mx-auto mb-6 italic">
        {current.caption}
      </p>

      {/* Thumbnail strip */}
      <div className="flex justify-center gap-3">
        {screenshots.map((item, i) => (
          <button
            key={i}
            onClick={() => setActiveIndex(i)}
            className={`
              relative overflow-hidden rounded-lg border-2 transition-all duration-200 w-20 h-14 flex-shrink-0
              ${i === activeIndex
                ? 'border-stone-900 shadow-sm ring-1 ring-stone-900/10'
                : 'border-stone-200 hover:border-stone-400 opacity-60 hover:opacity-100'
              }
            `}
          >
            <img
              src={item.src}
              alt={item.alt}
              className="w-full h-full object-cover"
            />
          </button>
        ))}
      </div>

      {/* Label for current screenshot */}
      <p className="text-center text-sm text-stone-400 mt-4">
        {current.label}
      </p>
    </Section>
  )
}