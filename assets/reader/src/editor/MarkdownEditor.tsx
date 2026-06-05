/**
 * Markdown 编辑器 —— 消费 Rust 后端 IR，自行渲染 DOM。
 *
 * 核心思路：
 * - 接收后端 ParsedDocument（pulldown-cmark 解析结果），渲染为 contenteditable DOM
 * - 编辑后从 DOM 序列化回 markdown source
 * - source 变化时 POST /api/parse 获取新 IR
 * - 解析完成后做真正的增量 DOM 更新：只替换内容有变化的非活跃 block，保留光标所在 block 完全不动
 * - 前端不解析 markdown，解析全部由 Rust 后端完成
 */
import { useEffect, useRef, useCallback } from 'react'
import type { Block, ParsedDocument, Inline, Alignment } from '../types'
import { resetInlineCache, renderInlines } from './inline-renderer'
import { renderHighlightedCode } from './code-highlight'
import { slugify } from '../slug'
import { extractText } from '../MarkdownIR'

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

/** 找到包含光标的 block-level 元素（host 的直接子元素） */
function getActiveBlockIndex(host: HTMLElement): number {
  const sel = window.getSelection()
  if (!sel || sel.rangeCount === 0) return -1
  let node: Node | null = sel.anchorNode
  while (node && node !== host) {
    if (node.parentNode === host) {
      return Array.from(host.children).indexOf(node as Element)
    }
    node = node.parentNode
  }
  return -1
}

/** 获取 block 的文本指纹（用于判断非活跃 block 是否需要更新） */
function blockFingerprint(block: Block): string {
  const k = block.kind
  switch (k.type) {
    case 'paragraph':
      return `p:${inlineFingerprint(k.value)}`
    case 'heading':
      return `h${k.value.level}:${inlineFingerprint(k.value.content)}`
    case 'code_block':
      return `c:${k.value.lang}:${k.value.code}`
    case 'rule':
      return 'hr'
    case 'html_block':
      return `html:${k.value}`
    case 'block_quote':
      return `bq:${k.value.map((b) => blockFingerprint(b)).join('|')}`
    case 'list':
      return `l:${k.value.ordered ? 'o' : 'u'}:${k.value.items.map((it) => `${it.checked}:${inlineFingerprint(it.content)}`).join(',')}`
    case 'table':
      return `t:${k.value.rows.map((row) => row.map((cell) => inlineFingerprint(cell)).join('|')).join('||')}`
    default:
      return 'unknown'
  }
}

function inlineFingerprint(inlines: Inline[]): string {
  return inlines.map(inlineFingerprintOne).join('')
}

function inlineFingerprintOne(i: Inline): string {
  switch (i.type) {
    case 'text':
      return i.value
    case 'strong':
      return `**${inlineFingerprint(i.value)}**`
    case 'emphasis':
      return `*${inlineFingerprint(i.value)}*`
    case 'strikethrough':
      return `~~${inlineFingerprint(i.value)}~~`
    case 'code':
      return `\`${i.value}\``
    case 'link':
      return `[${inlineFingerprint(i.value.text)}](${i.value.url})`
    case 'image':
      return `![${i.value.alt}](${i.value.url})`
    case 'soft_break':
      return ' '
    case 'hard_break':
      return '\n'
    case 'html':
      return i.value
    default:
      return ''
  }
}

/** 获取当前光标在某个 block 可编辑文本中的偏移；忽略 Markdown marker。 */
function getCaretTextOffset(root: HTMLElement): number | null {
  const sel = window.getSelection()
  if (!sel || sel.rangeCount === 0) return null
  const range = sel.getRangeAt(0)
  if (!root.contains(range.startContainer)) return null

  let offset = 0
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT)
  let node = walker.nextNode()
  while (node) {
    if (isMarkerTextNode(node)) {
      node = walker.nextNode()
      continue
    }
    if (node === range.startContainer) {
      return offset + range.startOffset
    }
    offset += node.textContent?.length ?? 0
    node = walker.nextNode()
  }
  return offset
}

