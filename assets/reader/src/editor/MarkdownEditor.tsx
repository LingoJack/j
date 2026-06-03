/**
 * Markdown 编辑器 —— 消费 Rust 后端 IR，自行渲染 DOM。
 *
 * 核心思路：
 * - 接收后端 ParsedDocument（pulldown_cmark 解析结果），渲染为 contenteditable DOM
 * - 编辑后从 DOM 序列化回 markdown source
 * - source 变化时 POST /api/parse 获取新 IR，增量更新 DOM
 * - 前端不解析 markdown，解析全部由 Rust 后端完成
 */
import { useEffect, useRef, useCallback } from 'react'
import type { Block, ParsedDocument, Inline, Alignment } from '../types'
import { RenderCache, blockKey } from './cache'
import { resetInlineCache, renderInlines } from './inline-renderer'

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
  const renderCache = useRef(new RenderCache())
  const updatingRef = useRef(false) // 防止循环更新
  const debounceTimer = useRef<ReturnType<typeof setTimeout> | null>(null)
  const lastParsedSource = useRef('') // 防止重复解析
  const parseSeq = useRef(0)
  const renderBlocksRef = useRef<(blocks: Block[], skipActive?: boolean) => void>(() => {})

  const onChangeRef = useLatest(onChange)
  const onParsedRef = useLatest(onParsed)
  const onSaveRef = useLatest(onSave)
  const pathRef = useLatest(path)
  const baseDirRef = useLatest(baseDir)

  const parseAndRender = useCallback((source: string, skipActive: boolean) => {
    if (source === lastParsedSource.current) {
      renderBlocksRef.current(docRef.current.blocks, skipActive)
      return
    }

    const seq = ++parseSeq.current
    lastParsedSource.current = source
    fetchParse(source)
      .then(doc => {
        if (seq !== parseSeq.current) return
        docRef.current = doc
        onParsedRef.current(pathRef.current, doc)
        renderBlocksRef.current(doc.blocks, skipActive)
      })
      .catch(() => { /* 静默失败 */ })
  }, [onParsedRef, pathRef])

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
      parseAndRender(md, true)
    }, 300)
  }, [onChangeRef, parseAndRender, pathRef])

  const flushParseAfterFocusChange = useCallback(() => {
    window.setTimeout(() => {
      const host = hostRef.current
      if (!host) return

      syncFromDom()
      if (debounceTimer.current) {
        clearTimeout(debounceTimer.current)
        debounceTimer.current = null
      }

      const skipActive = host.contains(document.activeElement)
      parseAndRender(sourceRef.current, skipActive)
    }, 0)
  }, [parseAndRender, syncFromDom])

  // ---- Cmd/Ctrl+S 保存 ----
  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    if ((e.metaKey || e.ctrlKey) && e.key === 's') {
      e.preventDefault()
      syncFromDom()
      void onSaveRef.current()
    }
  }, [onSaveRef, syncFromDom])

  // ---- Block 渲染 ----
  const renderBlocks = useCallback((blocks: Block[], skipActive?: boolean) => {
    const host = hostRef.current
    if (!host) return

    // 找到当前正在编辑的 block 元素
    const activeBlockEl = skipActive ? findActiveBlockElement(host) : null
    const activeCacheKey = activeBlockEl?.dataset?.cacheKey ?? null
    const activeIndex = activeBlockEl
      ? Array.from(host.children).indexOf(activeBlockEl)
      : -1

    if (skipActive && isHostEditing(host) && !activeBlockEl) {
      return
    }

    // 保存焦点 block 的 DOM 引用（如果有焦点）
    let savedActiveNode: HTMLElement | null = null
    if (activeBlockEl && activeCacheKey) {
      savedActiveNode = activeBlockEl
    }

    try {
      updatingRef.current = true

      // 清空所有现有 DOM（焦点 block 除外）
      const existingChildren = Array.from(host.children) as HTMLElement[]
      for (const child of existingChildren) {
        if (child === savedActiveNode) continue
        child.remove()
      }

      // 清空渲染缓存
      renderCache.current.clear()

      // 重新创建所有 block 的 DOM
      const fragment = document.createDocumentFragment()
      for (let i = 0; i < blocks.length; i++) {
        const block = blocks[i]
        const key = blockKey(block, i)

        // 焦点 block 保留原 DOM
        if (
          savedActiveNode
          && (key === activeCacheKey || i === activeIndex)
        ) {
          savedActiveNode.dataset.cacheKey = key
          renderCache.current.set(key, savedActiveNode)
          fragment.appendChild(savedActiveNode)
          continue
        }

        const el = createBlockElement(block, baseDirRef.current, syncFromDom)
        el.dataset.cacheKey = key
        renderCache.current.set(key, el)
        fragment.appendChild(el)
      }

      host.appendChild(fragment)
    } finally {
      updatingRef.current = false
    }
  }, [baseDirRef, syncFromDom])

  useEffect(() => {
    renderBlocksRef.current = renderBlocks
  }, [renderBlocks])

  // ---- 初始化 + source/doc 变化时全量渲染 ----
  useEffect(() => {
    const host = hostRef.current
    if (!host) return

    host.innerHTML = ''
    renderCache.current.clear()
    resetInlineCache()
    sourceRef.current = initialSource
    docRef.current = initialDoc
    lastParsedSource.current = initialSource

    renderBlocks(initialDoc.blocks)
    onParsedRef.current(path, initialDoc)
  }, [path, initialSource, initialDoc, renderBlocks, onParsedRef])

  // ---- 键盘事件 ----
  useEffect(() => {
    const host = hostRef.current
    if (!host) return
    host.addEventListener('keydown', handleKeyDown)
    host.addEventListener('focusout', flushParseAfterFocusChange)
    return () => {
      host.removeEventListener('keydown', handleKeyDown)
      host.removeEventListener('focusout', flushParseAfterFocusChange)
    }
  }, [flushParseAfterFocusChange, handleKeyDown])

  return (
    <div
      ref={hostRef}
      className="md-editor h-full overflow-auto outline-none"
      contentEditable
      onInput={syncFromDom}
      spellCheck={false}
      suppressContentEditableWarning
    />
  )
}

