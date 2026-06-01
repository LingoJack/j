/**
 * Reader 共享类型与上下文。
 *
 * 历史包袱：这里曾经放着完整的 IR → React 渲染管线（renderInline /
 * renderBlock / renderList / renderTable …），用于块拼接编辑器
 * `MarkdownLiveEditor` 的只读 block 渲染。迁到 Milkdown（ProseMirror）
 * 之后，渲染走 ProseMirror 自身的 schema，整套渲染函数都不再被任何地方
 * 引用，已删除。
 *
 * 仅保留：
 * - `MarkdownBaseDirContext` —— Reader 通过它把当前 tab 所在目录传给
 *   `MilkdownEditor`（用于解析图片相对路径）
 * - `extractText` —— TOC 等处需要从 Inline[] 提纯文本时用
 */

import { createContext } from 'react'
import type { Inline } from './types'

/** 当前文档所在目录的绝对路径；目录入口 / 找不到时为 null */
export const MarkdownBaseDirContext = createContext<string | null>(null)

/** 从 Inline[] 提纯文本（用于 heading id slug 等） */
export function extractText(inlines: Inline[]): string {
  return inlines
    .map((inline) => {
      switch (inline.type) {
        case 'text':
          return inline.value
        case 'strong':
        case 'emphasis':
        case 'strikethrough':
          return extractText(inline.value)
        case 'code':
          return inline.value
        case 'link':
          return extractText(inline.value.text)
        default:
          return ''
      }
    })
    .join('')
}