/** 按可编辑文本偏移恢复光标；忽略 Markdown marker。 */
function restoreCaretTextOffset(root: HTMLElement, targetOffset: number) {
  const sel = window.getSelection()
  if (!sel) return

  let remaining = targetOffset
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT)
  let fallback: Text | null = null
  let node = walker.nextNode()
  while (node) {
    if (isMarkerTextNode(node)) {
      node = walker.nextNode()
      continue
    }
    const text = node as Text
    fallback = text
    const len = text.textContent?.length ?? 0
    if (remaining <= len) {
      const range = document.createRange()
      range.setStart(text, Math.max(0, remaining))
      range.collapse(true)
      sel.removeAllRanges()
      sel.addRange(range)
      return
    }
    remaining -= len
    node = walker.nextNode()
  }

  if (fallback) {
    const range = document.createRange()
    range.setStart(fallback, fallback.textContent?.length ?? 0)
    range.collapse(true)
    sel.removeAllRanges()
    sel.addRange(range)
  }
}

function isMarkerTextNode(node: Node): boolean {
  const parent = node.parentElement
  return parent?.dataset.mdMarker === 'true'
}

// ---------------------------------------------------------------------------
// 主组件
// ---------------------------------------------------------------------------

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
  const sourceRef = useRef(initialSource)
  const docRef = useRef(initialDoc)
  const updatingRef = useRef(false)
  const debounceTimer = useRef<ReturnType<typeof setTimeout> | null>(null)
  const lastParsedSource = useRef('')
  const parseSeq = useRef(0)
  /** 上一次渲染的 block 指纹列表 */
  const lastFingerprints = useRef<string[]>([])

  const onChangeRef = useLatest(onChange)
  const onParsedRef = useLatest(onParsed)
  const onSaveRef = useLatest(onSave)
  const pathRef = useLatest(path)
  const baseDirRef = useLatest(baseDir)
  const syncFromDomFnRef = useRef<() => void>(() => {})

  // ---- DOM → Markdown 序列化 + 触发后端解析 ----
  const syncFromDom = useCallback(() => {
    if (updatingRef.current) return
    const host = hostRef.current
    if (!host) return

    const md = serializeDomToMarkdown(host)
    if (md === sourceRef.current) return

    sourceRef.current = md
    onChangeRef.current(pathRef.current, md)

    // Debounce 后端解析（300ms）
    if (debounceTimer.current) clearTimeout(debounceTimer.current)
    debounceTimer.current = setTimeout(() => {
      parseAndRender(md)
    }, 300)
  }, [onChangeRef, pathRef])

  syncFromDomFnRef.current = syncFromDom

  // ---- parse + incremental render ----
  const parseAndRender = useCallback(
    (source: string) => {
      if (source === lastParsedSource.current) return

      const seq = ++parseSeq.current
      lastParsedSource.current = source
      fetchParse(source)
        .then((doc) => {
          if (seq !== parseSeq.current) return
          docRef.current = doc
          onParsedRef.current(pathRef.current, doc)
          applyDiff(doc.blocks)
        })
        .catch(() => {
          /* 静默失败 */
        })
      // eslint-disable-next-line react-hooks/exhaustive-deps
    },
    [onParsedRef, pathRef]
  )

  // ---- 真正的增量 DOM 更新 ----
  // 策略：按指纹只替换变化的 block；光标所在 block 也会替换并恢复光标，
  // 这样正在编辑的 Markdown 语法（如 `code`）能在 debounce 解析后立即渲染。
  const applyDiff = useCallback(
    (blocks: Block[]) => {
      const host = hostRef.current
      if (!host) return

      const activeIdx = getActiveBlockIndex(host)
      const activeChild =
        activeIdx >= 0 ? (host.children[activeIdx] as HTMLElement | undefined) : undefined
      const caretOffset = activeChild ? getCaretTextOffset(activeChild) : null
      const newFps = blocks.map((b) => blockFingerprint(b))
      const oldFps = lastFingerprints.current
      const oldChildren = Array.from(host.children) as HTMLElement[]

      const lenMatch = oldChildren.length === blocks.length && oldFps.length === newFps.length
      let restoredBlock: HTMLElement | null = null

      try {
        updatingRef.current = true

        if (lenMatch) {
          // 同数量：逐个比较指纹，只替换有变化的 block
          for (let i = 0; i < blocks.length; i++) {
            if (i < oldFps.length && oldFps[i] === newFps[i]) continue // 指纹相同，跳过
            const newEl = createBlockElement(blocks[i], baseDirRef.current, syncFromDomFnRef)
            oldChildren[i].replaceWith(newEl)
            if (i === activeIdx) restoredBlock = newEl
          }
        } else {
          // 数量不同：重建 block 列表，但尽量复用指纹相同的旧 DOM
          const fragment = document.createDocumentFragment()
          for (let i = 0; i < blocks.length; i++) {
            let nextEl: HTMLElement
            if (i < oldChildren.length && i < oldFps.length && oldFps[i] === newFps[i]) {
              nextEl = oldChildren[i]
            } else {
              nextEl = createBlockElement(blocks[i], baseDirRef.current, syncFromDomFnRef)
            }
            if (i === activeIdx) restoredBlock = nextEl
            fragment.appendChild(nextEl)
          }
          host.replaceChildren(fragment)
        }
      } finally {
        updatingRef.current = false
      }

      if (restoredBlock && caretOffset !== null) {
        restoreCaretTextOffset(restoredBlock, caretOffset)
      }

      lastFingerprints.current = newFps
      // eslint-disable-next-line react-hooks/exhaustive-deps
    },
    [baseDirRef]
  )

  // ---- 全量渲染（初始化 / 切换文件）----
  const fullRender = useCallback(
    (blocks: Block[]) => {
      const host = hostRef.current
      if (!host) return

      const savedScrollTop = host.scrollTop

      try {
        updatingRef.current = true
        host.innerHTML = ''

        for (let i = 0; i < blocks.length; i++) {
          host.appendChild(createBlockElement(blocks[i], baseDirRef.current, syncFromDomFnRef))
        }
        lastFingerprints.current = blocks.map((b) => blockFingerprint(b))
      } finally {
        updatingRef.current = false
      }

      host.scrollTop = savedScrollTop
      // eslint-disable-next-line react-hooks/exhaustive-deps
    },
    [baseDirRef]
  )

  // ---- Cmd/Ctrl+S 保存 ----
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 's') {
        e.preventDefault()
        syncFromDom()
        void onSaveRef.current()
      }
    },
    [onSaveRef, syncFromDom]
  )

  // ---- 仅在切换文件（path 变化）时全量重建 ----
  useEffect(() => {
    const host = hostRef.current
    if (!host) return

    resetInlineCache()
    sourceRef.current = initialSource
    docRef.current = initialDoc
    lastParsedSource.current = initialSource

    fullRender(initialDoc.blocks)
    onParsedRef.current(path, initialDoc)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [path])

  // ---- 输入事件 ----
  useEffect(() => {
    const host = hostRef.current
    if (!host) return

    const onInput = () => {
      syncFromDom()
    }

    host.addEventListener('input', onInput)
    host.addEventListener('keydown', handleKeyDown)
    return () => {
      host.removeEventListener('input', onInput)
      host.removeEventListener('keydown', handleKeyDown)
    }
  }, [syncFromDom, handleKeyDown])

  return (
    <div
      ref={hostRef}
      className="md-editor h-full overflow-auto outline-none"
      contentEditable
      spellCheck={false}
      suppressContentEditableWarning
    />
  )
}

