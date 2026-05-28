import { useCallback, useEffect, useRef } from 'react'
import { MarkdownIR } from '../MarkdownIR'
import type { ParsedDocument, Tab } from '../types'
import { offsetToSelection, selectionToOffset } from './selection'
import { backspace, forwardDelete, insertAt, replaceRange } from './source'

interface Props {
  tab: Tab
  onChange: (source: string) => void
  onParsed: (doc: ParsedDocument) => void
  onSave: () => void | Promise<void>
}

/** debounce 时间：source 变化后多久 POST /api/parse */
const PARSE_DEBOUNCE_MS = 150

/**
 * 真所见即所得编辑器（contenteditable）。
 *
 * v0 实现：
 * - 整篇渲染为 MarkdownIR；外层 contenteditable
 * - 输入事件走 beforeinput 拦截 → 自己改 source → setState
 * - source 变化 150ms 后 parse → 重渲染
 * - 重渲染前 capture caret offset；重渲染后 restore
 *
 * 后续里程碑：
 * - syntax triggers（M1/M2）
 * - 表格、代码块的 contenteditable cell（M3/M4）
 * - undo / redo / 快捷键（M5/M6）
 */
export function WysiwygEditor({ tab, onChange, onParsed, onSave }: Props) {
  const articleRef = useRef<HTMLDivElement | null>(null)
  /** 重渲染期间要恢复的光标 offset */
  const pendingCaretOffsetRef = useRef<number | null>(null)
  /** IME 正在输入中文 */
  const composingRef = useRef(false)

  // —— Source → IR：debounced parse ——
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
        // 静默
      }
    }, PARSE_DEBOUNCE_MS)
    return () => window.clearTimeout(timer)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tab.source])

  // —— React 重渲染后恢复光标 ——
  useEffect(() => {
    const target = pendingCaretOffsetRef.current
    if (target == null) return
    const article = articleRef.current
    if (!article) return
    // 用 microtask 等待 DOM 真正更新完
    queueMicrotask(() => {
      offsetToSelection(article, tab.source, target)
      pendingCaretOffsetRef.current = null
    })
  }, [tab.doc, tab.source])

  /** 当前 selection 范围（start/end source offset） */
  const currentRange = useCallback((): { start: number; end: number } | null => {
    if (!articleRef.current) return null
    const sel = window.getSelection()
    if (!sel || sel.rangeCount === 0) return null
    const range = sel.getRangeAt(0)
    const startOffset = selectionToOffset(articleRef.current, tab.source)
    if (startOffset == null) return null
    if (range.collapsed) return { start: startOffset, end: startOffset }
    // 取 endContainer + endOffset 单独算一次
    const sel2 = window.getSelection()
    if (!sel2) return null
    // 临时 collapse 到 end，重新算 offset
    const savedRange = range.cloneRange()
    sel2.removeAllRanges()
    const endRange = document.createRange()
    endRange.setStart(range.endContainer, range.endOffset)
    endRange.collapse(true)
    sel2.addRange(endRange)
    const endOffset = selectionToOffset(articleRef.current, tab.source)
    sel2.removeAllRanges()
    sel2.addRange(savedRange)
    if (endOffset == null) return { start: startOffset, end: startOffset }
    return {
      start: Math.min(startOffset, endOffset),
      end: Math.max(startOffset, endOffset),
    }
  }, [tab.source])

  // —— beforeinput 路由 ——
  const handleBeforeInput = useCallback(
    (e: React.FormEvent<HTMLDivElement>) => {
      // React 把 InputEvent 当 FormEvent 包了一层；用 nativeEvent 拿原始
      const ne = (e.nativeEvent as InputEvent) ?? null
      if (!ne) return
      // 中文输入期间，让浏览器自然处理（compositionend 后再同步）
      if (composingRef.current) return

      const range = currentRange()
      if (!range) return

      const inputType = ne.inputType
      const data = ne.data ?? ''

      let next: { source: string; nextOffset: number } | null = null

      switch (inputType) {
        case 'insertText':
        case 'insertReplacementText':
        case 'insertFromPaste': {
          // 粘贴本轮仅取 plain text；replaceRange 会把选区替换掉
          if (data) {
            next = replaceRange(tab.source, range.start, range.end, data)
          }
          break
        }
        case 'insertParagraph': {
          // 回车：插入 \n
          next = replaceRange(tab.source, range.start, range.end, '\n')
          break
        }
        case 'insertLineBreak': {
          // Shift+Enter：硬换行
          next = replaceRange(tab.source, range.start, range.end, '\n')
          break
        }
        case 'deleteContentBackward': {
          if (range.start === range.end) {
            next = backspace(tab.source, range.start, 1)
          } else {
            next = replaceRange(tab.source, range.start, range.end, '')
          }
          break
        }
        case 'deleteContentForward': {
          if (range.start === range.end) {
            next = forwardDelete(tab.source, range.start, 1)
          } else {
            next = replaceRange(tab.source, range.start, range.end, '')
          }
          break
        }
        case 'deleteWordBackward': {
          // 简化：删一个空白边界；找到 start 前最近的非字母数字位置
          let i = range.start
          while (i > 0 && /\s/.test(tab.source[i - 1])) i--
          while (i > 0 && /\S/.test(tab.source[i - 1])) i--
          next =
            range.start === range.end
              ? { source: tab.source.slice(0, i) + tab.source.slice(range.start), nextOffset: i }
              : replaceRange(tab.source, range.start, range.end, '')
          break
        }
        case 'deleteWordForward': {
          let i = range.end
          while (i < tab.source.length && /\s/.test(tab.source[i])) i++
          while (i < tab.source.length && /\S/.test(tab.source[i])) i++
          next =
            range.start === range.end
              ? { source: tab.source.slice(0, range.start) + tab.source.slice(i), nextOffset: range.start }
              : replaceRange(tab.source, range.start, range.end, '')
          break
        }
        default: {
          // 兜底：让浏览器自己处理；用 onInput 走「DOM 反读」路径（暂未实现）
          // 大多数 inputType 我们处理了，这里 return 让浏览器走默认行为
          return
        }
      }

      if (next) {
        e.preventDefault()
        pendingCaretOffsetRef.current = next.nextOffset
        onChange(next.source)
      }
    },
    [tab.source, currentRange, onChange],
  )

  // —— composition 事件 ——
  const handleCompositionStart = useCallback(() => {
    composingRef.current = true
  }, [])

  const handleCompositionEnd = useCallback(
    (e: React.CompositionEvent<HTMLDivElement>) => {
      composingRef.current = false
      // 把 composition 期间浏览器自己改的 DOM 反同步回 source
      const article = articleRef.current
      if (!article) return
      const sel = window.getSelection()
      if (!sel || sel.rangeCount === 0) return
      const offset = selectionToOffset(article, tab.source)
      if (offset == null) return
      const data = e.data ?? ''
      // 简化：在 offset 前插入 composition 输出（浏览器已经写进 DOM 了，
      // 我们要保证 source 状态同步）
      // 最稳的策略：扔掉 DOM 当前文本，根据 source 重渲染（清掉 composition 残留）
      // 但代价是用户看到的字符会闪一下；先这么做，后期再优化。
      const updated = insertAt(tab.source, offset - data.length, data)
      pendingCaretOffsetRef.current = updated.nextOffset
      onChange(updated.source)
    },
    [tab.source, onChange],
  )

  // —— onPaste：仅取 plain text ——
  const handlePaste = useCallback(
    (e: React.ClipboardEvent<HTMLDivElement>) => {
      e.preventDefault()
      const text = e.clipboardData.getData('text/plain')
      if (!text) return
      const range = currentRange()
      if (!range) return
      const next = replaceRange(tab.source, range.start, range.end, text)
      pendingCaretOffsetRef.current = next.nextOffset
      onChange(next.source)
    },
    [tab.source, currentRange, onChange],
  )

  // —— Cmd+S 保存 ——
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLDivElement>) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 's') {
        e.preventDefault()
        void onSave()
      }
    },
    [onSave],
  )

  // 没 IR 时显示加载占位
  if (!tab.doc) {
    return (
      <div className="h-full overflow-y-auto bg-seeyue-bg">
        <div className="seeyue-prose max-w-3xl mx-auto px-8 py-8 text-seeyue-fg-dim text-sm">
          解析中…
        </div>
      </div>
    )
  }

  return (
    <div className="h-full overflow-y-auto bg-seeyue-bg">
      <div
        ref={articleRef}
        className="seeyue-prose seeyue-wysiwyg max-w-3xl mx-auto px-8 py-8 outline-none"
        contentEditable
        suppressContentEditableWarning
        spellCheck={false}
        onBeforeInput={handleBeforeInput}
        onCompositionStart={handleCompositionStart}
        onCompositionEnd={handleCompositionEnd}
        onPaste={handlePaste}
        onKeyDown={handleKeyDown}
      >
        <MarkdownIR doc={tab.doc} />
      </div>
    </div>
  )
}
