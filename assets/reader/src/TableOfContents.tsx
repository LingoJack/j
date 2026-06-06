import { useEffect, useState } from 'react'
import { ListTree, Pin, PinOff } from './Icon'
import type { HeadingItem } from './toc'

interface Props {
  headings: HeadingItem[]
  pinned: boolean
  onTogglePinned: () => void
}

export function TableOfContents({ headings, pinned, onTogglePinned }: Props) {
  const [activeId, setActiveId] = useState<string>('')

  useEffect(() => {
    if (headings.length === 0) return
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            setActiveId(entry.target.id)
          }
        }
      },
      { rootMargin: '-80px 0px -60% 0px' }
    )
    for (const h of headings) {
      const el = document.getElementById(h.id)
      if (el) observer.observe(el)
    }
    return () => observer.disconnect()
  }, [headings])

  const minLevel = headings.length > 0 ? Math.min(...headings.map((h) => h.level)) : 1

  if (pinned) {
    return (
      <TocPanel
        headings={headings}
        activeId={activeId}
        minLevel={minLevel}
        pinned={pinned}
        onTogglePinned={onTogglePinned}
      />
    )
  }

  return (
    <div className="group absolute right-3 top-3 z-10 w-[36px] hover:w-[240px] focus-within:w-[240px] max-h-[calc(100%-24px)] transition-[width] duration-200 ease-out">
      <div className="absolute right-0 top-0 flex items-center justify-center w-8 h-8 rounded-full bg-seeyue-bg/90 backdrop-blur-md text-seeyue-fg-dim border border-seeyue-border-dim shadow-[0_4px_18px_rgba(26,22,18,0.10)] transition-all duration-150 group-hover:opacity-0 group-focus-within:opacity-0">
        <ListTree size={15} />
      </div>
      <TocPanel
        headings={headings}
        activeId={activeId}
        minLevel={minLevel}
        pinned={pinned}
        onTogglePinned={onTogglePinned}
        className="pointer-events-none opacity-0 translate-x-2 scale-[0.98] group-hover:pointer-events-auto group-hover:opacity-100 group-hover:translate-x-0 group-hover:scale-100 group-focus-within:pointer-events-auto group-focus-within:opacity-100 group-focus-within:translate-x-0 group-focus-within:scale-100 transition-all duration-200 ease-out origin-top-right"
      />
    </div>
  )
}

function PinButton({ pinned, onTogglePinned }: { pinned: boolean; onTogglePinned: () => void }) {
  return (
    <button
      type="button"
      className={
        'inline-flex items-center justify-center w-6 h-6 rounded-full bg-transparent border-0 cursor-pointer transition-colors duration-150 ' +
        (pinned
          ? 'text-seeyue-accent bg-seeyue-accent-soft hover:text-seeyue-accent'
          : 'text-seeyue-fg-dim hover:text-seeyue-fg-strong hover:bg-seeyue-elevated')
      }
      onClick={onTogglePinned}
      title={pinned ? '取消固定目录' : '固定目录'}
      aria-label={pinned ? '取消固定目录' : '固定目录'}
      aria-pressed={pinned}
    >
      {pinned ? <PinOff size={13} /> : <Pin size={13} />}
    </button>
  )
}

function TocPanel({
  headings,
  activeId,
  minLevel,
  pinned,
  onTogglePinned,
  className = '',
}: {
  headings: HeadingItem[]
  activeId: string
  minLevel: number
  pinned: boolean
  onTogglePinned: () => void
  className?: string
}) {
  const basePanelClass =
    'z-10 bg-seeyue-bg/92 backdrop-blur-md border border-seeyue-border-dim flex flex-col overflow-hidden '
  const placementClass = pinned
    ? 'relative w-[248px] h-full max-h-none rounded-none border-y-0 border-r-0 border-l-seeyue-border bg-seeyue-sidebar/72 shadow-[inset_1px_0_0_rgba(255,255,255,0.48)] '
    : 'absolute right-0 top-0 w-[220px] max-h-[calc(100vh-128px)] rounded-xl shadow-[0_10px_34px_rgba(26,22,18,0.13)] '
  const panelClass = basePanelClass + placementClass + className

  if (headings.length === 0) {
    return (
      <div className={panelClass}>
        <div className="flex items-center justify-between px-3 pt-2.5 pb-1.5 shrink-0">
          <span className="text-[11px] font-semibold text-seeyue-fg-dim uppercase tracking-wider flex items-center gap-1.5">
            <ListTree size={12} /> Contents
          </span>
          <PinButton pinned={pinned} onTogglePinned={onTogglePinned} />
        </div>
        <div className="px-3 pb-3 text-[11px] text-seeyue-fg-dim italic">暂无标题</div>
      </div>
    )
  }

  return (
    <nav className={panelClass}>
      <div className="flex items-center justify-between px-3 pt-2.5 pb-1.5 shrink-0">
        <span className="text-[11px] font-semibold text-seeyue-fg-dim uppercase tracking-wider flex items-center gap-1.5">
          <ListTree size={12} /> Contents
        </span>
        <PinButton pinned={pinned} onTogglePinned={onTogglePinned} />
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
                  document.getElementById(h.id)?.scrollIntoView({ behavior: 'smooth' })
                }}
                className={
                  'block py-[4px] px-1.5 rounded-md text-[12px] no-underline whitespace-nowrap overflow-hidden text-ellipsis transition-colors duration-150 ' +
                  (isActive
                    ? 'text-seeyue-fg-strong font-medium bg-seeyue-accent-soft'
                    : 'text-seeyue-fg-muted hover:text-seeyue-accent hover:bg-seeyue-elevated/60')
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
