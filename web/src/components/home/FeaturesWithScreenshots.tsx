import { useState, useEffect, useRef, useCallback } from 'react'
import type { I18nData } from '../../types'

interface FeaturesWithScreenshotsProps {
  t: I18nData
}

export function FeaturesWithScreenshots({ t }: FeaturesWithScreenshotsProps) {
  const features = t.features.list
  const screenshots = t.screenshots.list

  // --- Left: smooth infinite vertical scroll ---
  const scrollRef = useRef<HTMLDivElement>(null)
  const offsetRef = useRef(0)
  const rafRef = useRef(0)

  // Refs to sync left height with right
  const leftRef = useRef<HTMLDivElement>(null)
  const rightRef = useRef<HTMLDivElement>(null)

  const syncHeight = useCallback(() => {
    if (leftRef.current && rightRef.current) {
      leftRef.current.style.height = `${rightRef.current.offsetHeight}px`
    }
  }, [])

  useEffect(() => {
    const el = scrollRef.current
    if (!el) return
    const speed = 0.4

    const tick = () => {
      offsetRef.current += speed
      const halfH = el.scrollHeight / 2
      if (offsetRef.current >= halfH) {
        offsetRef.current -= halfH
      }
      el.style.transform = `translateY(-${offsetRef.current}px)`
      rafRef.current = requestAnimationFrame(tick)
    }
    rafRef.current = requestAnimationFrame(tick)
    return () => cancelAnimationFrame(rafRef.current)
  }, [])

  // Sync left height to right after mount and on resize
  useEffect(() => {
    syncHeight()
    const observer = new ResizeObserver(syncHeight)
    if (rightRef.current) observer.observe(rightRef.current)
    return () => observer.disconnect()
  }, [syncHeight])

  // --- Right: screenshot carousel ---
  const [shotIndex, setShotIndex] = useState(0)
  const shotPrev = () => setShotIndex((prev) => (prev - 1 + screenshots.length) % screenshots.length)
  const shotNext = () => setShotIndex((prev) => (prev + 1) % screenshots.length)

  return (
    <section id="features" className="py-16 md:py-24 px-6 bg-white border-y border-stone-200">
      <div className="max-w-7xl mx-auto">
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
        <div className="grid grid-cols-1 lg:grid-cols-[300px_1fr] gap-8 lg:gap-10 items-start">

          {/* Left: smooth infinite scrolling feature list */}
          <div ref={leftRef} className="hidden lg:block overflow-hidden">
            <div ref={scrollRef}>
              {[...features, ...features].map((feature, index) => (
                <div key={index} className="py-2">
                  <div className="p-4 bg-stone-50 rounded-lg border border-stone-200">
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

          {/* Right: screenshot carousel */}
          <div ref={rightRef} className="flex flex-col">
            {/* Main image with overlay arrows */}
            <div className="relative group rounded-lg shadow-md overflow-hidden border border-stone-300">
              <img
                src={screenshots[shotIndex].src}
                alt={screenshots[shotIndex].alt}
                className="w-full h-auto block"
              />

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
