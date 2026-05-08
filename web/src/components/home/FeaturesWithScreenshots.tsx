import { useState, useEffect, useCallback } from 'react'
import type { I18nData } from '../../types'

interface FeaturesWithScreenshotsProps {
  t: I18nData
}

export function FeaturesWithScreenshots({ t }: FeaturesWithScreenshotsProps) {
  const features = t.features.list
  const screenshots = t.screenshots.list

  // Left: feature auto-scroll, one item at a time
  const [featureIndex, setFeatureIndex] = useState(0)
  const featureNext = useCallback(() => {
    setFeatureIndex((prev) => (prev + 1) % features.length)
  }, [features.length])
  useEffect(() => {
    const timer = setInterval(featureNext, 4000)
    return () => clearInterval(timer)
  }, [featureNext])

  // Right: screenshot manual carousel
  const [shotIndex, setShotIndex] = useState(0)
  const shotPrev = () => setShotIndex((prev) => (prev - 1 + screenshots.length) % screenshots.length)
  const shotNext = () => setShotIndex((prev) => (prev + 1) % screenshots.length)

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

        {/* Two-column layout */}
        <div className="grid grid-cols-1 lg:grid-cols-[340px_1fr] gap-10 items-start">
          {/* Left: vertical auto-scrolling feature list */}
          <div className="flex flex-col">
            <div className="relative overflow-hidden h-[400px]">
              <div
                className="transition-transform duration-500 ease-in-out"
                style={{ transform: `translateY(-${featureIndex * 400}px)` }}
              >
                {features.map((feature, index) => (
                  <div key={index} className="h-[400px] flex items-start pt-4">
                    <div className="p-6 bg-stone-50 rounded-xl border border-stone-200 w-full">
                      <div className="text-2xl mb-3">{feature.icon}</div>
                      <h3 className="text-lg font-medium text-stone-900 mb-2">{feature.title}</h3>
                      <p className="text-stone-600 text-sm leading-relaxed">{feature.description}</p>
                    </div>
                  </div>
                ))}
              </div>
            </div>

            {/* Dot indicators */}
            <div className="flex gap-2 justify-center mt-4">
              {features.map((_, i) => (
                <button
                  key={i}
                  onClick={() => setFeatureIndex(i)}
                  className={`h-2 rounded-full transition-all duration-300 ${
                    i === featureIndex
                      ? 'bg-stone-900 w-6'
                      : 'bg-stone-300 w-2 hover:bg-stone-400'
                  }`}
                  aria-label={`Go to feature ${i + 1}`}
                />
              ))}
            </div>
          </div>

          {/* Right: screenshot carousel */}
          <div>
            {/* Main image with arrows */}
            <div className="flex items-center gap-3">
              <button
                onClick={shotPrev}
                className="flex-shrink-0 w-10 h-10 flex items-center justify-center
                           bg-white border border-stone-200 rounded-full shadow-sm
                           text-stone-400 hover:text-stone-900 hover:border-stone-300 transition-colors"
                aria-label="Previous screenshot"
              >
                <svg width="16" height="16" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2}>
                  <path strokeLinecap="round" strokeLinejoin="round" d="M15 19l-7-7 7-7" />
                </svg>
              </button>

              <div className="flex-1 overflow-hidden rounded-lg shadow-md">
                <img
                  src={screenshots[shotIndex].src}
                  alt={screenshots[shotIndex].alt}
                  className="w-full h-auto block"
                />
              </div>

              <button
                onClick={shotNext}
                className="flex-shrink-0 w-10 h-10 flex items-center justify-center
                           bg-white border border-stone-200 rounded-full shadow-sm
                           text-stone-400 hover:text-stone-900 hover:border-stone-300 transition-colors"
                aria-label="Next screenshot"
              >
                <svg width="16" height="16" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2}>
                  <path strokeLinecap="round" strokeLinejoin="round" d="M9 5l7 7-7 7" />
                </svg>
              </button>
            </div>

            {/* Caption */}
            <p className="text-stone-500 text-sm leading-relaxed mt-4">
              {screenshots[shotIndex].caption}
            </p>

            {/* Thumbnail strip */}
            <div className="flex gap-2 mt-4">
              {screenshots.map((item, i) => (
                <button
                  key={i}
                  onClick={() => setShotIndex(i)}
                  className={`
                    relative overflow-hidden rounded-md border-2 transition-all duration-200 flex-1 aspect-video
                    ${i === shotIndex
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
