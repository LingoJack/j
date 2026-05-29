/**
 * Heading id 生成 —— 与服务端 IR slug 规则严格对齐。
 *
 * 服务端（`MarkdownIR.tsx` 的 headingId、`src/markdown/...`）规则：
 *   const slug = text.toLowerCase()
 *     .replace(/[^\w一-鿿]+/g, '-')
 *     .replace(/^-|-$/g, '')
 *   // 重复时：foo, foo-1, foo-2 ...
 *
 * `TableOfContents.tsx` 通过 `document.getElementById(slug)` 滚到目标
 * 标题；只有让 Milkdown 输出的 `<h1..6 id>` 与上述规则完全一致，TOC
 * 才能继续工作。
 *
 * 实现：
 * 1. `seeyueHeadingIdConfig` —— 在 Editor.config 中替换 commonmark 自带
 *    `headingIdGenerator` 的实现，让 toDOM 首次渲染拿到的 id 就是 seeyue slug
 *    （未去重，单纯 slug）。
 * 2. `seeyueHeadingIdSync` —— $prose plugin，每次 doc 变更时遍历所有
 *    heading，按出现顺序去重（foo, foo-1, foo-2 ...），通过
 *    setNodeMarkup + addToHistory:false 写到 attrs.id 上。
 *
 * commonmark 自带的 syncHeadingIdPlugin 也会跑，但我们这只在最后写入
 * 我们的规则（按 view.update 顺序，自定义 plugin 在 use 顺序更靠后即可
 * 后写胜出）。
 */

import { headingIdGenerator } from '@milkdown/kit/preset/commonmark'
import { $prose } from '@milkdown/kit/utils'
import { Plugin, PluginKey } from '@milkdown/kit/prose/state'
import type { Ctx } from '@milkdown/ctx'
import type { Node as ProseNode } from '@milkdown/kit/prose/model'
import type { EditorView } from '@milkdown/kit/prose/view'
import { slugify } from '../slug'

/** 在 Editor.config 时调用：覆写 commonmark 默认 generator */
export function seeyueHeadingIdConfig(ctx: Ctx): void {
  ctx.set(headingIdGenerator.key, (node: ProseNode) => slugify(node.textContent))
}

const SEEYUE_HEADING_ID_KEY = new PluginKey('SEEYUE_HEADING_ID_SYNC')

/**
 * 自定义 $prose plugin —— 与 commonmark 自带 syncHeadingIdPlugin 同形态，
 * 但用 `foo`, `foo-1`, `foo-2` 风格的 dedupe（与服务端 slug 规则一致）。
 *
 * 后注册（`.use(seeyueHeadingIdSync)` 放在 `.use(commonmark)` 之后）即可
 * 让我们的 setNodeMarkup 后写胜出。
 */
export const seeyueHeadingIdSync = $prose(() => {
  const refresh = (view: EditorView) => {
    if (view.composing) return
    const counts: Record<string, number> = {}
    let tr = view.state.tr.setMeta('addToHistory', false)
    let changed = false
    view.state.doc.descendants((node, pos) => {
      if (node.type.name !== 'heading') return
      const base = slugify(node.textContent)
      if (!base) return
      const n = (counts[base] = (counts[base] ?? 0) + 1)
      const id = n === 1 ? base : `${base}-${n - 1}`
      if (node.attrs.id !== id) {
        tr = tr.setNodeMarkup(pos, undefined, { ...node.attrs, id })
        changed = true
      }
    })
    if (changed) view.dispatch(tr)
  }

  return new Plugin({
    key: SEEYUE_HEADING_ID_KEY,
    view: (view) => {
      // 初次挂载也跑一次，让 mounted 后立刻就有正确 id
      queueMicrotask(() => {
        try {
          refresh(view)
        } catch {
          /* view 已 destroy 等竞态，忽略 */
        }
      })
      return {
        update: (next, prev) => {
          if (!next.state.doc.eq(prev.doc)) refresh(next)
        },
      }
    },
  })
})
