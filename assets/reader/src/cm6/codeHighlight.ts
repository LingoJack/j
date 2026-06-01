/**
 * CM6 代码块语法高亮插件，基于 refractor（Prism 内核）+ 按块缓存。
 *
 * 设计要点：
 * - 走 Lezer markdown tree 拿 FencedCode 节点 + CodeInfo（语言）+ 内部代码文本范围
 * - 用 refractor.highlight(text, lang) 拿 hast 树，flatNodes 拍平成
 *   `{ text, className[] }` 列表
 * - 把列表转成 (relFrom, relTo, classes) → 加到全局 from 偏移做 Decoration.mark
 * - 缓存 key = `lang:::text`，hit 直接复用（typing 时只重算改动那一块）
 * - 围栏 ``` 行：光标不在代码块内时整行 hide（语言 chip 由 CSS 给行加底色）
 *
 * 与 livePreview.ts 保持解耦：本文件只暴露一个 ViewPlugin extension。
 */
import { syntaxTree } from '@codemirror/language'
import {
  Decoration,
  type DecorationSet,
  EditorView,
  ViewPlugin,
  type ViewUpdate,
} from '@codemirror/view'
import { type Range } from '@codemirror/state'
import { refractor } from 'refractor'
import bash from 'refractor/bash'
import shell from 'refractor/shell-session'
import javascript from 'refractor/javascript'
import typescript from 'refractor/typescript'
import jsx from 'refractor/jsx'
import tsx from 'refractor/tsx'
import python from 'refractor/python'
import rust from 'refractor/rust'
import go from 'refractor/go'
import cLang from 'refractor/c'
import cpp from 'refractor/cpp'
import csharp from 'refractor/csharp'
import java from 'refractor/java'
import ruby from 'refractor/ruby'
import sql from 'refractor/sql'
import json from 'refractor/json'
import yaml from 'refractor/yaml'
import toml from 'refractor/toml'
import markdownLang from 'refractor/markdown'
import html from 'refractor/markup'
import css from 'refractor/css'
import scss from 'refractor/scss'
import diffLang from 'refractor/diff'

// ---------------------------------------------------------------------------
// refractor 注册（模块级 once）
// ---------------------------------------------------------------------------

let _languagesRegistered = false
function ensureLanguages() {
  if (_languagesRegistered) return
  _languagesRegistered = true
  for (const lang of [
    bash,
    shell,
    javascript,
    typescript,
    jsx,
    tsx,
    python,
    rust,
    go,
    cLang,
    cpp,
    csharp,
    java,
    ruby,
    sql,
    json,
    yaml,
    toml,
    markdownLang,
    html,
    css,
    scss,
    diffLang,
  ]) {
    try {
      refractor.register(lang)
    } catch {
      /* 重复注册忽略 */
    }
  }
}

// ---------------------------------------------------------------------------
// hast → 平铺 token
// ---------------------------------------------------------------------------

interface FlatToken {
  text: string
  className: string[]
}

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

