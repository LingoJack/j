import { useEffect, useState } from 'react'
import type { Block, ParsedDocument } from './types'
import { extractText } from './MarkdownIR'

export interface HeadingItem {
  id: string
  level: number
  text: string
}

/** 从 ParsedDocument 提取标题列表 */
export function extractHeadings(doc: ParsedDocument): HeadingItem[] {
  const headings: HeadingItem[] = []
  const idCounter = new Map<string, number>()

  function walk(blocks: Block[]) {
    for (const block of blocks) {
      if (block.kind.type === 'heading') {
        const { level, content } = block.kind.value
        const text = extractText(content)
        const slug = text
          .toLowerCase()
          .replace(/[^\w\u4e00-\u9fff]+/g, '-')
          .replace(/^-|-$/g, '')
        const count = idCounter.get(slug) ?? 0
        idCounter.set(slug, count + 1)
        const id = count === 0 ? slug : `${slug}-${count}`
        headings.push({ id, level, text })
      } else if (block.kind.type === 'block_quote') {
        walk(block.kind.value)
      }
    }
  }

  walk(doc.blocks)
  return headings
}

interface Props {
  headings: HeadingItem[]
}

export function TableOfContents({ headings }: Props) {
  const [activeId, setActiveId] = useState<string>('')
  const [open, setOpen] = useState(true)

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
      { rootMargin: '-80px 0px -60% 0px' },
    )

    for (const h of headings) {
      const el = document.getElementById(h.id)
      if (el) observer.observe(el)
    }

    return () => observer.disconnect()
  }, [headings])

  if (headings.length === 0) return null

  const minLevel = Math.min(...headings.map((h) => h.level))

  // 收起状态：右侧小标签
  if (!open) {
    return (
      <button
        onClick={() => setOpen(true)}
        className="sticky top-20 flex items-center gap-1 px-2 py-3 text-stone-400 hover:text-stone-600 transition-colors duration-150 border-l border-stone-200 hover:border-stone-400"
        title="展开目录"
      >
        <svg
          className="w-3.5 h-3.5"
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
          strokeWidth={2}
        >
          <path strokeLinecap="round" strokeLinejoin="round" d="M15 19l-7-7 7-7" />
        </svg>
        <span className="writing-vertical-rl text-[10px] uppercase tracking-widest font-medium">
          目录
        </span>
      </button>
    )
  }

  // 展开状态
  return (
    <div className="sticky top-16 h-fit py-6 pr-4 pl-4">
      <nav className="text-xs">
        <div className="flex items-center justify-between mb-3">
          <span className="text-stone-400 uppercase tracking-wider text-[10px] font-medium">
            目录
          </span>
          <button
            onClick={() => setOpen(false)}
            className="text-stone-300 hover:text-stone-500 transition-colors duration-150"
            title="收起目录"
          >
            <svg
              className="w-3.5 h-3.5"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              strokeWidth={2}
            >
              <path strokeLinecap="round" strokeLinejoin="round" d="M9 5l7 7-7 7" />
            </svg>
          </button>
        </div>
        <ul className="space-y-1.5 border-l border-stone-200">
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
                  className={`
                    block truncate transition-colors duration-150
                    ${isActive ? 'text-stone-900 font-medium' : 'text-stone-400 hover:text-stone-700'}
                  `}
                  style={{ paddingLeft: `${indent * 12 + 10}px` }}
                  title={h.text}
                >
                  <span
                    className={`
                      inline-block border-l-2 -ml-px pl-2 py-0.5
                      ${isActive ? 'border-stone-900' : 'border-transparent'}
                      transition-colors duration-150
                    `}
                  >
                    {h.text}
                  </span>
                </a>
              </li>
            )
          })}
        </ul>
      </nav>
    </div>
  )
}
