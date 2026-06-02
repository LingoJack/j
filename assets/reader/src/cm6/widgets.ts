/**
 * 图片 / inline HTML widget —— 替换 markdown 源码中的 `![alt](url)` 与裸 HTML 节点。
 *
 * 设计：
 * - **图片**：Lezer 解析出 `Image` 节点 → 抓取 url + alt → 用 WidgetType 在
 *   该范围尾部"附加"一张实图（block widget）；同时把 `![alt](url)` 文本本身
 *   在光标不在该行时 hide 掉。
 *   走完整 hide+widget 模式：光标进入该行 → 暴露源码可编辑；离开 → 看到图。
 *
 * - **HTML**：Lezer 给出 `HTMLBlock` / `HTMLTag` 节点（HTMLTag 是 inline）。
 *   光标不在该范围时用 WidgetType 渲染真 DOM（innerHTML），并隐藏原始 HTML 文本；
 *   光标进入则还原。
 *   安全：reader 只打开用户本地 markdown，视为可信源；但仍剥离 script / iframe /
 *   object / embed / link / style 等危险标签，避免无意中执行远程脚本。
 */
import { syntaxTree } from '@codemirror/language'
import {
  Decoration,
  type DecorationSet,
  EditorView,
  WidgetType,
} from '@codemirror/view'
import { type EditorState, type Extension, type Range } from '@codemirror/state'
import { resolveAssetUrl } from '../assetUrl'

// ---------------------------------------------------------------------------
// 图片 widget
// ---------------------------------------------------------------------------

class ImageWidget extends WidgetType {
  readonly src: string
  readonly alt: string
  constructor(src: string, alt: string) {
    super()
    this.src = src
    this.alt = alt
  }
  eq(other: ImageWidget) {
    return other.src === this.src && other.alt === this.alt
  }
  toDOM() {
    const wrap = document.createElement('span')
    wrap.className = 'cm-md-image-widget'
    const img = document.createElement('img')
    img.src = this.src
    img.alt = this.alt
    img.addEventListener('error', () => {
      wrap.classList.add('errored')
      wrap.title = `图片加载失败：${this.src}`
    })
    wrap.appendChild(img)
    if (this.alt) {
      const cap = document.createElement('span')
      cap.className = 'cap'
      cap.textContent = this.alt
      wrap.appendChild(cap)
    }
    return wrap
  }
  ignoreEvent() {
    return true
  }
}

// ---------------------------------------------------------------------------
// HTML widget
// ---------------------------------------------------------------------------

const DANGEROUS_TAGS = [
  'script',
  'iframe',
  'object',
  'embed',
  'link',
  'style',
  'meta',
  'base',
  'frame',
  'frameset',
]

/**
 * 朴素 sanitize：用临时 DOM 解析 → 走 querySelectorAll 把危险标签全 remove。
 * 这不是完整的 XSS 防御（DOMPurify 才是），但 reader 信任本地内容，
 * 这里更多是防"无意中粘了别处的脚本"。
 */
function sanitizeHtml(html: string): string {
  const tpl = document.createElement('template')
  tpl.innerHTML = html
  for (const sel of DANGEROUS_TAGS) {
    tpl.content.querySelectorAll(sel).forEach((el) => el.remove())
  }
  // on* 事件属性也剥掉
  tpl.content.querySelectorAll('*').forEach((el) => {
    for (const attr of Array.from(el.attributes)) {
      if (/^on/i.test(attr.name)) el.removeAttribute(attr.name)
      // javascript: URL
      if (/^(href|src|xlink:href)$/i.test(attr.name)) {
        const v = attr.value.trim()
        if (/^javascript:/i.test(v)) el.removeAttribute(attr.name)
      }
    }
  })
  return tpl.innerHTML
}