// ---------------------------------------------------------------------------
// Block → DOM 渲染
// ---------------------------------------------------------------------------

function createBlockElement(
  block: Block,
  baseDir: string | null,
  onEditRef: React.RefObject<() => void>
): HTMLElement {
  const kind = block.kind
  const onEdit = () => onEditRef.current?.()

  switch (kind.type) {
    case 'heading': {
      const tag = `h${kind.value.level}` as 'h1' | 'h2' | 'h3' | 'h4' | 'h5' | 'h6'
      const el = document.createElement(tag)
      el.className = 'md-block md-heading'
      el.dataset.blockType = 'heading'
      el.dataset.level = String(kind.value.level)
      // 设置 heading id，供 TOC 点击 scrollIntoView
      const headingText = extractText(kind.value.content)
      if (headingText) el.id = slugify(headingText)
      el.contentEditable = 'true'
      el.appendChild(createMarkdownMarker(`${'#'.repeat(kind.value.level)} `))
      renderInlines(kind.value.content, el, baseDir)
      el.addEventListener('input', onEdit)
      return el
    }

    case 'paragraph': {
      const el = document.createElement('p')
      el.className = 'md-block md-paragraph'
      el.dataset.blockType = 'paragraph'
      el.contentEditable = 'true'
      renderInlines(kind.value, el, baseDir)
      el.addEventListener('input', onEdit)
      return el
    }

    case 'code_block': {
      return createCodeBlockElement(kind.value.lang, kind.value.code, onEdit)
    }

    case 'table': {
      return createTableElement(kind.value, onEdit, baseDir)
    }

    case 'block_quote': {
      const el = document.createElement('blockquote')
      el.className = 'md-block md-blockquote'
      el.dataset.blockType = 'blockquote'
      for (const inner of kind.value) {
        el.appendChild(createBlockElement(inner, baseDir, onEditRef))
      }
      return el
    }

    case 'list': {
      return createListElement(kind.value, onEdit, baseDir)
    }

    case 'html_block': {
      const el = document.createElement('div')
      el.className = 'md-block md-html-block'
      el.dataset.blockType = 'html_block'
      const sanitized = kind.value
        .replace(/<script[\s\S]*?<\/script>/gi, '')
        .replace(/\son\w+\s*=/gi, ' data-removed=')
      el.innerHTML = sanitized
      return el
    }

    case 'rule': {
      const el = document.createElement('hr')
      el.className = 'md-block md-hr'
      el.dataset.blockType = 'rule'
      return el
    }

    default: {
      const el = document.createElement('p')
      el.className = 'md-block md-paragraph'
      el.dataset.blockType = 'paragraph'
      return el
    }
  }
}