function isHostEditing(host: HTMLElement): boolean {
  const active = document.activeElement
  if (active instanceof Node && host.contains(active)) {
    return true
  }

  const selection = window.getSelection()
  const anchor = selection?.anchorNode
  return anchor instanceof Node && host.contains(anchor)
}

function findActiveBlockElement(host: HTMLElement): HTMLElement | null {
  const active = document.activeElement
  if (active instanceof HTMLElement && host.contains(active)) {
    const block = active.closest('[data-block-type]')
    if (block instanceof HTMLElement) return block
  }

  const selection = window.getSelection()
  const anchor = selection?.anchorNode
  if (!(anchor instanceof Node) || !host.contains(anchor)) return null

  const el = anchor instanceof HTMLElement ? anchor : anchor.parentElement
  const block = el?.closest('[data-block-type]')
  return block instanceof HTMLElement ? block : null
}

// ---------------------------------------------------------------------------
// Block → DOM 渲染
// ---------------------------------------------------------------------------

function createBlockElement(
  block: Block,
  baseDir: string | null,
  onEdit: () => void,
): HTMLElement {
  const kind = block.kind

  switch (kind.type) {
    case 'heading': {
      const tag = `h${kind.value.level}` as 'h1' | 'h2' | 'h3' | 'h4' | 'h5' | 'h6'
      const el = document.createElement(tag)
      el.className = 'md-block md-heading'
      el.dataset.blockType = 'heading'
      el.dataset.level = String(kind.value.level)
      el.contentEditable = 'true'
      el.appendChild(createMarkdownMarker(`${'#'.repeat(kind.value.level)} `))
      renderInlines(kind.value.content, el)
      el.addEventListener('input', onEdit)
      return el
    }

    case 'paragraph': {
      const el = document.createElement('p')
      el.className = 'md-block md-paragraph'
      el.dataset.blockType = 'paragraph'
      el.contentEditable = 'true'
      renderInlines(kind.value, el)
      el.addEventListener('input', onEdit)
      return el
    }

    case 'code_block': {
      return createCodeBlockElement(kind.value.lang, kind.value.code, onEdit)
    }

    case 'table': {
      return createTableElement(kind.value, onEdit)
    }

    case 'block_quote': {
      const el = document.createElement('blockquote')
      el.className = 'md-block md-blockquote'
      el.dataset.blockType = 'blockquote'
      for (const inner of kind.value) {
        el.appendChild(createBlockElement(inner, baseDir, onEdit))
      }
      return el
    }

    case 'list': {
      return createListElement(kind.value, onEdit)
    }

    case 'html_block': {
      const el = document.createElement('div')
      el.className = 'md-block md-html-block'
      el.dataset.blockType = 'html_block'
      // 安全渲染 HTML（移除 script）
      const sanitized = kind.value.replace(/<script[\s\S]*?<\/script>/gi, '').replace(/\son\w+\s*=/gi, ' data-removed=')
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
      // 未知 block 类型，显示为段落
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
): HTMLElement {
  const wrap = document.createElement('div')
  wrap.className = 'md-block md-table-wrap'
  wrap.dataset.blockType = 'table'

  const table = document.createElement('table')
  table.className = 'md-table'

  const { rows, alignments } = data

  if (rows.length > 0) {
    // Header
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
      renderInlines(rows[0][c], th)
      th.addEventListener('input', onEdit)
      headerRow.appendChild(th)
    }
    thead.appendChild(headerRow)
    table.appendChild(thead)

    // Body
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
          renderInlines(rows[r][c], td)
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

/** 表格内 Tab/Shift+Tab 跳转单元格 */
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
  label.textContent = '```' + lang
  label.dataset.mdMarker = 'true'
  wrap.appendChild(label)

  // 代码区域
  const pre = document.createElement('pre')
  pre.className = 'md-code-pre'

  const codeEl = document.createElement('code')
  codeEl.className = 'md-code-content'
  codeEl.contentEditable = 'true'
  codeEl.textContent = code
  codeEl.addEventListener('input', onEdit)

  pre.appendChild(codeEl)
  wrap.appendChild(pre)

  const footer = document.createElement('div')
  footer.className = 'md-code-fence-end'
  footer.dataset.mdMarker = 'true'
  footer.textContent = '```'
  wrap.appendChild(footer)

  return wrap
}

// ---- 列表 ----

function createListElement(
  data: { ordered: boolean; items: { checked: boolean | null; content: Inline[] }[] },
  onEdit: () => void,
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
    renderInlines(item.content, span)
    span.addEventListener('input', onEdit)
    li.appendChild(span)
    el.appendChild(li)
  }

  return el
}

// ---------------------------------------------------------------------------
// Block 内容增量更新
// ---------------------------------------------------------------------------

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
        const prefix = '#'.repeat(level) + ' '
        lines.push(prefix + serializeInlineContent(el))
        break
      }

      case 'paragraph': {
        lines.push(serializeInlineContent(el))
        break
      }

      case 'code': {
        const lang = el.dataset.lang ?? ''
        lines.push('```' + lang)
        const codeEl = el.querySelector('.md-code-content')
        lines.push(codeEl?.textContent ?? '')
        lines.push('```')
        break
      }

      case 'table': {
        const table = el.querySelector('table')
        if (table) lines.push(...serializeTable(table))
        break
      }

      case 'blockquote': {
        const inner = serializeDomToMarkdown(el)
        for (const line of inner.split('\n')) {
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

    lines.push('') // block 间空行
  }

  return lines.join('\n').replace(/\n{3,}/g, '\n\n').trimEnd() + '\n'
}

