/**
 * 原生 HTML 支持 —— 让 markdown 里的 `<table>` `<details>` `<img>` 等
 * HTML 标签真正渲染成 DOM，而不是当文本显示。
 *
 * 现状：Milkdown commonmark 默认的 `htmlSchema` 是 `atom + inline`，且 toDOM
 * 把 `node.attrs.value` 当成纯文本塞 `<span>`。
 *
 * 关键发现：commonmark 还启用了 `remarkHtmlTransformer`，把 root /
 * listItem / blockquote 下面的 raw `html` 节点全部包成 `paragraph`。也就是说
 * 经 remark 处理之后，**所有** html 节点都已经在 paragraph 内（inline 上下文），
 * 我们再造一份 block 级 schema 反而吃不到。
 *
 * 修复策略：保留默认 inline `htmlSchema`，仅用 `$view` 装一个 NodeView 把
 * 渲染替换掉：
 *   - 检测 `value` 是不是块级 HTML（多行 / 块级标签开头）→ 渲染成 `<div>`
 *   - 否则（如 `<sub>` `<br>` `<kbd>`）→ 渲染成 `<span>`
 * NodeView 返回的 DOM 元素是由 JS 直接 createElement 构造的，浏览器
 * 不会因为 `<table>` 在 `<p>` 里就拒绝渲染（HTML5 parser 限制只在解析
 * HTML 字符串时生效，对已构造的 DOM 树不生效）。
 *
 * 同时把 `<img src="./x.png">` 这种相对路径转换成 `./api/asset?path=...`，
 * 与 markdown 原生 image 节点的 imageResolver 形成对偶覆盖。
 *
 * baseDir（当前文档所在目录）通过一个 `$ctx` 槽位注入；切 tab 时整体
 * remount，新 ctx 拿到新 baseDir。
 *
 * 安全：reader 仅打开用户本地 markdown 文件，视为可信来源，允许 innerHTML。
 */

import { htmlSchema } from '@milkdown/kit/preset/commonmark'
import { $ctx, $view } from '@milkdown/kit/utils'
import type { Ctx } from '@milkdown/ctx'
import { resolveAssetUrl } from '../assetUrl'

// ---------------------------------------------------------------------------
// baseDir context
// ---------------------------------------------------------------------------

/** 当前文档目录的绝对路径，由 MilkdownEditor.tsx 在 mount 时 set */
export const seeyueBaseDirCtx = $ctx<string | null, 'seeyueBaseDir'>(
  null,
  'seeyueBaseDir',
)

// ---------------------------------------------------------------------------
// 块级 / inline HTML 判定
// ---------------------------------------------------------------------------

const BLOCK_TAG_RE =
  /^\s*<\/?(table|tbody|thead|tfoot|tr|td|th|details|summary|div|section|article|aside|nav|header|footer|figure|figcaption|blockquote|pre|h[1-6]|ul|ol|li|hr|p|form|iframe|video|audio|canvas|svg|html|body|head|main|dl|dt|dd|address|fieldset|legend|center|picture|source)\b/i

function looksLikeBlockHtml(value: string): boolean {
  if (!value) return false
  if (value.includes('\n')) return true
  return BLOCK_TAG_RE.test(value)
}

// ---------------------------------------------------------------------------
// 渲染后重写图片相对路径
// ---------------------------------------------------------------------------

function rewriteImgSrc(root: HTMLElement, baseDir: string | null): void {
  if (!baseDir) return
  const imgs = root.querySelectorAll('img')
  imgs.forEach((img) => {
    const src = img.getAttribute('src')
    if (!src) return
    const resolved = resolveAssetUrl(src, baseDir)
    if (resolved !== src) img.setAttribute('src', resolved)
  })
}

// ---------------------------------------------------------------------------
// inline html schema 的 NodeView 替换
// ---------------------------------------------------------------------------

export const htmlInlineView = $view(htmlSchema.node, (ctx: Ctx) => (node) => {
  const value = (node.attrs.value as string) ?? ''
  const isBlock = looksLikeBlockHtml(value)
  const dom = document.createElement(isBlock ? 'div' : 'span')
  dom.setAttribute('data-type', isBlock ? 'html-block' : 'html-inline')
  dom.className = isBlock ? 'seeyue-html-block' : 'seeyue-html-inline'
  dom.contentEditable = 'false'
  dom.innerHTML = value
  rewriteImgSrc(dom, ctx.get(seeyueBaseDirCtx.key))
  return { dom }
})