// ---- 表格 ----

function createTableElement(
  data: { alignments: Alignment[]; rows: Inline[][][] },
  onEdit: () => void,
  baseDir: string | null
): HTMLElement {
  const wrap = document.createElement('div')
  wrap.className = 'md-block md-table-wrap'
  wrap.dataset.blockType = 'table'

  const table = document.createElement('table')
  table.className = 'md-table'

  const { rows, alignments } = data

  if (rows.length > 0) {
    const thead = document.createElement('thead')
    const headerRow = document.createElement('tr')
    for (let c = 0; c < rows[0].length; c++) {
      const th = document.createElement('th')
      th.contentEditable = 'true'
      th.dataset.col = String(c)
      const align = alignments[c]
      if (align === 'center') th.style.textAlign = 'center'
      else if (align === 'right') th.style.textAlign = 'right'
      else if (align === 'left') th.style.textAlign = 'left'
      renderInlines(rows[0][c], th, baseDir)
      th.addEventListener('input', onEdit)
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
          td.contentEditable = 'true'
          td.dataset.col = String(c)
          td.dataset.row = String(r)
          const align = alignments[c]
          if (align === 'center') td.style.textAlign = 'center'
          else if (align === 'right') td.style.textAlign = 'right'
          else if (align === 'left') td.style.textAlign = 'left'
          renderInlines(rows[r][c], td, baseDir)
          td.addEventListener('input', onEdit)
          td.addEventListener('keydown', handleTableKeyDown)
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

function handleTableKeyDown(e: KeyboardEvent) {
  const td = e.target as HTMLElement
  if (e.key === 'Tab') {
    e.preventDefault()
    const row = td.closest('tr')
    if (!row) return
    const cells = Array.from(row.querySelectorAll('td, th'))
    const idx = cells.indexOf(td)
    if (e.shiftKey) {
      const prev = cells[idx - 1]
      if (prev) (prev as HTMLElement).focus()
      else {
        const prevRow = row.previousElementSibling
        if (prevRow) {
          const prevCells = prevRow.querySelectorAll('td, th')
          const last = prevCells[prevCells.length - 1]
          if (last) (last as HTMLElement).focus()
        }
      }
    } else {
      const next = cells[idx + 1]
      if (next) (next as HTMLElement).focus()
      else {
        const nextRow = row.nextElementSibling
        if (nextRow) {
          const nextCells = nextRow.querySelectorAll('td, th')
          const first = nextCells[0]
          if (first) (first as HTMLElement).focus()
        }
      }
    }
  }
}

// ---- 代码块 ----

function createCodeBlockElement(lang: string, code: string, onEdit: () => void): HTMLElement {
  const wrap = document.createElement('div')
  wrap.className = 'md-block md-code-wrap'
  wrap.dataset.blockType = 'code'
  wrap.dataset.lang = lang

  // 语言标签
  const label = document.createElement('div')
  label.className = 'md-code-lang'
  label.textContent = lang || 'code'
  label.dataset.mdMarker = 'true'
  wrap.appendChild(label)

  const pre = document.createElement('pre')
  pre.className = 'md-code-pre'

  const codeEl = document.createElement('code')
  codeEl.className = 'md-code-content'
  codeEl.contentEditable = 'true'
  // 使用语法高亮渲染
  codeEl.appendChild(renderHighlightedCode(code, lang))
  codeEl.addEventListener('input', onEdit)

  pre.appendChild(codeEl)
  wrap.appendChild(pre)

  return wrap
}

// ---- 列表 ----

function createListElement(
  data: { ordered: boolean; items: { checked: boolean | null; content: Inline[] }[] },
  onEdit: () => void,
  baseDir: string | null
): HTMLElement {
  const el = document.createElement(data.ordered ? 'ol' : 'ul')
  el.className = `md-block md-list ${data.ordered ? 'md-ol' : 'md-ul'}`
  el.dataset.blockType = 'list'

  for (const item of data.items) {
    const li = document.createElement('li')
    li.className = 'md-list-item'
    if (item.checked !== null) {
      const checkbox = document.createElement('input')
      checkbox.type = 'checkbox'
      checkbox.checked = item.checked
      checkbox.className = 'md-checkbox'
      checkbox.disabled = false
      checkbox.addEventListener('change', onEdit)
      li.appendChild(checkbox)
      li.classList.add('md-task-item')
    }
    const span = document.createElement('span')
    span.className = 'md-list-text'
    span.contentEditable = 'true'
    renderInlines(item.content, span, baseDir)
    span.addEventListener('input', onEdit)
    li.appendChild(span)
    el.appendChild(li)
  }

  return el
}

// ---------------------------------------------------------------------------
// DOM → Markdown 序列化
// ---------------------------------------------------------------------------

function serializeDomToMarkdown(host: HTMLElement): string {
  const lines: string[] = []

  for (const child of Array.from(host.children)) {
    const el = child as HTMLElement
    const blockType = el.dataset.blockType

    switch (blockType) {
      case 'heading': {
        const level = Number(el.dataset.level ?? 1)
        lines.push('#'.repeat(level) + ' ' + serializeInlineContent(el))
        break
      }
      case 'paragraph': {
        lines.push(serializeInlineContent(el))
        break
      }
      case 'code': {
        const lang = el.dataset.lang ?? ''
        lines.push('```' + lang)
        lines.push(el.querySelector('.md-code-content')?.textContent ?? '')
        lines.push('```')
        break
      }
      case 'table': {
        const table = el.querySelector('table')
        if (table) lines.push(...serializeTable(table))
        break
      }
      case 'blockquote': {
        for (const line of serializeDomToMarkdown(el).split('\n')) {
          lines.push('> ' + line)
        }
        break
      }
      case 'list': {
        lines.push(...serializeList(el))
        break
      }
      case 'rule': {
        lines.push('---')
        break
      }
      case 'html_block': {
        lines.push(el.innerHTML)
        break
      }
      default:
        lines.push(el.textContent ?? '')
    }
    lines.push('')
  }

  return (
    lines
      .join('\n')
      .replace(/\n{3,}/g, '\n\n')
      .trimEnd() + '\n'
  )
}

function serializeInlineContent(el: HTMLElement): string {
  let result = ''
  for (const node of Array.from(el.childNodes)) {
    if (node.nodeType === Node.TEXT_NODE) {
      result += node.textContent ?? ''
    } else if (node.nodeType === Node.ELEMENT_NODE) {
      const child = node as HTMLElement
      if (child.dataset.mdMarker === 'true') continue
      const tag = child.tagName
      if (tag === 'STRONG' || tag === 'B') {
        result += '**' + serializeInlineContent(child) + '**'
      } else if (tag === 'EM' || tag === 'I') {
        result += '*' + serializeInlineContent(child) + '*'
      } else if (tag === 'DEL' || tag === 'S') {
        result += '~~' + serializeInlineContent(child) + '~~'
      } else if (tag === 'CODE') {
        const text = getEditableText(child)
        result += text ? '`' + text + '`' : ''
      } else if (tag === 'A') {
        result +=
          '[' + serializeInlineContent(child) + '](' + (child.getAttribute('href') ?? '') + ')'
      } else if (tag === 'IMG') {
        result +=
          '![' +
          (child.getAttribute('alt') ?? '') +
          '](' +
          (child.dataset.originalSrc ?? child.getAttribute('src') ?? '') +
          ')'
      } else if (tag === 'BR') {
        result += '\n'
      } else {
        result += serializeInlineContent(child)
      }
    }
  }
  return result
}

function getEditableText(el: HTMLElement): string {
  let result = ''
  for (const node of Array.from(el.childNodes)) {
    if (node.nodeType === Node.TEXT_NODE) {
      result += node.textContent ?? ''
      continue
    }
    if (node.nodeType !== Node.ELEMENT_NODE) continue
    const child = node as HTMLElement
    if (child.dataset.mdMarker === 'true') continue
    if (child.tagName === 'BR') {
      result += '\n'
    } else {
      result += getEditableText(child)
    }
  }
  return result
}

function createMarkdownMarker(text: string): HTMLElement {
  const span = document.createElement('span')
  span.className = 'md-marker'
  span.dataset.mdMarker = 'true'
  span.contentEditable = 'false'
  span.textContent = text
  return span
}

function serializeTable(table: HTMLTableElement): string[] {
  const result: string[] = []
  const rows = Array.from(table.querySelectorAll('tr'))
  if (rows.length === 0) return result
  const colCount = rows[0].querySelectorAll('th, td').length
  result.push(
    '| ' +
      Array.from(rows[0].querySelectorAll('th, td'))
        .map((c) => serializeInlineContent(c as HTMLElement).trim())
        .join(' | ') +
      ' |'
  )
  result.push('| ' + Array.from({ length: colCount }, () => '---').join(' | ') + ' |')
  for (let i = 1; i < rows.length; i++) {
    result.push(
      '| ' +
        Array.from(rows[i].querySelectorAll('td, th'))
          .map((c) => serializeInlineContent(c as HTMLElement).trim())
          .join(' | ') +
        ' |'
    )
  }
  return result
}

function serializeList(listEl: HTMLElement): string[] {
  const result: string[] = []
  const isOl = listEl.tagName === 'OL'
  let num = 1
  for (const li of Array.from(listEl.querySelectorAll(':scope > li'))) {
    const checkbox = li.querySelector('input[type="checkbox"]') as HTMLInputElement | null
    const textEl = li.querySelector('.md-list-text') ?? li
    const text = serializeInlineContent(textEl as HTMLElement)
    if (checkbox) {
      result.push(`- ${checkbox.checked ? '[x]' : '[ ]'} ${text}`)
    } else if (isOl) {
      result.push(`${num++}. ${text}`)
    } else {
      result.push(`- ${text}`)
    }
  }
  return result
}
