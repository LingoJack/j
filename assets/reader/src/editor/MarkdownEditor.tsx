/*
 * Markdown 编辑器 —— 类 Typora 混合编辑模式。
 *
 * 核心思路：
 * - 默认显示后端解析后的 Markdown 渲染结果；
 * - 点击某个 block 后，仅该 block 切换为 Markdown 源码 textarea；
 * - 其它 block 继续保持渲染态；
 * - textarea 是真实输入控件，避免 contenteditable 编辑渲染 DOM 导致内容错乱和光标跳转；
 * - source 变化后仍通过 Rust 后端 /api/parse 解析，前端不自行解析 Markdown。
 */
import { useCallback, useEffect, useRef, useState } from 'react'
import type { Alignment, Block, Inline, ListData, ParsedDocument } from '../types'
import { extractText } from '../MarkdownIR'
import { slugify } from '../slug'
import { renderHighlightedCode } from './code-highlight'
import { resetInlineCache, renderInlines } from './inline-renderer'

interface Props {
  path: string
  baseDir: string | null
  initialSource: string
  /** 后端已解析好的 ParsedDocument（来自 /api/file 的 payload） */
  initialDoc: ParsedDocument
  onChange: (path: string, source: string) => void
  onParsed: (path: string, doc: ParsedDocument) => void
  onSave: () => void | Promise<void>
}

type LineRange = {
  startLine: number
  endLine: number
}

type FocusRequest = {
  line: number
  column: number
}

function useLatest<T>(value: T) {
  const ref = useRef(value)
  useEffect(() => {
    ref.current = value
  }, [value])
  return ref
}

/** 调用后端 /api/parse 解析 markdown source */
async function fetchParse(source: string): Promise<ParsedDocument> {
  const resp = await fetch('./api/parse', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ source }),
  })
  if (!resp.ok) throw new Error(`/api/parse failed: ${resp.status}`)
  return resp.json()
}

function sameRange(a: LineRange | null, b: LineRange | null): boolean {
  if (!a || !b) return a === b
  return a.startLine === b.startLine && a.endLine === b.endLine
}

function normalizeRange(block: Block, lineCount: number): LineRange {
  const startLine = Math.max(0, Math.min(block.source.start_line, Math.max(0, lineCount - 1)))
  const endLine = Math.max(startLine, Math.min(block.source.end_line, Math.max(0, lineCount - 1)))
  return { startLine, endLine }
}

function normalizeBlockRanges(blocks: Block[], lines: string[]): LineRange[] {
  return blocks.map((block, index) => {
    const range = normalizeRange(block, lines.length)
    const next = blocks[index + 1]
    if (next) {
      const nextRange = normalizeRange(next, lines.length)
      if (nextRange.startLine > range.startLine && nextRange.startLine <= range.endLine) {
        range.endLine = nextRange.startLine - 1
      }
    }
    return trimEditableRange(block, range, lines)
  })
}

function trimEditableRange(block: Block, range: LineRange, lines: string[]): LineRange {
  if (block.kind.type === 'code_block') return range

  while (range.endLine > range.startLine && (lines[range.endLine]?.trim() ?? '') === '') {
    range.endLine -= 1
  }
  return range
}

function splitListItemRanges(range: LineRange, lines: string[]): LineRange[] {
  const ranges: LineRange[] = []
  for (let line = range.startLine; line <= range.endLine; line++) {
    if (isListMarkerLine(lines[line] ?? '')) ranges.push({ startLine: line, endLine: line })
  }

  return ranges.length > 0 ? ranges : [range]
}

function findEditableRangeForLine(ranges: LineRange[], targetLine: number): LineRange | null {
  if (targetLine < 0) return null
  return ranges.find((range) => targetLine >= range.startLine && targetLine <= range.endLine) ?? null
}

function getEditableRanges(blocks: Block[], lines: string[]): LineRange[] {
  const blockRanges = normalizeBlockRanges(blocks, lines)
  const ranges: LineRange[] = []
  let nextLine = 0

  blocks.forEach((block, index) => {
    const range = blockRanges[index]
    for (let line = nextLine; line < range.startLine; line++) ranges.push({ startLine: line, endLine: line })
    if (block.kind.type === 'list') ranges.push(...splitListItemRanges(range, lines))
    else ranges.push(range)
    nextLine = range.endLine + 1
  })

  for (let line = nextLine; line < lines.length; line++) ranges.push({ startLine: line, endLine: line })
  return ranges
}