/**
 * 把 contenteditable DOM 子节点序列化回 markdown inline 文本。
 * 保留 **bold**、*italic*、`code`、[link](url)、~~strikethrough~~ 等标记。
 */
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
        result += '`' + serializeInlineContent(child) + '`'
      } else if (tag === 'A') {
        const href = child.getAttribute('href') ?? ''
        result += '[' + serializeInlineContent(child) + '](' + href + ')'
      } else if (tag === 'IMG') {
        const src = child.getAttribute('src') ?? ''
        const alt = child.getAttribute('alt') ?? ''
        result += '![' + alt + '](' + src + ')'
      } else if (tag === 'BR') {
        result += '\n'
      } else {
        result += serializeInlineContent(child)
      }
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

  const headerCells = rows[0].querySelectorAll('th, td')
  const colCount = headerCells.length
  result.push('| ' + Array.from(headerCells).map(c => serializeInlineContent(c as HTMLElement).trim()).join(' | ') + ' |')
  result.push('| ' + Array.from({ length: colCount }, () => '---').join(' | ') + ' |')

  for (let i = 1; i < rows.length; i++) {
    const cells = rows[i].querySelectorAll('td, th')
    result.push('| ' + Array.from(cells).map(c => serializeInlineContent(c as HTMLElement).trim()).join(' | ') + ' |')
  }

  return result
}

function serializeList(listEl: HTMLElement): string[] {
  const result: string[] = []
  const isOl = listEl.tagName === 'OL'
  const items = listEl.querySelectorAll(':scope > li')
  let num = 1

  for (const li of Array.from(items)) {
    const checkbox = li.querySelector('input[type="checkbox"]') as HTMLInputElement | null
    const textEl = li.querySelector('.md-list-text') ?? li
    const text = serializeInlineContent(textEl as HTMLElement)

    let prefix: string
    if (checkbox) {
      const mark = checkbox.checked ? '[x]' : '[ ]'
      prefix = `- ${mark} `
    } else if (isOl) {
      prefix = `${num}. `
      num++
    } else {
      prefix = '- '
    }

    result.push(prefix + text)
  }

  return result
}
