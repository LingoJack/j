/**
 * 自定义带缓存的 Prism 高亮 ProseMirror 插件 —— 替换 `@milkdown/plugin-prism`。
 *
 * 上游插件的性能问题：只要 `transaction.docChanged` 触发，`apply` 就会
 * 用 `getDecorations(transaction.doc, ...)` 把**整个文档里所有代码块**全
 * 跑一遍 `refractor.highlight`。文档里有 5+ 代码块、单块大几十行时，
 * 每按一个键就上百毫秒，光标跳动、卡顿是必然的。
 *
 * 这里改成"按代码块缓存高亮"：
 * - 缓存 key = `lang:::text`
 * - 当前 transaction 改动了哪个代码块，只重算那个；其它块用上一轮的
 *   token decoration，再走 `transaction.mapping.map(...)` 自动迁移到新位置
 * - LRU cap 256 条；超过容量自动淘汰旧条目，避免长时间使用内存膨胀
 *
 * `prismConfig` 仍从上游导出 —— 上层（MilkdownEditor.tsx）调用方式不变。
 */

import { findChildren } from '@milkdown/prose'
import { Plugin, PluginKey } from '@milkdown/prose/state'
import { Decoration, DecorationSet } from '@milkdown/prose/view'
import { $prose } from '@milkdown/utils'
import { prismConfig } from '@milkdown/plugin-prism'
import { refractor as defaultRefractor } from 'refractor'
import type { Refractor } from 'refractor/core'
import type { Node } from '@milkdown/prose/model'

const NODE_NAME = 'code_block'
const CACHE_LIMIT = 256

// ---------------------------------------------------------------------------
// 单块高亮：把 refractor 输出的 hast 树拍平为「文本片段 + className 数组」
// ---------------------------------------------------------------------------

interface FlatToken {
  text: string
  className: string[]
}

/**
 * refractor.highlight 返回的 hast 树节点。我们只用 type / value / children /
 * properties.className 这几个字段，类型用 `any` 简化（refractor 自己的类型
 * 来自 `@types/hast`，引入会让该模块的 TS 配置耦合到 hast，不值得）。
 */
type HastNodeLike = {
  type: 'element' | 'text'
  value?: string
  children?: HastNodeLike[]
  properties?: { className?: string[] }
}

function flatNodes(
  nodes: HastNodeLike[],
  className: string[] = [],
): FlatToken[] {
  const out: FlatToken[] = []
  for (const node of nodes) {
    if (node.type === 'element') {
      const cls = node.properties?.className ?? []
      const merged = cls.length ? [...className, ...cls] : className
      out.push(...flatNodes(node.children ?? [], merged))
    } else {
      out.push({ text: node.value ?? '', className })
    }
  }
  return out
}

/** 给定 lang + text，跑一次 refractor.highlight 并拍平成 token 列表。 */
function tokenize(
  refractor: Refractor,
  lang: string,
  text: string,
): FlatToken[] {
  // refractor.highlight 在 lang 没注册时会抛；上层已校验，但加 try 保底
  try {
    const root = refractor.highlight(text, lang) as unknown as {
      children?: HastNodeLike[]
    }
    return flatNodes(root.children ?? [])
  } catch {
    return [{ text, className: [] }]
  }
}

// ---------------------------------------------------------------------------
// LRU 缓存：key=lang+text，value=token 列表
// ---------------------------------------------------------------------------

class TokenCache {
  private map = new Map<string, FlatToken[]>()

  get(key: string): FlatToken[] | undefined {
    const v = this.map.get(key)
    if (v) {
      // 重新插入 → Map 会把它移到末尾（最新使用）
      this.map.delete(key)
      this.map.set(key, v)
    }
    return v
  }

  set(key: string, val: FlatToken[]) {
    if (this.map.has(key)) this.map.delete(key)
    this.map.set(key, val)
    if (this.map.size > CACHE_LIMIT) {
      // 淘汰最旧条目（Map 的迭代顺序按插入顺序）
      const oldestKey = this.map.keys().next().value
      if (oldestKey !== undefined) this.map.delete(oldestKey)
    }
  }
}

