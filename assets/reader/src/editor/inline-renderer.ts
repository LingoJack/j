/**
 * Inline markdown → DOM 渲染器。
 *
 * 将 Inline[]（来自 parser）渲染为 DOM 节点，追加到父元素中。
 * 使用 InlineCache 加速重复内容的渲染。
 */
import type { Inline } from '../types'
import { InlineCache } from './cache'

const inlineCache = new InlineCache()

// 重置缓存（tab 切换时）
export function resetInlineCache() {
  inlineCache.clear()
}

/**
 * 将 Inline[] 渲染为 DOM 节点并追加到 parent 中。
 * @param inlines Inline[] 数组
 * @param parent 目标父元素
 */
export function renderInlines(inlines: Inline[], parent: HTMLElement) {
  for (const inline of inlines) {
    parent.appendChild(renderOneInline(inline))
  }
}

/**
 * 创建包含所有 inline 节点的 DocumentFragment。
 */
export function createInlineFragment(inlines: Inline[]): DocumentFragment {
  const frag = document.createDocumentFragment()
  renderInlines(inlines, frag as unknown as HTMLElement)
  return frag
}

function renderOneInline(inline: Inline): Node {
  switch (inline.type) {
    case 'text':
      return document.createTextNode(inline.value)

    case 'strong': {
      const el = document.createElement('strong')
      renderInlines(inline.value, el)
      return el
    }

    case 'emphasis': {
      const el = document.createElement('em')
      renderInlines(inline.value, el)
      return el
    }

    case 'strikethrough': {
      const el = document.createElement('del')
      renderInlines(inline.value, el)
      return el
    }

    case 'code': {
      const el = document.createElement('code')
      el.textContent = inline.value
      return el
    }

    case 'link': {
      const a = document.createElement('a')
      a.href = inline.value.url
      a.target = '_blank'
      a.rel = 'noopener noreferrer'
      renderInlines(inline.value.text, a)
      return a
    }

    case 'image': {
      const img = document.createElement('img')
      img.src = inline.value.url
      img.alt = inline.value.alt
      img.loading = 'lazy'
      return img
    }

    case 'html': {
      // 内联 HTML（如 <br />）
      const span = document.createElement('span')
      span.innerHTML = inline.value
      if (span.childNodes.length === 1) return span.childNodes[0]
      return span
    }

    case 'soft_break':
      return document.createTextNode(' ')

    case 'hard_break':
      return document.createElement('br')

    default:
      return document.createTextNode('')
  }
}