function tokenize(lang: string, text: string): FlatToken[] {
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
// LRU 缓存
// ---------------------------------------------------------------------------

const CACHE_LIMIT = 256

class TokenCache {
  private map = new Map<string, FlatToken[]>()
  get(key: string): FlatToken[] | undefined {
    const v = this.map.get(key)
    if (v) {
      this.map.delete(key)
      this.map.set(key, v)
    }
    return v
  }
  set(key: string, val: FlatToken[]) {
    if (this.map.has(key)) this.map.delete(key)
    this.map.set(key, val)
    if (this.map.size > CACHE_LIMIT) {
      const oldest = this.map.keys().next().value
      if (oldest !== undefined) this.map.delete(oldest)
    }
  }
}

const cache = new TokenCache()

// ---------------------------------------------------------------------------
// 代码块定位 + 高亮装饰构建
// ---------------------------------------------------------------------------

interface CodeBlockInfo {
  /** FencedCode 节点的整体范围（含围栏行） */
  blockFrom: number
  blockTo: number
  /** 代码内容（不含围栏）的范围 */
  contentFrom: number
  contentTo: number
  /** 语言（CodeInfo 节点的 trim 文本） */
  lang: string
}

function findCodeBlocks(view: EditorView): CodeBlockInfo[] {
  ensureLanguages()
  const blocks: CodeBlockInfo[] = []
  const tree = syntaxTree(view.state)
  const doc = view.state.doc

  tree.iterate({
    enter: (node) => {
      if (node.name !== 'FencedCode') return undefined
      // 找子节点：CodeInfo / CodeMark / CodeText / FencedCode 内可能没有 CodeText
      // 我们手动定位"内容范围"：从首行末尾的下一行开始，到最后一行之前
      const blockFrom = node.from
      const blockTo = node.to
      const startLine = doc.lineAt(blockFrom)
      const endLine = doc.lineAt(blockTo)
      // 内容 = 中间所有行
      const contentFrom = startLine.to + 1 // 跳过首行的换行
      const contentTo = endLine.from - 1 // 不含末行前的换行
      // 语言：取首行去掉前导 ``` 后的剩余
      const startText = doc.sliceString(startLine.from, startLine.to)
      const m = /^[`~]{3,}\s*([^\s]+)?/.exec(startText)
      const lang = (m?.[1] ?? '').trim().toLowerCase()
      if (contentFrom <= contentTo && startLine.number !== endLine.number) {
        blocks.push({ blockFrom, blockTo, contentFrom, contentTo, lang })
      }
      return false
    },
  })
  return blocks
}

function buildHighlightDecos(view: EditorView): DecorationSet {
  const supported = new Set(refractor.listLanguages())
  const blocks = findCodeBlocks(view)
  const ranges: Range<Decoration>[] = []

  for (const b of blocks) {
    // 行级装饰：整个 block 加 .cm-md-codeblock-line
    const startLine = view.state.doc.lineAt(b.blockFrom).number
    const endLine = view.state.doc.lineAt(b.blockTo).number
    for (let n = startLine; n <= endLine; n++) {
      const ln = view.state.doc.line(n)
      ranges.push(
        Decoration.line({ class: 'cm-md-codeblock-line' }).range(ln.from),
      )
    }

    // 围栏行（``` 开 / ``` 闭）永久 hide —— 跟其它 marker 一样，
    // 用户删 backtick 让 FencedCode 不再成立时，fence 自动以文本形态回来。
    // 首行同时也带有语言标识符（```rust），整行一起 hide；语言以 ::after 展示在右上角。
    const firstLine = view.state.doc.line(startLine)
    const lastLine = view.state.doc.line(endLine)
    ranges.push(Decoration.replace({}).range(firstLine.from, firstLine.to))
    if (endLine !== startLine) {
      ranges.push(Decoration.replace({}).range(lastLine.from, lastLine.to))
    }

    // 把语言标识符通过 line decoration 的 attributes 透到 DOM 上，
    // 让 CSS ::before 在第一个内容行右上角显示语言 chip
    if (b.lang && endLine - startLine >= 2) {
      const firstContentLine = view.state.doc.line(startLine + 1)
      ranges.push(
        Decoration.line({
          class: 'cm-md-codeblock-line cm-md-codeblock-first',
          attributes: { 'data-lang': b.lang },
        }).range(firstContentLine.from),
      )
    }

    // token 高亮
    if (!b.lang || !supported.has(b.lang)) continue
    const text = view.state.doc.sliceString(b.contentFrom, b.contentTo)
    const key = `${b.lang}:::${text}`
    let toks = cache.get(key)
    if (!toks) {
      toks = tokenize(b.lang, text)
      cache.set(key, toks)
    }
    let pos = b.contentFrom
    for (const tok of toks) {
      const to = pos + tok.text.length
      if (tok.className.length && to > pos) {
        ranges.push(
          Decoration.mark({ class: tok.className.join(' ') }).range(pos, to),
        )
      }
      pos = to
    }
  }

  // RangeSetBuilder 要求严格递增；用 Decoration.set(arr, sort=true)
  return Decoration.set(ranges, true)
}

export const codeHighlight = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet
    constructor(view: EditorView) {
      this.decorations = buildHighlightDecos(view)
    }
    update(u: ViewUpdate) {
      if (u.docChanged || u.viewportChanged) {
        this.decorations = buildHighlightDecos(u.view)
      }
    }
  },
  {
    decorations: (v) => v.decorations,
  },
)
