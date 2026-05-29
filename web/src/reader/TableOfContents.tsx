import { useEffect, useState } from 'react'
import type { HeadingItem } from './toc'

interface Props {
  headings: HeadingItem[]
  collapsed: boolean
  onToggleCollapsed: () => void
}

export function TableOfContents({ headings, collapsed, onToggleCollapsed }: Props) {
  const [activeId, setActiveId] = useState<string>('')

  useEffect(() => {
    if (headings.length === 0 || collapsed) return
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            setActiveId(entry.target.id)
          }
        }
      },
      { rootMargin: '-80px 0px -60% 0px' },
    )
    for (const h of headings) {
      const el = document.getElementById(h.id)
      if (el) observer.observe(el)
    }
    return () => observer.disconnect()
  }, [headings, collapsed])

  if (collapsed) {
    return (
      <button
        onClick={onToggleCollapsed}
        className="h-full w-full flex items-start justify-center pt-4 text-seeyue-fg-dim hover:text-seeyue-fg transition-colors"
        title="展开目录"
      >
        <span className="text-xs">«</span>
      </button>
    )
  }

  if (headings.length === 0) {
    return (
      <div className="px-4 py-4 text-seeyue-fg-dim text-xs flex items-start justify-between">
        <span>无标题</span>
        <button
          onClick={onToggleCollapsed}
          className="text-seeyue-fg-dim hover:text-seeyue-fg transition-colors"
          title="收起目录"
        >
          »
        </button>
      </div>
    )
  }

  const minLevel = Math.min(...headings.map((h) => h.level))

  return (
    <nav className="text-xs px-3 py-4">
      <div className="flex items-center justify-between mb-3 px-1">
        <span className="text-seeyue-fg-dim uppercase tracking-wider text-[10px] font-medium">
          目录
        </span>
        <button
          onClick={onToggleCollapsed}
          className="text-seeyue-fg-dim hover:text-seeyue-fg transition-colors text-xs"
          title="收起目录"
        >
          »
        </button>
      </div>
      <ul className="space-y-1 border-l border-seeyue-border">
        {headings.map((h) => {
          const indent = h.level - minLevel
          const isActive = h.id === activeId
          return (
            <li key={h.id}>
              <a
                href={`#${h.id}`}
                onClick={(e) => {
                  e.preventDefault()
                  document
                    .getElementById(h.id)
                    ?.scrollIntoView({ behavior: 'smooth' })
                }}
                className={`block truncate transition-colors ${
                  isActive
                    ? 'text-seeyue-fg-strong font-medium'
                    : 'text-seeyue-fg-muted hover:text-seeyue-fg'
                }`}
                style={{ paddingLeft: `${indent * 12 + 10}px` }}
                title={h.text}
              >
                <span
                  className={`inline-block border-l-2 -ml-px pl-2 py-0.5 transition-colors ${
                    isActive
                      ? 'border-seeyue-accent'
                      : 'border-transparent'
                  }`}
                >
                  {h.text}
                </span>
              </a>
            </li>
          )
        })}
      </ul>
    </nav>
  )
}