function isListMarkerLine(line: string): boolean {
  return /^\s*(?:[-+*]\s+|\d+[.)]\s+)/.test(line)
}

function getRangeText(lines: string[], range: LineRange): string {
  return lines.slice(range.startLine, range.endLine + 1).join('\n')
}

function replaceRangeText(source: string, range: LineRange, text: string): string {
  const lines = source.split('\n')
  const replacement = text.split('\n')
  lines.splice(range.startLine, range.endLine - range.startLine + 1, ...replacement)
  return lines.join('\n')
}

function rangeLineCount(text: string): number {
  return text.split('\n').length
}

export function MarkdownEditor({
  path,
  baseDir,
  initialSource,
  initialDoc,
  onChange,
  onParsed,
  onSave,
}: Props) {
  const hostRef = useRef<HTMLDivElement | null>(null)
  const scrollRef = useRef<HTMLDivElement | null>(null)
  const sourceRef = useRef(initialSource)
  const docRef = useRef(initialDoc)
  const debounceTimer = useRef<ReturnType<typeof setTimeout> | null>(null)
  const lastParsedSource = useRef('')
  const parseSeq = useRef(0)
  const activeRangeRef = useRef<LineRange | null>(null)
  const focusRequestRef = useRef<FocusRequest | null>(null)
  const documentCursorRef = useRef<FocusRequest>({ line: 0, column: 0 })
  const switchingRangeRef = useRef(false)
  const [activeRange, setActiveRange] = useState<LineRange | null>(null)

  const onChangeRef = useLatest(onChange)
  const onParsedRef = useLatest(onParsed)
  const onSaveRef = useLatest(onSave)
  const pathRef = useLatest(path)
  const baseDirRef = useLatest(baseDir)

  const setActiveRangeSafely = useCallback((next: LineRange | null, focus?: FocusRequest) => {
    if (focus) {
      documentCursorRef.current = focus
      switchingRangeRef.current = true
    }
    activeRangeRef.current = next
    focusRequestRef.current = focus ?? null
    setActiveRange((prev) => (sameRange(prev, next) ? prev : next))
  }, [])

  const moveDocumentCursor = useCallback(
    (delta: -1 | 1, from?: FocusRequest): boolean => {
      const lines = sourceRef.current.split('\n')
      const current = from ?? documentCursorRef.current
      const targetLine = Math.max(0, Math.min(lines.length - 1, current.line + delta))
      if (targetLine === current.line) return false

      const targetColumn = Math.min(current.column, lines[targetLine]?.length ?? 0)
      const focus = { line: targetLine, column: targetColumn }
      const ranges = getEditableRanges(docRef.current.blocks, lines)
      const targetRange = findEditableRangeForLine(ranges, targetLine)
      if (!targetRange) return false
      setActiveRangeSafely(targetRange, focus)
      return true
    },
    [setActiveRangeSafely]
  )

  const renderDocument = useCallback(() => {
    const host = hostRef.current
    if (!host) return

    const savedScrollTop = scrollRef.current?.scrollTop ?? 0
    const lines = sourceRef.current.split('\n')
    const active = activeRangeRef.current
    const blocks = docRef.current.blocks
    const ranges = normalizeBlockRanges(blocks, lines)

    resetInlineCache()
    host.replaceChildren()

    let nextLine = 0
    blocks.forEach((block, index) => {
      const range = ranges[index]
      if (block.kind.type === 'list') {
        const listData = block.kind.value
        const listRanges = splitListItemRanges(range, lines)
        appendInterBlockLines({ host, fromLine: nextLine, toLine: listRanges[0]?.startLine ?? range.startLine, lines, active, setActiveRangeSafely, moveDocumentCursor, sourceRef, activeRangeRef, switchingRangeRef, onChangeRef, pathRef, onSaveRef, scheduleParse, parseAndRender })
        listRanges.forEach((listRange, itemIndex) => {
          if (active && sameRange(active, listRange)) {
            host.appendChild(
              createSourceBlockEditor({
                range: listRange,
                block,
                rawValue: getRangeText(lines, listRange),
                onChange: (value) => {
                  const currentRange = activeRangeRef.current ?? listRange
                  const nextSource = replaceRangeText(sourceRef.current, currentRange, value)
                  const nextRange = {
                    startLine: currentRange.startLine,
                    endLine: currentRange.startLine + rangeLineCount(value) - 1,
                  }
                  sourceRef.current = nextSource
                  activeRangeRef.current = nextRange
                  onChangeRef.current(pathRef.current, nextSource)
                  scheduleParse(nextSource)
                },
                onBlur: () => {
                  if (switchingRangeRef.current) return
                  if (!sameRange(activeRangeRef.current, listRange)) return
                  setActiveRangeSafely(null)
                  parseAndRender(sourceRef.current)
                },
                onMoveLine: moveDocumentCursor,
                onSave: () => void onSaveRef.current(),
              })
            )
          } else {
            const el = createListItemElement(listData, itemIndex, baseDirRef.current)
            el.dataset.startLine = String(listRange.startLine)
            el.dataset.endLine = String(listRange.endLine)
            el.addEventListener('mousedown', (event) => {
              event.preventDefault()
              event.stopPropagation()
              setActiveRangeSafely(listRange, { line: listRange.startLine, column: 0 })
            })
            host.appendChild(el)
          }
          nextLine = listRange.endLine + 1
        })
        return
      }

      appendInterBlockLines({ host, fromLine: nextLine, toLine: range.startLine, lines, active, setActiveRangeSafely, moveDocumentCursor, sourceRef, activeRangeRef, switchingRangeRef, onChangeRef, pathRef, onSaveRef, scheduleParse, parseAndRender })

      if (active && sameRange(active, range)) {
        host.appendChild(
          createSourceBlockEditor({
            range,
            block,
            rawValue: getRangeText(lines, range),
            onChange: (value) => {
              const currentRange = activeRangeRef.current ?? range
              const nextSource = replaceRangeText(sourceRef.current, currentRange, value)
              const nextRange = {
                startLine: currentRange.startLine,
                endLine: currentRange.startLine + rangeLineCount(value) - 1,
              }
              sourceRef.current = nextSource
              activeRangeRef.current = nextRange
              onChangeRef.current(pathRef.current, nextSource)
              scheduleParse(nextSource)
            },
            onBlur: () => {
              if (switchingRangeRef.current) return
              if (!sameRange(activeRangeRef.current, range)) return
              setActiveRangeSafely(null)
              parseAndRender(sourceRef.current)
            },
            onMoveLine: moveDocumentCursor,
            onSave: () => void onSaveRef.current(),
          })
        )
      } else {
        const el = createBlockElement(block, baseDirRef.current)
        el.dataset.startLine = String(range.startLine)
        el.dataset.endLine = String(range.endLine)
        el.addEventListener('mousedown', (event) => {
          event.preventDefault()
          event.stopPropagation()
          setActiveRangeSafely(range, { line: range.startLine, column: 0 })
        })
        host.appendChild(el)
      }

      nextLine = range.endLine + 1
    })
    appendInterBlockLines({ host, fromLine: nextLine, toLine: lines.length, lines, active, setActiveRangeSafely, moveDocumentCursor, sourceRef, activeRangeRef, switchingRangeRef, onChangeRef, pathRef, onSaveRef, scheduleParse, parseAndRender })

    if (scrollRef.current) scrollRef.current.scrollTop = savedScrollTop

    const editor =
      host.querySelector<HTMLElement>('.md-preferred-focus') ??
      host.querySelector<HTMLElement>('.md-block-source-input, .md-code-source-input, .md-table-cell-input, .md-code-lang-input')
    if (editor && document.activeElement !== editor) {
      editor.focus()
    }
    const focusRequest = focusRequestRef.current
    if (focusRequest && editor instanceof HTMLTextAreaElement) {
      focusRequestRef.current = null
      setTextareaCursorForDocumentLine(editor, focusRequest.line, focusRequest.column)
      requestAnimationFrame(() => {
        switchingRangeRef.current = false
      })
    } else {
      switchingRangeRef.current = false
    }
  }, [baseDirRef, onChangeRef, onSaveRef, pathRef, setActiveRangeSafely])

  const parseAndRender = useCallback(
    (nextSource: string) => {
      if (nextSource === lastParsedSource.current) {
        if (!activeRangeRef.current) renderDocument()
        return
      }

      const seq = ++parseSeq.current
      lastParsedSource.current = nextSource
      fetchParse(nextSource)
        .then((doc) => {
          if (seq !== parseSeq.current) return
          docRef.current = doc
          onParsedRef.current(pathRef.current, doc)
          if (!activeRangeRef.current) renderDocument()
        })
        .catch(() => {
          if (!activeRangeRef.current) renderDocument()
        })
    },
    [onParsedRef, pathRef, renderDocument]
  )

  useEffect(() => {
    renderDocument()
  }, [activeRange, renderDocument])

  useEffect(() => {
    sourceRef.current = initialSource
    docRef.current = initialDoc
    lastParsedSource.current = initialSource
    parseSeq.current += 1
    setActiveRangeSafely(null)
    renderDocument()
    onParsedRef.current(path, initialDoc)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [path])

  useEffect(() => {
    return () => {
      if (debounceTimer.current) clearTimeout(debounceTimer.current)
    }
  }, [])

  return (
    <div ref={scrollRef} className="md-editor-shell">
      <div ref={hostRef} className="md-editor min-h-full outline-none" />
    </div>
  )

  function scheduleParse(nextSource: string) {
    if (debounceTimer.current) clearTimeout(debounceTimer.current)
    debounceTimer.current = setTimeout(() => {
      parseAndRender(nextSource)
    }, 250)
  }
}

type InterBlockOptions = {
  host: HTMLElement
  fromLine: number
  toLine: number
  lines: string[]
  active: LineRange | null
  sourceRef: React.MutableRefObject<string>
  activeRangeRef: React.MutableRefObject<LineRange | null>
  switchingRangeRef: React.MutableRefObject<boolean>
  onChangeRef: React.MutableRefObject<(path: string, source: string) => void>
  pathRef: React.MutableRefObject<string>
  onSaveRef: React.MutableRefObject<() => void | Promise<void>>
  setActiveRangeSafely: (next: LineRange | null, focus?: FocusRequest) => void
  moveDocumentCursor: (delta: -1 | 1, from?: FocusRequest) => boolean
  scheduleParse: (source: string) => void
  parseAndRender: (source: string) => void
}

function appendInterBlockLines(options: InterBlockOptions) {
  const { host, fromLine, toLine, lines, active } = options
  for (let line = fromLine; line < toLine; line++) {
    const range = { startLine: line, endLine: line }
    if (active && sameRange(active, range)) {
      host.appendChild(
        createSourceBlockEditor({
          range,
          block: createPlainTextBlock(range),
          rawValue: lines[line] ?? '',
          onChange: (value) => {
            const currentRange = options.activeRangeRef.current ?? range
            const nextSource = replaceRangeText(options.sourceRef.current, currentRange, value)
            const nextRange = {
              startLine: currentRange.startLine,
              endLine: currentRange.startLine + rangeLineCount(value) - 1,
            }
            options.sourceRef.current = nextSource
            options.activeRangeRef.current = nextRange
            options.onChangeRef.current(options.pathRef.current, nextSource)
            options.scheduleParse(nextSource)
          },
          onBlur: () => {
            if (options.switchingRangeRef.current) return
            if (!sameRange(options.activeRangeRef.current, range)) return
            options.setActiveRangeSafely(null)
            options.parseAndRender(options.sourceRef.current)
          },
          onMoveLine: options.moveDocumentCursor,
          onSave: () => void options.onSaveRef.current(),
        })
      )
    } else {
      const blank = document.createElement('div')
      blank.className = 'md-block md-blank-line'
      blank.dataset.blockType = 'blank'
      blank.dataset.startLine = String(line)
      blank.dataset.endLine = String(line)
      blank.textContent = '\u00a0'
      blank.addEventListener('mousedown', (event) => {
        event.preventDefault()
        event.stopPropagation()
        options.setActiveRangeSafely(range, { line, column: 0 })
      })
      host.appendChild(blank)
    }
  }
}

function createPlainTextBlock(range: LineRange): Block {
  return {
    source: { start_line: range.startLine, end_line: range.endLine },
    kind: { type: 'paragraph', value: [] },
  } as Block
}

type SourceBlockEditorOptions = {
  range: LineRange
  block: Block
  rawValue: string
  onChange: (value: string) => void
  onBlur: () => void
  onMoveLine: (delta: -1 | 1, from?: FocusRequest) => boolean
  onSave: () => void
}

function createSourceBlockEditor(options: SourceBlockEditorOptions): HTMLElement {
  const kind = options.block.kind
  if (kind.type === 'code_block') return createCodeBlockEditor(options, kind.value.lang, kind.value.code)
  if (kind.type === 'table') return createTableBlockEditor(options, kind.value.alignments)
  return createRawBlockEditor(options)
}

function createRawBlockEditor(options: SourceBlockEditorOptions): HTMLElement {
  const wrap = document.createElement('div')
  wrap.className = 'md-block md-block-source'
  wrap.dataset.blockType = 'source'
  wrap.dataset.startLine = String(options.range.startLine)
  wrap.dataset.endLine = String(options.range.endLine)

  const textarea = createAutoGrowTextarea('md-block-source-input', options.rawValue)
  textarea.addEventListener('input', () => {
    autoGrowTextarea(textarea)
    options.onChange(textarea.value)
  })
  bindCommonEditorKeys(textarea, options)

  wrap.appendChild(textarea)
  return wrap
}

function createCodeBlockEditor(options: SourceBlockEditorOptions, lang: string, code: string): HTMLElement {
  const wrap = document.createElement('div')
  wrap.className = 'md-block md-code-source md-code-wrap'
  wrap.dataset.blockType = 'source_code'
  wrap.dataset.startLine = String(options.range.startLine)
  wrap.dataset.endLine = String(options.range.endLine)

  const langInput = document.createElement('input')
  langInput.className = 'md-code-lang-input md-code-lang'
  langInput.value = lang
  langInput.placeholder = 'code'
  langInput.spellcheck = false

  const textarea = createAutoGrowTextarea('md-code-source-input md-code-content md-preferred-focus', trimCodeBlockDisplayNewline(code))
  const closeEditor = () => {
    langInput.blur()
    textarea.blur()
    options.onBlur()
  }

  const emit = () => {
    options.onChange(buildCodeBlockSource(langInput.value, textarea.value))
  }
  langInput.addEventListener('input', emit)
  langInput.addEventListener('keydown', (event) => {
    if ((event.metaKey || event.ctrlKey) && event.key === 's') {
      event.preventDefault()
      options.onSave()
    }
    if (event.key === 'Escape') {
      event.preventDefault()
      closeEditor()
    }
  })
  textarea.addEventListener('input', () => {
    autoGrowTextarea(textarea)
    emit()
  })
  bindCommonEditorKeys(textarea, options, false)
  langInput.addEventListener('blur', () => {
    setTimeout(() => {
      if (!wrap.contains(document.activeElement)) options.onBlur()
    }, 0)
  })
  textarea.addEventListener('blur', () => {
    setTimeout(() => {
      if (!wrap.contains(document.activeElement)) options.onBlur()
    }, 0)
  })

  const pre = document.createElement('pre')
  pre.className = 'md-code-pre'
  pre.appendChild(textarea)

  wrap.appendChild(langInput)
  wrap.appendChild(pre)
  return wrap
}

function createTableBlockEditor(options: SourceBlockEditorOptions, alignments: Alignment[]): HTMLElement {
  const wrap = document.createElement('div')
  wrap.className = 'md-block md-table-source md-table-wrap'
  wrap.dataset.blockType = 'source_table'
  wrap.dataset.startLine = String(options.range.startLine)
  wrap.dataset.endLine = String(options.range.endLine)

  const rows = parseTableSource(options.rawValue)
  const table = document.createElement('table')
  table.className = 'md-table md-table-edit-grid'

  const emit = () => {
    options.onChange(buildTableSource(rows, alignments))
  }

  rows.forEach((row, rowIndex) => {
    const tr = document.createElement('tr')
    row.forEach((cellValue, columnIndex) => {
      const cell = document.createElement(rowIndex === 0 ? 'th' : 'td')
      applyCellAlignment(cell, alignments[columnIndex])

      const textarea = createAutoGrowTextarea('md-table-cell-input md-table-cell-textarea', cellValue)
      textarea.addEventListener('input', () => {
        autoGrowTextarea(textarea)
        rows[rowIndex][columnIndex] = textarea.value
        emit()
      })
      bindCommonEditorKeys(textarea, options, false)
      textarea.addEventListener('blur', () => {
        setTimeout(() => {
          if (!wrap.contains(document.activeElement)) options.onBlur()
        }, 0)
      })
      if (rowIndex === 0 && columnIndex === 0) textarea.classList.add('md-preferred-focus')

      cell.appendChild(textarea)
      tr.appendChild(cell)
    })
    table.appendChild(tr)
  })

  wrap.appendChild(table)
  return wrap
}

function parseTableSource(source: string): string[][] {
  const lines = source.split('\n').filter((line) => line.trim().length > 0)
  const rows = lines
    .filter((line, index) => index !== 1 || !isMarkdownTableDelimiter(line))
    .map(splitMarkdownTableRow)
  return normalizeTableRows(rows)
}

function splitMarkdownTableRow(line: string): string[] {
  const trimmed = line.trim()
  const withoutOuterPipes = trimmed.replace(/^\|/, '').replace(/\|$/, '')
  return withoutOuterPipes.split('|').map((cell) => cell.trim())
}

function normalizeTableRows(rows: string[][]): string[][] {
  const columnCount = Math.max(1, ...rows.map((row) => row.length))
  return rows.length > 0
    ? rows.map((row) => Array.from({ length: columnCount }, (_, index) => row[index] ?? ''))
    : [Array.from({ length: columnCount }, () => '')]
}

function isMarkdownTableDelimiter(line: string): boolean {
  return /^\s*\|?\s*:?-{3,}:?\s*(?:\|\s*:?-{3,}:?\s*)*\|?\s*$/.test(line)
}

function buildTableSource(rows: string[][], alignments: Alignment[]): string {
  const columnCount = Math.max(1, ...rows.map((row) => row.length), alignments.length)
  const normalizedRows = rows.map((row) => Array.from({ length: columnCount }, (_, index) => row[index] ?? ''))
  const header = normalizedRows[0] ?? Array.from({ length: columnCount }, () => '')
  const body = normalizedRows.slice(1)
  const delimiter = Array.from({ length: columnCount }, (_, index) => tableDelimiterForAlignment(alignments[index]))
  return [header, delimiter, ...body].map(formatMarkdownTableRow).join('\n')
}

function tableDelimiterForAlignment(alignment: Alignment | undefined): string {
  if (alignment === 'center') return ':---:'
  if (alignment === 'right') return '---:'
  return '---'
}

function formatMarkdownTableRow(cells: string[]): string {
  return `| ${cells.map(escapeMarkdownTableCell).join(' | ')} |`
}

function escapeMarkdownTableCell(cell: string): string {
  return cell.replace(/\n/g, '<br>').replace(/\|/g, '\\|')
}

function getTextareaBlockElement(textarea: HTMLTextAreaElement): HTMLElement | null {
  return textarea.closest<HTMLElement>('.md-block')
}

function createAutoGrowTextarea(className: string, value: string): HTMLTextAreaElement {
  const textarea = document.createElement('textarea')
  textarea.className = className
  textarea.spellcheck = false
  textarea.value = value
  textarea.rows = Math.max(1, rangeLineCount(value))
  requestAnimationFrame(() => autoGrowTextarea(textarea))
  return textarea
}

function autoGrowTextarea(textarea: HTMLTextAreaElement) {
  textarea.style.height = 'auto'
  textarea.style.height = `${textarea.scrollHeight}px`
}

function bindCommonEditorKeys(textarea: HTMLTextAreaElement, options: SourceBlockEditorOptions, bindBlur = true) {
  textarea.addEventListener('keydown', (event) => {
    if ((event.metaKey || event.ctrlKey) && event.key === 's') {
      event.preventDefault()
      options.onSave()
      return
    }
    if (event.key === 'Escape') {
      event.preventDefault()
      textarea.blur()
      return
    }
    if ((event.key === 'ArrowUp' || event.key === 'ArrowDown') && !event.metaKey && !event.ctrlKey && !event.altKey) {
      const delta = event.key === 'ArrowUp' ? -1 : 1
      const movement = getBoundaryLineMovement(textarea, options.range, delta)
      if (movement && options.onMoveLine(delta, movement)) {
        event.preventDefault()
      }
    }
  })
  if (bindBlur) textarea.addEventListener('blur', options.onBlur)
}

function getBoundaryLineMovement(textarea: HTMLTextAreaElement, range: LineRange, delta: -1 | 1): FocusRequest | null {
  const cursor = getTextareaDocumentCursor(textarea, range)
  const local = getTextareaLocalCursor(textarea)
  const lineCount = textarea.value.split('\n').length
  const isCodeContent = textarea.classList.contains('md-code-source-input')

  if (delta < 0 && local.line === 0) {
    return isCodeContent ? { line: range.startLine, column: cursor.column } : cursor
  }
  if (delta > 0 && local.line === lineCount - 1) {
    return isCodeContent ? { line: range.endLine, column: cursor.column } : cursor
  }
  return null
}

function getTextareaLocalCursor(textarea: HTMLTextAreaElement): { line: number; column: number } {
  const value = textarea.value
  const cursor = textarea.selectionStart ?? 0
  const beforeCursor = value.slice(0, cursor)
  const line = beforeCursor.split('\n').length - 1
  const lineStart = beforeCursor.lastIndexOf('\n') + 1
  return { line, column: cursor - lineStart }
}

function getTextareaDocumentCursor(textarea: HTMLTextAreaElement, range: LineRange): FocusRequest {
  const local = getTextareaLocalCursor(textarea)
  return {
    line: range.startLine + getTextareaDocumentLineOffset(textarea) + local.line,
    column: local.column,
  }
}

function getTextareaDocumentLineOffset(textarea: HTMLTextAreaElement): number {
  return textarea.classList.contains('md-code-source-input') ? 1 : 0
}

function setTextareaCursorForDocumentLine(textarea: HTMLTextAreaElement, documentLine: number, column: number) {
  const lineOffset = getTextareaDocumentLineOffset(textarea)
  const rangeStart = Number(getTextareaBlockElement(textarea)?.dataset.startLine ?? '0')
  const localLine = Math.max(0, documentLine - rangeStart - lineOffset)
  const lines = textarea.value.split('\n')
  const clampedLine = Math.min(localLine, Math.max(0, lines.length - 1))
  const lineStart = getTextareaLineStartOffset(lines, clampedLine)
  const target = lineStart + Math.min(column, lines[clampedLine]?.length ?? 0)
  textarea.setSelectionRange(target, target)
  textarea.focus()
}

function getTextareaLineStartOffset(lines: string[], line: number): number {
  let offset = 0
  for (let index = 0; index < line; index++) offset += (lines[index]?.length ?? 0) + 1
  return offset
}

function buildCodeBlockSource(lang: string, code: string): string {
  return `\`\`\`${lang.trim()}\n${code}\n\`\`\``
}

function createBlockElement(block: Block, baseDir: string | null): HTMLElement {
  const kind = block.kind

  switch (kind.type) {
    case 'heading': {
      const tag = `h${kind.value.level}` as 'h1' | 'h2' | 'h3' | 'h4' | 'h5' | 'h6'
      const el = document.createElement(tag)
      el.className = 'md-block md-heading md-rendered-block'
      el.dataset.blockType = 'heading'
      el.dataset.level = String(kind.value.level)
      const headingText = extractText(kind.value.content)
      if (headingText) el.id = slugify(headingText)
      renderInlines(kind.value.content, el, baseDir)
      return el
    }

    case 'paragraph': {
      const el = document.createElement('p')
      el.className = 'md-block md-paragraph md-rendered-block'
      el.dataset.blockType = 'paragraph'
      renderInlines(kind.value, el, baseDir)
      return el
    }

    case 'code_block': {
      return createCodeBlockElement(kind.value.lang, kind.value.code)
    }

    case 'table': {
      return createTableElement(kind.value, baseDir)
    }

    case 'block_quote': {
      const el = document.createElement('blockquote')
      el.className = 'md-block md-blockquote md-rendered-block'
      el.dataset.blockType = 'blockquote'
      for (const inner of kind.value) {
        el.appendChild(createBlockElement(inner, baseDir))
      }
      return el
    }

    case 'list': {
      return createListElement(kind.value, baseDir)
    }

    case 'html_block': {
      const el = document.createElement('div')
      el.className = 'md-block md-html-block md-rendered-block'
      el.dataset.blockType = 'html_block'
      const sanitized = kind.value
        .replace(/<script[\s\S]*?<\/script>/gi, '')
        .replace(/\son\w+\s*=/gi, ' data-removed=')
      el.innerHTML = sanitized
      return el
    }

    case 'rule': {
      const el = document.createElement('hr')
      el.className = 'md-block md-hr md-rendered-block'
      el.dataset.blockType = 'rule'
      return el
    }

    default: {
      const el = document.createElement('p')
      el.className = 'md-block md-paragraph md-rendered-block'
      el.dataset.blockType = 'paragraph'
      return el
    }
  }
}

function createTableElement(
  data: { alignments: Alignment[]; rows: Inline[][][] },
  baseDir: string | null
): HTMLElement {
  const wrap = document.createElement('div')
  wrap.className = 'md-block md-table-wrap md-rendered-block'
  wrap.dataset.blockType = 'table'

  const table = document.createElement('table')
  table.className = 'md-table'

  const { rows, alignments } = data

  if (rows.length > 0) {
    const thead = document.createElement('thead')
    const headerRow = document.createElement('tr')
    for (let c = 0; c < rows[0].length; c++) {
      const th = document.createElement('th')
      th.dataset.col = String(c)
      applyCellAlignment(th, alignments[c])
      renderInlines(rows[0][c], th, baseDir)
      headerRow.appendChild(th)
    }
    thead.appendChild(headerRow)
    table.appendChild(thead)

    if (rows.length > 1) {
      const tbody = document.createElement('tbody')
      for (let r = 1; r < rows.length; r++) {
        const tr = document.createElement('tr')
        for (let c = 0; c < rows[r].length; c++) {
          const td = document.createElement('td')
          td.dataset.col = String(c)
          td.dataset.row = String(r)
          applyCellAlignment(td, alignments[c])
          renderInlines(rows[r][c], td, baseDir)
          tr.appendChild(td)
        }
        tbody.appendChild(tr)
      }
      table.appendChild(tbody)
    }
  }

  wrap.appendChild(table)
  return wrap
}

function applyCellAlignment(cell: HTMLElement, align: Alignment | undefined) {
  if (align === 'center') cell.style.textAlign = 'center'
  else if (align === 'right') cell.style.textAlign = 'right'
  else if (align === 'left') cell.style.textAlign = 'left'
}

function trimCodeBlockDisplayNewline(code: string): string {
  return code.endsWith('\n') ? code.slice(0, -1) : code
}

function createCodeBlockElement(lang: string, code: string): HTMLElement {
  const wrap = document.createElement('div')
  wrap.className = 'md-block md-code-wrap md-rendered-block'
  wrap.dataset.blockType = 'code'
  wrap.dataset.lang = lang

  const label = document.createElement('div')
  label.className = 'md-code-lang'
  label.textContent = lang || 'code'
  label.dataset.mdMarker = 'true'
  wrap.appendChild(label)

  const pre = document.createElement('pre')
  pre.className = 'md-code-pre'

  const codeEl = document.createElement('code')
  codeEl.className = 'md-code-content'
  codeEl.appendChild(renderHighlightedCode(trimCodeBlockDisplayNewline(code), lang))

  pre.appendChild(codeEl)
  wrap.appendChild(pre)

  return wrap
}

function createListElement(data: ListData, baseDir: string | null): HTMLElement {
  const el = document.createElement(data.ordered ? 'ol' : 'ul')
  el.className = `md-block md-list md-rendered-block ${data.ordered ? 'md-ol' : 'md-ul'}`
  el.dataset.blockType = 'list'

  data.items.forEach((item, index) => {
    el.appendChild(createListItemNode(item, baseDir))
    if (data.ordered && data.start_index !== null && index === 0) {
      ;(el as HTMLOListElement).start = data.start_index
    }
  })

  return el
}

function createListItemElement(data: ListData, itemIndex: number, baseDir: string | null): HTMLElement {
  const el = document.createElement(data.ordered ? 'ol' : 'ul')
  el.className = `md-block md-list md-rendered-block ${data.ordered ? 'md-ol' : 'md-ul'}`
  el.dataset.blockType = 'list_item'
  if (data.ordered) (el as HTMLOListElement).start = (data.start_index ?? 1) + itemIndex
  const item = data.items[itemIndex]
  if (item) el.appendChild(createListItemNode(item, baseDir))
  return el
}

function createListItemNode(item: ListData['items'][number], baseDir: string | null): HTMLLIElement {
  const li = document.createElement('li')
  li.className = 'md-list-item'
  if (item.checked !== null) {
    const checkbox = document.createElement('input')
    checkbox.type = 'checkbox'
    checkbox.checked = item.checked
    checkbox.className = 'md-checkbox'
    checkbox.disabled = true
    li.appendChild(checkbox)
    li.classList.add('md-task-item')
  }
  const span = document.createElement('span')
  span.className = 'md-list-text'
  renderInlines(item.content, span, baseDir)
  li.appendChild(span)
  return li
}
