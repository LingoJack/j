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
      <div className="flex items-center gap-4 mb-6">
        {/* Left arrow — outside image area */}
        <button
          onClick={() => setActiveIndex((prev) => (prev - 1 + screenshots.length) % screenshots.length)}
          className="flex-shrink-0 w-10 h-10 flex items-center justify-center
                     bg-white border border-stone-200 rounded-full shadow-sm
                     text-stone-400 hover:text-stone-900 hover:border-stone-300 transition-colors"
          aria-label="Previous"
        >
          <svg width="18" height="18" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M15 19l-7-7 7-7" />
          </svg>
        </button>

        {/* Image — no extra frame, just a subtle shadow */}
        <div className="flex-1 overflow-hidden rounded-lg shadow-md">
          <img
            src={current.src}
            alt={current.alt}
            className="w-full h-auto block"
          />
        </div>

        {/* Right arrow — outside image area */}
        <button
          onClick={() => setActiveIndex((prev) => (prev + 1) % screenshots.length)}
          className="flex-shrink-0 w-10 h-10 flex items-center justify-center
                     bg-white border border-stone-200 rounded-full shadow-sm
                     text-stone-400 hover:text-stone-900 hover:border-stone-300 transition-colors"
          aria-label="Next"
        >
          <svg width="18" height="18" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2}>
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
                ? 'border-stone-900 shadow-sm'
                : 'border-stone-200 hover:border-stone-400 opacity-50 hover:opacity-100'
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
