/**
 * 从服务端 IR (`/api/parse`) 提取 heading 列表 —— TOC 渲染数据来源。
 *
 * id 必须与 `milkdown/headingId.ts` 给 ProseMirror <h*> 写入的 id 完全
 * 一致，否则 TOC 点击 `document.getElementById(id)` 找不到节点。
 * 双方共用 `./slug` 里的规则。
 */

import type { Block, ParsedDocument } from './types'
import { extractText } from './MarkdownIR'
import { dedupSlugs, slugify } from './slug'

export interface HeadingItem {
  id: string
  level: number
  text: string
}

export function extractHeadings(doc: ParsedDocument): HeadingItem[] {
  const flat: { level: number; text: string; slug: string }[] = []

  function walk(blocks: Block[]) {
    for (const block of blocks) {
      if (block.kind.type === 'heading') {
        const { level, content } = block.kind.value
        const text = extractText(content)
        flat.push({ level, text, slug: slugify(text) })
      } else if (block.kind.type === 'block_quote') {
        walk(block.kind.value)
      }
    }
  }

  walk(doc.blocks)
  const ids = dedupSlugs(flat.map((h) => h.slug))
  return flat.map((h, i) => ({ id: ids[i] ?? '', level: h.level, text: h.text }))
}
