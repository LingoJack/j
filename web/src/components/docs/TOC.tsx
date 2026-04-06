import { useMemo, useEffect, useState, useRef, useCallback } from 'react'
import type { Language } from '../../types'

interface TOCItem {
  id: string
  text: string
  level: number // 2 for h2, 3 for h3, 4 for h4
}

interface TOCProps {
  content: string
  lang: Language
}

// i18n for TOC title
const tocTitleI18n: Record<Language, string> = {
  en: 'On This Page',
  zh: '本文目录'
}

// Top navigation bar height
const NAV_HEIGHT = 70

// Generate slug from text
function slugify(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^\w\u4e00-\u9fa5]+/g, '-') // Keep alphanumeric and Chinese
    .replace(/^-+|-+$/g, '')
    .slice(0, 50) // Limit length
}

// Extract headings from markdown content
function extractHeadings(content: string): TOCItem[] {
  const lines = content.split('\n')
  const headings: TOCItem[] = []
  const usedIds = new Set<string>()
  
  lines.forEach(line => {
    let text: string
    let level: number
    
    if (line.startsWith('## ')) {
      text = line.slice(3).trim()
      level = 2
    } else if (line.startsWith('### ')) {
      text = line.slice(4).trim()
      level = 3
    } else if (line.startsWith('#### ')) {
      text = line.slice(5).trim()
      level = 4
    } else {
      return
    }
    
    // Remove markdown formatting from text
    text = text.replace(/\*\*([^*]+)\*\*/g, '$1')
    text = text.replace(/\*([^*]+)\*/g, '$1')
    text = text.replace(/`([^`]+)`/g, '$1')
    
    // Generate unique id
    let id = slugify(text)
    let counter = 1
    while (usedIds.has(id)) {
      id = `${slugify(text)}-${counter}`
      counter++
    }
    usedIds.add(id)
    
    headings.push({ id, text, level })
  })
  
  return headings
}

export function TOC({ content, lang }: TOCProps) {
  const headings = useMemo(() => extractHeadings(content), [content])
  const [activeId, setActiveId] = useState<string | null>(null)
  const isScrollingRef = useRef(false)
  
  // Scroll to heading with accurate position
  const scrollToHeading = useCallback((id: string) => {
    const element = document.getElementById(id)
    if (!element) return
    
    // Immediately update active state
    setActiveId(id)
    isScrollingRef.current = true
    
    // Calculate scroll position considering nav height
    const elementTop = element.offsetTop - NAV_HEIGHT
    window.scrollTo({
      top: elementTop,
      behavior: 'smooth'
    })
    
    // Reset scrolling flag after animation
    setTimeout(() => {
      isScrollingRef.current = false
    }, 500)
  }, [])
  
  // Track scroll position to update active heading
  useEffect(() => {
    if (headings.length === 0) return
    
    const handleScroll = () => {
      // Skip during programmatic scroll
      if (isScrollingRef.current) return
      
      const scrollY = window.scrollY + NAV_HEIGHT + 20
      
      // Find the heading closest to current scroll position
      let currentId: string | null = null
      for (const heading of headings) {
        const el = document.getElementById(heading.id)
        if (el && el.offsetTop <= scrollY) {
          currentId = heading.id
        }
      }
      
      if (currentId && currentId !== activeId) {
        setActiveId(currentId)
      }
    }
    
    // Initialize on mount
    handleScroll()
    
    window.addEventListener('scroll', handleScroll, { passive: true })
    return () => window.removeEventListener('scroll', handleScroll)
  }, [headings, activeId])
  
  // Don't render if no headings
  if (headings.length === 0) {
    return null
  }
  
  return (
    <nav className="hidden xl:block fixed right-0 top-[65px] w-52 h-[calc(100vh-65px)] border-l border-stone-200/70 bg-[#faf9f6]/95 backdrop-blur-sm">
      {/* Header */}
      <div className="sticky top-0 px-4 py-3 border-b border-stone-200/50 bg-[#faf9f6]">
        <span className="text-xs font-semibold text-stone-400 uppercase tracking-wider">
          {tocTitleI18n[lang]}
        </span>
      </div>
      
      {/* Heading list */}
      <ul className="py-2 px-1 overflow-y-auto max-h-[calc(100vh-120px)]">
        {headings.map(({ id, text, level }) => {
          const isActive = activeId === id
          const isH3 = level === 3
          const isH4 = level === 4
          
          return (
            <li key={id}>
              <button
                onClick={() => scrollToHeading(id)}
                className={`
                  relative w-full text-left py-1.5 px-3 rounded-lg transition-all duration-200
                  ${isH4 
                    ? 'pl-8 text-xs' 
                    : isH3 
                      ? 'pl-6 text-xs' 
                      : 'text-sm mt-1'
                  }
                  ${isActive 
                    ? isH4
                      ? 'text-stone-700 font-medium bg-stone-50 before:absolute before:left-5 before:top-1/2 before:-translate-y-1/2 before:w-1 before:h-1 before:bg-stone-400 before:rounded-full'
                      : isH3
                        ? 'text-stone-800 font-medium bg-stone-50 before:absolute before:left-3 before:top-1/2 before:-translate-y-1/2 before:w-1 before:h-1 before:bg-stone-500 before:rounded-full'
                        : 'text-stone-900 font-medium bg-stone-100 before:absolute before:left-0 before:top-1/2 before:-translate-y-1/2 before:w-0.5 before:h-4 before:bg-stone-900 before:rounded-full'
                    : isH4
                      ? 'text-stone-400 hover:text-stone-500 hover:bg-stone-50'
                      : isH3
                        ? 'text-stone-400 hover:text-stone-600 hover:bg-stone-50'
                        : 'text-stone-500 hover:text-stone-700 hover:bg-stone-50'
                  }
                `}
              >
                {text}
              </button>
            </li>
          )
        })}
      </ul>
    </nav>
  )
}