// ---------------------------------------------------------------------------
// 把一个代码块的 token 列表转成 ProseMirror Decorations
// ---------------------------------------------------------------------------

function decorationsForBlock(
  blockPos: number,
  tokens: FlatToken[],
): Decoration[] {
  const decos: Decoration[] = []
  // 代码块文本起点 = block.pos + 1（跳过开节点）
  let from = blockPos + 1
  for (const tok of tokens) {
    const to = from + tok.text.length
    if (tok.className.length) {
      decos.push(
        Decoration.inline(from, to, {
          class: tok.className.join(' '),
        }),
      )
    }
    from = to
  }
  return decos
}

// ---------------------------------------------------------------------------
// 插件本体
// ---------------------------------------------------------------------------

interface BlockEntry {
  pos: number
  text: string
  lang: string
}

function listAllCodeBlocks(doc: Node): BlockEntry[] {
  const found = findChildren((node) => node.type.name === NODE_NAME)(doc)
  return found.map((c) => ({
    pos: c.pos,
    text: c.node.textContent,
    lang: (c.node.attrs.language as string | undefined) ?? '',
  }))
}

export const seeyuePrism = $prose((ctx) => {
  const { configureRefractor } = ctx.get(prismConfig.key)
  const refractor =
    (configureRefractor(defaultRefractor) as Refractor | undefined) ??
    defaultRefractor

  const cache = new TokenCache()
  const supported = new Set(refractor.listLanguages())

  function highlightDoc(doc: Node): DecorationSet {
    const blocks = listAllCodeBlocks(doc)
    const decos: Decoration[] = []
    for (const b of blocks) {
      if (!b.lang || !supported.has(b.lang)) continue
      const key = `${b.lang}:::${b.text}`
      let toks = cache.get(key)
      if (!toks) {
        toks = tokenize(refractor, b.lang, b.text)
        cache.set(key, toks)
      }
      decos.push(...decorationsForBlock(b.pos, toks))
    }
    return DecorationSet.create(doc, decos)
  }

  return new Plugin({
    key: new PluginKey('SEEYUE_PRISM'),
    state: {
      init: (_, { doc }) => highlightDoc(doc),
      apply: (tr, oldSet, _oldState, newState) => {
        if (!tr.docChanged) {
          // 选区移动等 —— decorations 完全不变
          return oldSet
        }

        // —— 增量重算 ——
        // 关键：tokenize 走缓存，key=`lang:::text`。所以：
        // - 没改动到的代码块：缓存命中 → 0 计算，直接生成新 deco
        // - 改动到的代码块：缓存 miss → 只跑这一个块的 refractor.highlight
        // - 编辑普通段落（不在代码块里）：所有代码块都缓存命中
        //
        // findChildren 默认 descend=false 只走顶层节点，复杂度
        // O(top-level-blocks)，对 200KB 文档也很轻。
        const blocks = listAllCodeBlocks(newState.doc)
        const allDecos: Decoration[] = []

        for (const b of blocks) {
          if (!b.lang || !supported.has(b.lang)) continue
          const key = `${b.lang}:::${b.text}`
          let toks = cache.get(key)
          if (!toks) {
            toks = tokenize(refractor, b.lang, b.text)
            cache.set(key, toks)
          }
          allDecos.push(...decorationsForBlock(b.pos, toks))
        }
        return DecorationSet.create(newState.doc, allDecos)
      },
    },
    props: {
      decorations(state) {
        return this.getState(state)
      },
    },
  })
})

/** 暴露上游的 prismConfig，使外层注册 refractor 的代码不用改 */
export { prismConfig } from '@milkdown/plugin-prism'

/** 与上游 `prism` 同名导出：plugin 列表 */
export const seeyuePrismBundle = [seeyuePrism, prismConfig]
