import { useCallback, useEffect, useRef } from 'react'
import { BlockSourceEditor } from './BlockSourceEditor'
import {
  MarkdownIR,
  renderSingleBlock,
  resetHeadingIdCounter,
} from './MarkdownIR'
import type { Block, ParsedDocument, Tab } from './types'

interface Props {
  tab: Tab
  onChange: (source: string) => void
  onParsed: (doc: ParsedDocument) => void
  onSave: () => void | Promise<void>
  /** 父级控制：当前编辑的 block index（null = 全渲染） */
  editingBlockIdx: number | null
  setEditingBlockIdx: (idx: number | null) => void
}

/** debounce 时间：左侧编辑停顿后多久重 parse */
const PARSE_DEBOUNCE_MS = 150

/**
 * Typora 风「所见即所得」编辑器：
 * - 当前光标所在的 block → 渲染为 textarea（源码态）
 * - 其它 block → 用 MarkdownIR 渲染（渲染态）
 * - 点击渲染态的 block → 进入源码态
 * - blur / Esc / 点击其它 block → 退出源码态
 */
export function MarkdownLiveEditor({
  tab,
  onChange,
  onParsed,
  onSave,
  editingBlockIdx,
  setEditingBlockIdx,
}: Props) {
  const containerRef = useRef<HTMLDivElement | null>(null)

  // —— 实时预览：source → /api/parse → doc ——
  const reqIdRef = useRef(0)
  useEffect(() => {
    const myId = ++reqIdRef.current
    const timer = window.setTimeout(async () => {
      try {
        const res = await fetch('./api/parse', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ source: tab.source }),
        })
        if (!res.ok) return
        const doc = (await res.json()) as ParsedDocument
        if (myId !== reqIdRef.current) return
        onParsed(doc)
      } catch {
        // 静默失败：保留上次预览
      }
    }, PARSE_DEBOUNCE_MS)
    return () => window.clearTimeout(timer)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tab.source])

  // —— 文档级 click：点空白处退出当前编辑 ——
  // （block 自身已 stopPropagation，所以这里捕获的就是「不在任何 block 上」）

  /** 把整段 source 中 [start, end] 行（含）替换为 newSlice */
  const replaceLines = useCallback(
    (source: string, start: number, end: number, newSlice: string): string => {
      const lines = source.split('\n')
      const before = lines.slice(0, start).join('\n')
      const after = lines.slice(end + 1).join('\n')
      const middle = newSlice
      // 保留行边界：before + '\n' + middle + '\n' + after，三段都可能为空
      const parts: string[] = []
      if (start > 0) parts.push(before)
      parts.push(middle)
      if (end + 1 < lines.length) parts.push(after)
      return parts.join('\n')
    },
    [],
  )

  // 渲染时复位 heading id 计数（与 MarkdownIR 全文渲染一致）
  resetHeadingIdCounter()

  // 没有 IR 时回退到全文 textarea（首次进 tab 还没拿到 doc 的极短时间）
  if (!tab.doc) {
    return (
      <div className="h-full overflow-y-auto bg-seeyue-bg">
        <div className="seeyue-prose max-w-3xl mx-auto px-8 py-8 text-seeyue-fg-dim text-sm">
          解析中…
        </div>
      </div>
    )
  }

  const blocks: Block[] = tab.doc.blocks
  const lines = tab.source.split('\n')

  return (
    <div
      ref={containerRef}
      className="h-full overflow-y-auto bg-seeyue-bg"
    >
      <div className="seeyue-prose max-w-3xl mx-auto px-8 py-8">
        {blocks.map((block, i) => {
          const start = block.source.start_line
          const end = block.source.end_line
          if (i === editingBlockIdx) {
            const slice = lines.slice(start, end + 1).join('\n')
            return (
              <BlockSourceEditor
                key={`b-${i}-edit`}
                sourceSlice={slice}
                onChangeSlice={(text) => {
                  const newSource = replaceLines(tab.source, start, end, text)
                  onChange(newSource)
                }}
                onLeave={() => setEditingBlockIdx(null)}
                onSave={onSave}
              />
            )
          }
          return (
            <div
              key={`b-${i}`}
              className="cursor-text rounded -mx-2 px-2 hover:bg-white/[0.02] transition-colors"
              onClick={(e) => {
                e.stopPropagation()
                setEditingBlockIdx(i)
              }}
            >
              {renderSingleBlock(block, `b-${i}`)}
            </div>
          )
        })}
        {/* 文档末尾插入空白点击区，避免点不到最后一行下方 */}
        <div
          className="h-32"
          onClick={() => setEditingBlockIdx(null)}
        />
      </div>
    </div>
  )
}

// 导出 MarkdownIR 以便其它地方仍能用全渲染（避免 import 循环）
export { MarkdownIR }
