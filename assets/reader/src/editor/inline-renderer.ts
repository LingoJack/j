/**
 * Inline markdown → DOM 渲染器。
 *
 * 将 Inline[]（来自 parser）渲染为 DOM 节点，追加到父元素中。
 * 使用 InlineCache 加速重复内容的渲染。
 */
import type { Inline } from '../types'
import { InlineCache } from './cache'
import { isRemoteAssetUrl, resolveAssetUrl } from '../assetUrl'

const inlineCache = new InlineCache()

// 重置缓存（tab 切换时）
export function resetInlineCache() {
  inlineCache.clear()
}

/**
 * 将 Inline[] 渲染为 DOM 节点并追加到 parent 中。
 * @param inlines Inline[] 数组
 * @param parent 目标父元素
 * @param baseDir 当前文档所在目录（用于解析相对图片路径）
 */
export function renderInlines(
  inlines: Inline[],
  parent: HTMLElement,
  baseDir: string | null = null
) {
  for (const inline of inlines) {
    parent.appendChild(renderOneInline(inline, baseDir))
  }
}

/**
 * 创建包含所有 inline 节点的 DocumentFragment。
 */
export function createInlineFragment(
  inlines: Inline[],
  baseDir: string | null = null
): DocumentFragment {
  const frag = document.createDocumentFragment()
  renderInlines(inlines, frag as unknown as HTMLElement, baseDir)
  return frag
}

function renderOneInline(inline: Inline, baseDir: string | null): Node {
  switch (inline.type) {
    case 'text':
      return document.createTextNode(inline.value)

    case 'strong': {
      const el = document.createElement('strong')
      el.appendChild(createMarker('**'))
      renderInlines(inline.value, el, baseDir)
      el.appendChild(createMarker('**'))
      return el
    }

    case 'emphasis': {
      const el = document.createElement('em')
      el.appendChild(createMarker('*'))
      renderInlines(inline.value, el, baseDir)
      el.appendChild(createMarker('*'))
      return el
    }

    case 'strikethrough': {
      const el = document.createElement('del')
      el.appendChild(createMarker('~~'))
      renderInlines(inline.value, el, baseDir)
      el.appendChild(createMarker('~~'))
      return el
    }

    case 'code': {
      const el = document.createElement('code')
      el.appendChild(createMarker('`'))
      el.appendChild(document.createTextNode(inline.value))
      el.appendChild(createMarker('`'))
      return el
    }

    case 'link': {
      const a = document.createElement('a')
      a.href = inline.value.url
      a.target = '_blank'
      a.rel = 'noopener noreferrer'
      a.appendChild(createMarker('['))
      renderInlines(inline.value.text, a, baseDir)
      a.appendChild(createMarker(`](${inline.value.url})`))
      return a
    }

    case 'image': {
      const img = document.createElement('img')
      const originalUrl = inline.value.url
      img.src = resolveAssetUrl(originalUrl, baseDir)
      img.dataset.originalSrc = originalUrl
      img.alt = inline.value.alt
      if (isRemoteAssetUrl(originalUrl)) {
        img.referrerPolicy = 'no-referrer'
      }
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

function createMarker(text: string): HTMLElement {
  const span = document.createElement('span')
  span.className = 'md-marker'
  span.dataset.mdMarker = 'true'
  span.contentEditable = 'false'
  span.textContent = text
  return span
}