class HtmlInlineWidget extends WidgetType {
  readonly raw: string
  readonly baseDir: string | null
  readonly block: boolean
  constructor(raw: string, baseDir: string | null, block: boolean) {
    super()
    this.raw = raw
    this.baseDir = baseDir
    this.block = block
  }
  eq(other: HtmlInlineWidget) {
    return other.raw === this.raw && other.block === this.block
  }
  toDOM() {
    const host = document.createElement(this.block ? 'div' : 'span')
    host.className = 'cm-md-html-widget'
    host.innerHTML = sanitizeHtml(this.raw)
    // 把内部相对路径的 src 解析到 /api/asset
    host.querySelectorAll('img,source,video,audio').forEach((el) => {
      const src = el.getAttribute('src')
      if (src) {
        const resolved = resolveAssetUrl(src, this.baseDir)
        if (resolved && resolved !== src) el.setAttribute('src', resolved)
      }
    })
    return host
  }
  ignoreEvent() {
    return false // 让点击 / 选中能交给浏览器
  }
}

// ---------------------------------------------------------------------------
// 装饰构建
// ---------------------------------------------------------------------------
//
// CM6 硬性约束：`Decoration.replace({block: true, ...})` **必须**通过
// `EditorView.decorations.compute(...)` 这种 facet provider（等价于 StateField）
// 提供；从 `ViewPlugin` 暴露 block 装饰会抛
// "Block decorations may not be specified via plugins"。
// HTMLBlock 是块级（如 README 里 `<div align="center">…</div>`），所以这部分
// 走 facet；inline 的 Image / HTMLTag 仍然能从 ViewPlugin 提供，但为了简单
// 一并合到 facet provider 里。
//
// 代价：facet provider 不知道 viewport，必须扫全文。但语法树本身有缓存，
// 实际成本可接受 —— Image/HTML 节点扫描比代码块高亮便宜得多。

function buildWidgetDecos(state: EditorState, baseDir: string | null): DecorationSet {
  const ranges: Range<Decoration>[] = []

  syntaxTree(state).iterate({
    enter: (node) => {
      const name = node.name
      const nodeFrom = node.from
      const nodeTo = node.to

      if (name === 'Image') {
        // 解析 ![alt](url) —— 永久替成实图 widget，不再因为光标位置切换。
        // 用户要改 url 时直接进文档里改，Lezer 重解析失败（没匹配 Image 节点）
        // 自然就回到 ![alt](url) 文本形态。
        const raw = state.doc.sliceString(nodeFrom, nodeTo)
        const m = /^!\[([^\]]*)\]\(([^)\s]+)(?:\s+"[^"]*")?\)/.exec(raw)
        if (!m) return undefined
        const alt = m[1]
        const url = m[2]
        const src = resolveAssetUrl(url, baseDir)
        ranges.push(
          Decoration.replace({
            widget: new ImageWidget(src, alt),
          }).range(nodeFrom, nodeTo),
        )
        return false
      }

      if (name === 'HTMLBlock') {
        const raw = state.doc.sliceString(nodeFrom, nodeTo)
        ranges.push(
          Decoration.replace({
            widget: new HtmlInlineWidget(raw, baseDir, true),
            block: true,
          }).range(nodeFrom, nodeTo),
        )
        return false
      }

      if (name === 'HTMLTag') {
        const raw = state.doc.sliceString(nodeFrom, nodeTo)
        ranges.push(
          Decoration.replace({
            widget: new HtmlInlineWidget(raw, baseDir, false),
          }).range(nodeFrom, nodeTo),
        )
        return false
      }
      return undefined
    },
  })
  return Decoration.set(ranges, true)
}

/**
 * 工厂：传 baseDir 进来，每次切 tab Reader 整体 remount。
 *
 * 不再监听 selectionSet —— 图片 / HTML widget 的存在与否完全由语法树决定，
 * 跟光标位置无关。这是 Typora 体验：图永远是图，破坏语法（比如把 url 删半截）
 * 才会回到源码。
 *
 * 用 `EditorView.decorations.compute` 是因为 HTMLBlock 是块级 widget，
 * CM6 不允许块级装饰从 ViewPlugin 提供。
 */
export function widgetsExtension(baseDir: string | null): Extension {
  return EditorView.decorations.compute(['doc'], (state) =>
    buildWidgetDecos(state, baseDir),
  )
}
