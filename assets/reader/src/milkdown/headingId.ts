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
 * 2. `seeyueHeadingIdSync` —— $prose plugin，通过 `state.appendTransaction`
 *    在每个 transaction 提交前补一个修正性 transaction，把所有 heading 的
 *    `attrs.id` 改成"按出现顺序去重"的形式。
 *
 * 历史教训：曾经把这事放在 `view.update` 里，自己 `view.dispatch(tr)`。
 * 当 transaction 来源是别的插件触发的 attr 变更时，新的 dispatch 会再次
 * 触发 view.update → dispatch，无限递归 → "Maximum call stack size exceeded"。
 * appendTransaction 是 ProseMirror 给修正性 transaction 设计的钩子，运行时
 * 保证不会循环（追加 tr 的迭代有上限）。
 */

import { headingIdGenerator } from '@milkdown/kit/preset/commonmark'
import { $prose } from '@milkdown/kit/utils'
import { Plugin, PluginKey } from '@milkdown/kit/prose/state'
import type { Ctx } from '@milkdown/ctx'
import type { Node as ProseNode } from '@milkdown/kit/prose/model'
import { slugify } from '../slug'

/** 在 Editor.config 时调用：覆写 commonmark 默认 generator */
export function seeyueHeadingIdConfig(ctx: Ctx): void {
  ctx.set(headingIdGenerator.key, (node: ProseNode) => slugify(node.textContent))
}

const SEEYUE_HEADING_ID_KEY = new PluginKey('SEEYUE_HEADING_ID_SYNC')

/**
 * appendTransaction 钩子：
 * - 第一遍（doc 真变了）：算 expected id，发现不匹配 → 返回 setNodeMarkup tr。
 * - 第二遍（这是我们自己的 setNodeMarkup 之后被 ProseMirror 喊回来）：
 *   doc 已经达到目标态 → expected id 全部匹配 → 返回 null，链路终止。
 *
 * 这个"自己看自己一次回收"的模式，正是 PM appendTransaction 的标准用法。
 */
export const seeyueHeadingIdSync = $prose(() => {
  return new Plugin({
    key: SEEYUE_HEADING_ID_KEY,
    appendTransaction: (transactions, _oldState, newState) => {
      // 只在 doc 真变化时跑（attr-only 变更时 docChanged 也是 true，但代价小）
      const anyDocChange = transactions.some((t) => t.docChanged)
      if (!anyDocChange) return null

      const counts: Record<string, number> = {}
      let tr = newState.tr.setMeta('addToHistory', false)
      let changed = false

      newState.doc.descendants((node, pos) => {
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

      return changed ? tr : null
    },
  })
})
