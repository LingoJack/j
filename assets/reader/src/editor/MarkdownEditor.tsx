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
  const [activeRange, setActiveRange] = useState<LineRange | null>(null)

  const onChangeRef = useLatest(onChange)
  const onParsedRef = useLatest(onParsed)
  const onSaveRef = useLatest(onSave)
  const pathRef = useLatest(path)
  const baseDirRef = useLatest(baseDir)

  const setActiveRangeSafely = useCallback((next: LineRange | null) => {
    activeRangeRef.current = next
    setActiveRange((prev) => (sameRange(prev, next) ? prev : next))
  }, [])

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
        const listRanges = splitListItemRanges(range, lines, block.kind.value.items.length)
        listRanges.forEach((listRange, itemIndex) => {
          appendBlankLines(host, nextLine, listRange.startLine)
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
                  if (!sameRange(activeRangeRef.current, listRange)) return
                  setActiveRangeSafely(null)
                  parseAndRender(sourceRef.current)
                },
                onSave: () => void onSaveRef.current(),
              })
            )
          } else {
            const el = createListItemElement(block.kind.value, itemIndex, baseDirRef.current)
            el.dataset.startLine = String(listRange.startLine)
            el.dataset.endLine = String(listRange.endLine)
            el.addEventListener('mousedown', (event) => {
              event.preventDefault()
              event.stopPropagation()
              setActiveRangeSafely(listRange)
            })
            host.appendChild(el)
          }
          nextLine = listRange.endLine + 1
        })
        return
      }

      appendBlankLines(host, nextLine, range.startLine)

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
              if (!sameRange(activeRangeRef.current, range)) return
              setActiveRangeSafely(null)
              parseAndRender(sourceRef.current)
            },
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
          setActiveRangeSafely(range)
        })
        host.appendChild(el)
      }

      nextLine = range.endLine + 1
    })
    appendBlankLines(host, nextLine, lines.length)

    if (scrollRef.current) scrollRef.current.scrollTop = savedScrollTop

    const editor =
      host.querySelector<HTMLElement>('.md-preferred-focus') ??
      host.querySelector<HTMLElement>('.md-block-source-input, .md-code-source-input, .md-table-cell-input, .md-code-lang-input')
    if (editor && document.activeElement !== editor) {
      editor.focus()
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

function appendBlankLines(host: HTMLElement, fromLine: number, toLine: number) {
  for (let line = fromLine; line < toLine; line++) {
    const blank = document.createElement('div')
    blank.className = 'md-block md-blank-line'
    blank.dataset.blockType = 'blank'
    blank.dataset.startLine = String(line)
    blank.dataset.endLine = String(line)
    blank.textContent = '\u00a0'
    host.appendChild(blank)
  }
}

type SourceBlockEditorOptions = {
  range: LineRange
  block: Block
  rawValue: string
  onChange: (value: string) => void
  onBlur: () => void
  onSave: () => void
}

function createSourceBlockEditor(options: SourceBlockEditorOptions): HTMLElement {
  const kind = options.block.kind
  if (kind.type === 'code_block') return createCodeBlockEditor(options, kind.value.lang, kind.value.code)
  if (kind.type === 'table') return createTableBlockEditor(options, kind.value)
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
  wrap.className = 'md-block md-code-source'
  wrap.dataset.blockType = 'source_code'
  wrap.dataset.startLine = String(options.range.startLine)
  wrap.dataset.endLine = String(options.range.endLine)

  const toolbar = document.createElement('div')
  toolbar.className = 'md-code-source-toolbar'

  const langLabel = document.createElement('label')
  langLabel.className = 'md-code-source-label'

  const langInput = document.createElement('input')
  langInput.className = 'md-code-lang-input'
  langInput.value = lang
  langInput.placeholder = 'text'
  langInput.spellcheck = false

  langLabel.appendChild(langInput)
  toolbar.appendChild(langLabel)

  const textarea = createAutoGrowTextarea('md-code-source-input md-preferred-focus', trimCodeBlockDisplayNewline(code))
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
  textarea.addEventListener('keydown', (event) => {
    if ((event.metaKey || event.ctrlKey) && event.key === 's') {
      event.preventDefault()
      options.onSave()
    }
    if (event.key === 'Escape') {
      event.preventDefault()
      closeEditor()
    }
  })
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

  wrap.appendChild(toolbar)
  wrap.appendChild(textarea)
  return wrap
}

function createTableBlockEditor(options: SourceBlockEditorOptions, data: { alignments: Alignment[]; rows: Inline[][][] }): HTMLElement {
  const wrap = document.createElement('div')
  wrap.className = 'md-block md-table-source'
  wrap.dataset.blockType = 'source_table'
  wrap.dataset.startLine = String(options.range.startLine)
  wrap.dataset.endLine = String(options.range.endLine)

  const table = document.createElement('table')
  table.className = 'md-table-edit-grid'

  const maxCols = Math.max(data.alignments.length, ...data.rows.map((row) => row.length), 1)
  const rows = data.rows.length > 0 ? data.rows : [[[]]]

  for (let r = 0; r < rows.length; r++) {
    const tr = document.createElement('tr')
    for (let c = 0; c < maxCols; c++) {
      const cell = document.createElement(r === 0 ? 'th' : 'td')
      const textarea = createAutoGrowTextarea(
        'md-table-cell-input md-table-cell-textarea',
        inlinePlainText(rows[r][c] ?? [])
      )
      textarea.dataset.row = String(r)
      textarea.dataset.col = String(c)
      textarea.addEventListener('input', () => {
        autoGrowTextarea(textarea)
        options.onChange(buildTableSource(table, data.alignments))
      })
      textarea.addEventListener('keydown', (event) => {
        if ((event.metaKey || event.ctrlKey) && event.key === 's') {
          event.preventDefault()
          options.onSave()
        }
        if (event.key === 'Escape') {
          event.preventDefault()
          textarea.blur()
        }
      })
      textarea.addEventListener('blur', () => {
        setTimeout(() => {
          if (!wrap.contains(document.activeElement)) options.onBlur()
        }, 0)
      })
      cell.appendChild(textarea)
      tr.appendChild(cell)
    }
    table.appendChild(tr)
  }

  const hint = document.createElement('div')
  hint.className = 'md-table-source-hint'
  hint.textContent = '编辑单元格内容；表格结构暂按当前行列保持。'

  wrap.appendChild(table)
  wrap.appendChild(hint)
  return wrap
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

function bindCommonEditorKeys(textarea: HTMLTextAreaElement, options: SourceBlockEditorOptions) {
  textarea.addEventListener('keydown', (event) => {
    if ((event.metaKey || event.ctrlKey) && event.key === 's') {
      event.preventDefault()
      options.onSave()
    }
    if (event.key === 'Escape') {
      event.preventDefault()
      textarea.blur()
    }
  })
  textarea.addEventListener('blur', options.onBlur)
}

function buildCodeBlockSource(lang: string, code: string): string {
  return `\`\`\`${lang.trim()}\n${code}\n\`\`\``
}

function inlinePlainText(inlines: Inline[]): string {
  return extractText(inlines)
}

function escapeTableCell(value: string): string {
  return value.replace(/\|/g, '\\|').replace(/\r?\n/g, ' ')
}

function tableAlignMarker(align: Alignment | undefined): string {
  if (align === 'left') return ':---'
  if (align === 'center') return ':---:'
  if (align === 'right') return '---:'
  return '---'
}

function buildTableSource(table: HTMLTableElement, alignments: Alignment[]): string {
  const rows = Array.from(table.rows).map((row) =>
    Array.from(row.cells).map((cell) => escapeTableCell(cell.querySelector('textarea')?.value ?? ''))
  )
  const header = rows[0] ?? ['']
  const divider = header.map((_, index) => tableAlignMarker(alignments[index])).join(' | ')
  const body = rows.slice(1)
  return [`| ${header.join(' | ')} |`, `| ${divider} |`, ...body.map((row) => `| ${row.join(' | ')} |`)].join('\n')
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

function createListElement(data: { ordered: boolean; items: { checked: boolean | null; content: Inline[] }[] }, baseDir: string | null): HTMLElement {
  const el = document.createElement(data.ordered ? 'ol' : 'ul')
  el.className = `md-block md-list md-rendered-block ${data.ordered ? 'md-ol' : 'md-ul'}`
  el.dataset.blockType = 'list'

  for (const item of data.items) {
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
    el.appendChild(li)
  }

  return el
}
