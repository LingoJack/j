import { useState, useEffect, useCallback } from 'react'
import type { I18nData } from '../../types'

interface FeaturesWithScreenshotsProps {
  t: I18nData
}

const CARD_H = 130 // px per card
const VISIBLE = 3  // cards visible at once

export function FeaturesWithScreenshots({ t }: FeaturesWithScreenshotsProps) {
  const features = t.features.list
  const screenshots = t.screenshots.list

  // Left: list auto-scroll
  const [featureIndex, setFeatureIndex] = useState(0)
  const featureNext = useCallback(() => {
    setFeatureIndex((prev) => (prev + 1) % features.length)
  }, [features.length])
  useEffect(() => {
    const timer = setInterval(featureNext, 3000)
    return () => clearInterval(timer)
  }, [featureNext])

  // Right: screenshot carousel
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
        <div className="grid grid-cols-1 lg:grid-cols-[320px_1fr] gap-8 items-start">
          {/* Left: scrolling feature list */}
          <div className="flex flex-col">
            <div className="overflow-hidden" style={{ height: CARD_H * VISIBLE }}>
              <div
                className="transition-transform duration-500 ease-in-out"
                style={{ transform: `translateY(-${featureIndex * CARD_H}px)` }}
              >
                {features.map((feature, index) => (
                  <div key={index} style={{ height: CARD_H }} className="flex items-center py-2">
                    <div className="p-4 bg-stone-50 rounded-lg border border-stone-200 w-full h-full flex flex-col justify-center">
                      <div className="flex items-center gap-2 mb-1.5">
                        <span className="text-xl">{feature.icon}</span>
                        <h3 className="text-base font-medium text-stone-900">{feature.title}</h3>
                      </div>
                      <p className="text-stone-500 text-sm leading-relaxed line-clamp-2">{feature.description}</p>
                    </div>
                  </div>
                ))}
              </div>
            </div>

            {/* Dot indicators */}
            <div className="flex gap-1.5 justify-center mt-4">
              {features.map((_, i) => (
                <button
                  key={i}
                  onClick={() => setFeatureIndex(i)}
                  className={`h-1.5 rounded-full transition-all duration-300 ${
                    i === featureIndex
                      ? 'bg-stone-800 w-5'
                      : 'bg-stone-300 w-1.5 hover:bg-stone-400'
                  }`}
                  aria-label={`Go to feature ${i + 1}`}
                />
              ))}
            </div>
          </div>

          {/* Right: screenshot carousel */}
          <div>
            {/* Main image with overlay arrows */}
            <div className="relative group rounded-lg shadow-md overflow-hidden">
              <img
                src={screenshots[shotIndex].src}
                alt={screenshots[shotIndex].alt}
                className="w-full h-auto block"
              />

              {/* Left arrow overlay */}
              <button
                onClick={shotPrev}
                className="absolute left-3 top-1/2 -translate-y-1/2 w-9 h-9 flex items-center justify-center
                           bg-white/80 backdrop-blur-sm border border-stone-200/60 rounded-full shadow-sm
                           text-stone-500 hover:text-stone-900 hover:bg-white transition-all
                           opacity-0 group-hover:opacity-100"
                aria-label="Previous screenshot"
              >
                <svg width="16" height="16" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2}>
                  <path strokeLinecap="round" strokeLinejoin="round" d="M15 19l-7-7 7-7" />
                </svg>
              </button>

              {/* Right arrow overlay */}
              <button
                onClick={shotNext}
                className="absolute right-3 top-1/2 -translate-y-1/2 w-9 h-9 flex items-center justify-center
                           bg-white/80 backdrop-blur-sm border border-stone-200/60 rounded-full shadow-sm
                           text-stone-500 hover:text-stone-900 hover:bg-white transition-all
                           opacity-0 group-hover:opacity-100"
                aria-label="Next screenshot"
              >
                <svg width="16" height="16" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2}>
                  <path strokeLinecap="round" strokeLinejoin="round" d="M9 5l7 7-7 7" />
                </svg>
              </button>
            </div>

            {/* Caption + thumbnails */}
            <div className="mt-4">
              <p className="text-stone-500 text-sm leading-relaxed mb-3">
                {screenshots[shotIndex].caption}
              </p>

              <div className="flex gap-2">
                {screenshots.map((item, i) => (
                  <button
                    key={i}
                    onClick={() => setShotIndex(i)}
                    className={`
                      relative overflow-hidden rounded-md border-2 transition-all duration-200 flex-1 aspect-video
                      ${i === shotIndex
                        ? 'border-stone-800 shadow-sm'
                        : 'border-stone-200 hover:border-stone-400 opacity-40 hover:opacity-80'
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
      </div>
    </section>
  )
}
