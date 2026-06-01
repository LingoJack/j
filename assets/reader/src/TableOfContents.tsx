import { useEffect, useState } from 'react'
import { ChevronsLeft, ChevronsRight, ListTree } from './Icon'
import type { HeadingItem } from './toc'

interface Props {
  headings: HeadingItem[]
  collapsed: boolean
  onToggleCollapsed: () => void
}

export function TableOfContents({
  headings,
  collapsed,
  onToggleCollapsed,
}: Props) {
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
        type="button"
        onClick={onToggleCollapsed}
        className="seeyue-toc-rail"
        title="展开目录"
      >
        <ChevronsLeft size={16} />
      </button>
    )
  }

  if (headings.length === 0) {
    return (
      <div className="seeyue-toc-shell">
        <div className="head">
          <span className="title flex items-center gap-1.5">
            <ListTree size={14} /> Contents
          </span>
          <button
            type="button"
            className="seeyue-icon-btn"
            onClick={onToggleCollapsed}
            title="收起目录"
          >
            <ChevronsRight size={14} />
          </button>
        </div>
        <div className="px-4 pt-6 text-xs text-seeyue-fg-dim italic">
          暂无标题
        </div>
      </div>
    )
  }

  const minLevel = Math.min(...headings.map((h) => h.level))

  return (
    <nav className="seeyue-toc-shell">
      <div className="head">
        <span className="title flex items-center gap-1.5">
          <ListTree size={14} /> Contents
        </span>
        <button
          type="button"
          className="seeyue-icon-btn"
          onClick={onToggleCollapsed}
          title="收起目录"
        >
          <ChevronsRight size={14} />
        </button>
      </div>
      <ul className="seeyue-toc-list">
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
                className="row"
                data-active={isActive ? 'true' : undefined}
                style={{ paddingLeft: `${indent * 12 + 10}px` }}
                title={h.text}
              >
                {h.text}
              </a>
            </li>
          )
        })}
      </ul>
    </nav>
  )
}
