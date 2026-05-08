import { useState } from 'react'
import { FeatureCard } from '../common/FeatureCard'
import type { I18nData } from '../../types'

interface FeaturesWithScreenshotsProps {
  t: I18nData
}

export function FeaturesWithScreenshots({ t }: FeaturesWithScreenshotsProps) {
  const screenshots = t.screenshots.list
  const [activeIndex, setActiveIndex] = useState(0)
  const current = screenshots[activeIndex]

  return (
    <section id="features" className="py-16 md:py-24 px-6 bg-white border-y border-stone-200">
      <div className="max-w-6xl mx-auto">
        {/* Title */}
        <div className="mb-12">
          <h2 className="text-3xl sm:text-4xl font-light text-stone-900 mb-4">
            {t.features.title}
          </h2>
          <p className="text-stone-500 max-w-lg">
            {t.features.subtitle}
          </p>
        </div>

        {/* Two-column layout: features left, screenshots right */}
        <div className="grid grid-cols-1 lg:grid-cols-[1fr_1.2fr] gap-10 items-start">
          {/* Left: feature cards */}
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-2 gap-4">
            {t.features.list.map((feature, index) => (
              <FeatureCard key={index} {...feature} />
            ))}
          </div>

          {/* Right: screenshot carousel */}
          <div>
            {/* Main image */}
            <div className="flex items-center gap-3 mb-4">
              <button
                onClick={() => setActiveIndex((prev) => (prev - 1 + screenshots.length) % screenshots.length)}
                className="flex-shrink-0 w-9 h-9 flex items-center justify-center
                           bg-stone-50 border border-stone-200 rounded-full
                           text-stone-400 hover:text-stone-900 hover:border-stone-300 transition-colors"
                aria-label="Previous"
              >
                <svg width="16" height="16" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2}>
                  <path strokeLinecap="round" strokeLinejoin="round" d="M15 19l-7-7 7-7" />
                </svg>
              </button>

              <div className="flex-1 overflow-hidden rounded-lg shadow-md">
                <img
                  src={current.src}
                  alt={current.alt}
                  className="w-full h-auto block"
                />
              </div>

              <button
                onClick={() => setActiveIndex((prev) => (prev + 1) % screenshots.length)}
                className="flex-shrink-0 w-9 h-9 flex items-center justify-center
                           bg-stone-50 border border-stone-200 rounded-full
                           text-stone-400 hover:text-stone-900 hover:border-stone-300 transition-colors"
                aria-label="Next"
              >
                <svg width="16" height="16" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2}>
                  <path strokeLinecap="round" strokeLinejoin="round" d="M9 5l7 7-7 7" />
                </svg>
              </button>
            </div>

            {/* Caption */}
            <p className="text-stone-600 text-sm leading-relaxed mb-4">
              {current.caption}
            </p>

            {/* Thumbnails */}
            <div className="flex gap-2">
              {screenshots.map((item, i) => (
                <button
                  key={i}
                  onClick={() => setActiveIndex(i)}
                  className={`
                    relative overflow-hidden rounded-md border-2 transition-all duration-200 flex-1 aspect-video
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
          </div>
        </div>
      </div>
    </section>
  )
}
