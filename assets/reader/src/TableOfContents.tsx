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
        className="h-full flex items-start justify-center pt-3.5 bg-seeyue-sidebar text-seeyue-fg-dim cursor-pointer transition-colors duration-150 hover:text-seeyue-fg-strong"
        title="展开目录"
      >
        <ChevronsLeft size={16} />
      </button>
    )
  }

  if (headings.length === 0) {
    return (
      <div className="h-full flex flex-col bg-seeyue-sidebar">
        <div className="flex items-center justify-between px-3.5 pt-[14px] pb-1">
          <span className="font-cjk text-sm font-medium text-seeyue-fg-strong tracking-[0.04em] relative pb-1 after:content-[''] after:absolute after:left-0 after:bottom-[-1px] after:w-7 after:h-0.5 after:bg-seeyue-accent-strong after:rounded-sm flex items-center gap-1.5">
            <ListTree size={14} /> Contents
          </span>
          <button
            type="button"
            className="inline-flex items-center justify-center w-[26px] h-[26px] rounded-md text-seeyue-fg-dim bg-transparent border-0 cursor-pointer transition-all duration-150 hover:text-seeyue-fg-strong hover:bg-seeyue-elevated data-[active=true]:text-seeyue-success data-[active=true]:bg-[rgba(163,190,140,0.15)] disabled:opacity-30 disabled:cursor-not-allowed"
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
    <nav className="h-full flex flex-col bg-seeyue-sidebar">
      <div className="flex items-center justify-between px-3.5 pt-[14px] pb-1">
        <span className="font-cjk text-sm font-medium text-seeyue-fg-strong tracking-[0.04em] relative pb-1 after:content-[''] after:absolute after:left-0 after:bottom-[-1px] after:w-7 after:h-0.5 after:bg-seeyue-accent-strong after:rounded-sm flex items-center gap-1.5">
          <ListTree size={14} /> Contents
        </span>
        <button
          type="button"
          className="inline-flex items-center justify-center w-[26px] h-[26px] rounded-md text-seeyue-fg-dim bg-transparent border-0 cursor-pointer transition-all duration-150 hover:text-seeyue-fg-strong hover:bg-seeyue-elevated data-[active=true]:text-seeyue-success data-[active=true]:bg-[rgba(163,190,140,0.15)] disabled:opacity-30 disabled:cursor-not-allowed"
          onClick={onToggleCollapsed}
          title="收起目录"
        >
          <ChevronsRight size={14} />
        </button>
      </div>
      <ul className="flex-1 overflow-y-auto px-1 pb-4 list-none m-0 border-l border-seeyue-border ml-3.5">
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
                className="block py-[3px] px-1.5 pl-2.5 -ml-px border-l-2 border-transparent text-[12.5px] text-seeyue-fg-muted no-underline whitespace-nowrap overflow-hidden text-ellipsis transition-colors duration-150 hover:text-seeyue-accent data-[active=true]:text-seeyue-fg-strong data-[active=true]:border-l-seeyue-success data-[active=true]:font-medium"
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
