/**
 * DOM Selection ↔ source offset 双向映射（v0 近似版本）。
 *
 * 设计：
 * - 每个 IR block 在 DOM 顶层带 `data-src-start` `data-src-end`（行号）。
 * - selectionToOffset 找到 block 容器 → block 起始行号转 offset → 加 block 内 textContent 偏移
 * - offsetToSelection 反向。
 *
 * v0 已知限制：
 * - 已渲染的行内格式（strong / em / code）内部的字符偏移会与 source 漂移
 *   （DOM 显示 "foo"，source 是 "**foo**"）
 * - 这个漂移在「光标处于 strong 内部」时才表现出来；落在普通段落文本上是准确的
 * - syntax trigger 会通过「拿当前段落 source、在末尾追加新字符、parse」的策略避开此漂移
 * - M1 之后做精确 src-len 映射时再升级
 */

import { lineColToOffset, offsetToLineCol } from './source'

/** 从根 article 找到最近祖先 block 容器（带 data-src-start） */
function findBlockContainer(node: Node | null): HTMLElement | null {
  let cur: Node | null = node
  while (cur) {
    if (cur.nodeType === 1) {
      const el = cur as HTMLElement
      if (el.dataset && el.dataset.srcStart !== undefined) {
        return el
      }
    }
    cur = cur.parentNode
  }
  return null
}

/** 计算 root 内 startContainer + startOffset 到 root 起始的 textContent 长度 */
function textOffsetWithin(
  root: HTMLElement,
  endContainer: Node,
  endOffset: number,
): number {
  let count = 0
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, null)
  let node = walker.nextNode() as Text | null
  while (node) {
    if (node === endContainer) {
      return count + endOffset
    }
    count += node.length
    node = walker.nextNode() as Text | null
  }
  // 选区落在元素节点（非文本）
  if (endContainer.nodeType === 1) {
    // endOffset 是子节点索引；累加前 endOffset 个子节点的 textContent 长度
    const el = endContainer as HTMLElement
    for (let i = 0; i < endOffset && i < el.childNodes.length; i++) {
      count += el.childNodes[i].textContent?.length ?? 0
    }
  }
  return count
}

/**
 * 当前选区 → source offset。
 *
 * @param article 编辑器根（contenteditable）
 * @param source 当前 markdown source
 * @returns offset；selection 不在 article 内时返回 null
 */
export function selectionToOffset(
  article: HTMLElement,
  source: string,
): number | null {
  const sel = window.getSelection()
  if (!sel || sel.rangeCount === 0) return null
  const range = sel.getRangeAt(0)
  if (!article.contains(range.startContainer)) return null

  const block = findBlockContainer(range.startContainer)
  if (!block) {
    // 选区在 article 顶层（罕见）；退化为整文 text offset
    return Math.min(
      textOffsetWithin(article, range.startContainer, range.startOffset),
      source.length,
    )
  }

  const startLine = parseInt(block.dataset.srcStart ?? '0', 10)
  // block 在 source 中的起始 offset = 从 startLine 行第 0 列
  const blockSourceStart = lineColToOffset(source, startLine, 0)

  // block 内 textContent 偏移（粗略）
  const textOffset = textOffsetWithin(
    block,
    range.startContainer,
    range.startOffset,
  )

  return Math.min(blockSourceStart + textOffset, source.length)
}

/**
 * source offset → DOM 选区（用于重渲染后恢复光标）。
 *
 * @returns 是否成功设置 selection
 */
export function offsetToSelection(
  article: HTMLElement,
  source: string,
  offset: number,
): boolean {
  const { line, col } = offsetToLineCol(source, offset)

  // 找到包含该行的 block 容器
  const blocks = article.querySelectorAll<HTMLElement>('[data-src-start]')
  let target: HTMLElement | null = null
  for (const b of Array.from(blocks)) {
    const start = parseInt(b.dataset.srcStart ?? '0', 10)
    const end = parseInt(b.dataset.srcEnd ?? '0', 10)
    if (start <= line && line <= end) {
      target = b
      break
    }
  }
  if (!target) {
    // 选最后一个 block 末尾
    target = blocks.length > 0 ? blocks[blocks.length - 1] : article
  }

  // block 内的 textContent 偏移 = 跨过 (line - startLine) 个行 + col
  // 但因为 DOM 内的渲染态可能与 source 长度不一致，这里走「block 内文本距离起始多少」的粗略法：
  // 拿 source 的 block 范围切片，从头数到 (target line, col) 的 textOffset
  const startLine = parseInt(target.dataset.srcStart ?? '0', 10)
  const blockSourceStart = lineColToOffset(source, startLine, 0)
  let blockInternalOffset = Math.max(0, offset - blockSourceStart)

  // 在 target 的文本节点里走 blockInternalOffset 个字符
  const walker = document.createTreeWalker(target, NodeFilter.SHOW_TEXT, null)
  let node = walker.nextNode() as Text | null
  let remaining = blockInternalOffset
  while (node) {
    if (remaining <= node.length) {
      const range = document.createRange()
      try {
        range.setStart(node, remaining)
        range.collapse(true)
        const sel = window.getSelection()
        if (sel) {
          sel.removeAllRanges()
          sel.addRange(range)
        }
        return true
      } catch {
        return false
      }
    }
    remaining -= node.length
    node = walker.nextNode() as Text | null
  }

  // 文本不够 —— 落到 target 末尾
  const range = document.createRange()
  try {
    range.selectNodeContents(target)
    range.collapse(false)
    const sel = window.getSelection()
    if (sel) {
      sel.removeAllRanges()
      sel.addRange(range)
    }
    return true
  } catch {
    return false
  }
}
