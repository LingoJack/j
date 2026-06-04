import { useEffect, useState } from 'react'
import { ChevronsRight, ListTree } from './Icon'
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

  // 收起态：只显示一个小圆按钮
  if (collapsed) {
    return (
      <button
        type="button"
        onClick={onToggleCollapsed}
        className="absolute right-3 top-3 z-10 flex items-center justify-center w-7 h-7 rounded-md bg-seeyue-bg/80 backdrop-blur-sm text-seeyue-fg-dim border border-seeyue-border-dim cursor-pointer transition-all duration-150 hover:text-seeyue-fg-strong hover:bg-seeyue-elevated hover:border-seeyue-border-strong"
        title="展开目录"
      >
        <ListTree size={14} />
      </button>
    )
  }

  // 无标题
  if (headings.length === 0) {
    return (
      <div className="absolute right-3 top-3 z-10 w-[180px] rounded-lg bg-seeyue-bg/85 backdrop-blur-sm border border-seeyue-border-dim shadow-[0_2px_12px_rgba(26,22,18,0.06)]">
        <div className="flex items-center justify-between px-3 pt-2.5 pb-1">
          <span className="text-[11px] font-semibold text-seeyue-fg-dim uppercase tracking-wider flex items-center gap-1.5">
            <ListTree size={12} /> Contents
          </span>
          <button
            type="button"
            className="inline-flex items-center justify-center w-5 h-5 rounded text-seeyue-fg-dim bg-transparent border-0 cursor-pointer transition-colors duration-150 hover:text-seeyue-fg-strong hover:bg-seeyue-elevated"
            onClick={onToggleCollapsed}
            title="收起目录"
          >
            <ChevronsRight size={12} />
          </button>
        </div>
        <div className="px-3 pb-3 text-[11px] text-seeyue-fg-dim italic">
          暂无标题
        </div>
      </div>
    )
  }

  const minLevel = Math.min(...headings.map((h) => h.level))

  return (
    <nav className="absolute right-3 top-3 z-10 w-[200px] max-h-[calc(100%-24px)] rounded-lg bg-seeyue-bg/85 backdrop-blur-sm border border-seeyue-border-dim shadow-[0_2px_12px_rgba(26,22,18,0.06)] flex flex-col">
      <div className="flex items-center justify-between px-3 pt-2.5 pb-1 shrink-0">
        <span className="text-[11px] font-semibold text-seeyue-fg-dim uppercase tracking-wider flex items-center gap-1.5">
          <ListTree size={12} /> Contents
        </span>
        <button
          type="button"
          className="inline-flex items-center justify-center w-5 h-5 rounded text-seeyue-fg-dim bg-transparent border-0 cursor-pointer transition-colors duration-150 hover:text-seeyue-fg-strong hover:bg-seeyue-elevated"
          onClick={onToggleCollapsed}
          title="收起目录"
        >
          <ChevronsRight size={12} />
        </button>
      </div>
      <ul className="flex-1 overflow-y-auto px-2 pb-2 pt-1 list-none m-0">
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
                className={
                  'block py-[3px] px-1.5 rounded text-[12px] no-underline whitespace-nowrap overflow-hidden text-ellipsis transition-colors duration-150 ' +
                  (isActive
                    ? 'text-seeyue-fg-strong font-medium bg-seeyue-accent-soft'
                    : 'text-seeyue-fg-muted hover:text-seeyue-accent hover:bg-seeyue-elevated/50')
                }
                style={{ paddingLeft: `${indent * 12 + 6}px` }}
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
