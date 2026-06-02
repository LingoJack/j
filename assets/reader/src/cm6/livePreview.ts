/**
 * CM6 LivePreview 装饰器引擎（Typora 风：始终渲染）。
 *
 * 设计原则：
 * - **样式由语法树驱动，不由光标位置驱动**。只要 markdown 语法成立
 *   （`**foo**` `# bar` `[a](b)` 这些被 Lezer 识别为对应节点），就始终
 *   渲染样式 + 始终 hide 那些 marker 字符。
 * - 用户删 marker 字符 → Lezer 重解析后那个节点不再成立 → 装饰自动消失，
 *   `**` `#` 这些字符回到普通文本，自然就显示出来了。**这就是 Typora 体验的核心**。
 * - 不主动"暴露源码"。光标穿过 hidden marker 是 ProseMirror/CM6 的自然行为
 *   ——逻辑位置在，物理像素是 0，按 ←/→ 仍然能跨过去。
 *
 * 唯一保留 cursor 感知的特殊情况：fenced code 围栏 ``` 行
 *   —— 用户可能想改语言标识符，所以光标在块内时把 fence 暴露出来。
 */
import { syntaxTree } from '@codemirror/language'
import {
  Decoration,
  type DecorationSet,
  EditorView,
  ViewPlugin,
  type ViewUpdate,
  WidgetType,
} from '@codemirror/view'
import { type EditorState, type Range } from '@codemirror/state'
import { dedupSlugs, slugify } from '../slug'

// ---------------------------------------------------------------------------
// 装饰器定义
// ---------------------------------------------------------------------------

/**
 * 0 宽 widget —— 用 `Decoration.replace({widget: emptyWidget()})` 把
 * marker 字符（# / ** / __ / `> ` / 链接 [] () URL 等）替成"什么也不显示"。
 *
 * 为什么不用 `Decoration.mark({display:none})`？
 * 因为 mark 的两端 inclusive 行为有歧义：当光标点到行首（实际是 marker 之前），
 * 用户开始输入时新字符会插到 marker **之前** —— 比如 `# 标题` 变成 `字# 标题`，
 * 整行就不再是 heading 了。
 *
 * `replace` 装饰的 widget 有明确 `side`：默认 `side=1`（右）意味着光标"穿过"
 * widget 后落在右侧，从用户视角看：点行首 → 光标自动落到 marker 之后的逻辑位置 →
 * 输入新字符插在正确的地方。
 */
class EmptyWidget extends WidgetType {
  toDOM() {
    return document.createElement('span')
  }
  ignoreEvent() {
    return true
  }
  get estimatedHeight() {
    return 0
  }
}
const emptyWidget = new EmptyWidget()

/** marker hide：用 0 宽 widget 替换，避开 mark 装饰的 inclusive 歧义 */
const hideMarker = Decoration.replace({ widget: emptyWidget })

/** 行级 blockquote */
const blockquoteLine = Decoration.line({ class: 'cm-md-blockquote' })

/** 行级 list（普通 / 任务） */
const listLine = Decoration.line({ class: 'cm-md-list-line' })

/** 行级 hr */
const hrLine = Decoration.line({ class: 'cm-md-hr-line' })

/** 行级 hr 内的横线 widget —— 替换 `---` / `***` 整段 */
class HrWidget extends WidgetType {
  toDOM() {
    const el = document.createElement('span')
    el.className = 'cm-md-hr'
    return el
  }
  ignoreEvent() {
    return true
  }
}

/** 列表无序项的圆点 widget —— 替 `- ` / `* ` / `+ ` */
class BulletWidget extends WidgetType {
  toDOM() {
    const el = document.createElement('span')
    el.className = 'cm-md-bullet'
    el.textContent = '•'
    return el
  }
  ignoreEvent() {
    return true
  }
}

/**
 * 任务列表 checkbox widget —— 替 `[ ]` / `[x]`。
 * 真 checkbox：点击切换 → dispatch 修改文档。
 */
class TaskWidget extends WidgetType {
  readonly checked: boolean
  readonly from: number
  readonly to: number
  constructor(checked: boolean, from: number, to: number) {
    super()
    this.checked = checked
    this.from = from
    this.to = to
  }
  eq(other: TaskWidget) {
    return other.checked === this.checked && other.from === this.from
  }
  toDOM(view: EditorView) {
    const el = document.createElement('input')
    el.type = 'checkbox'
    el.checked = this.checked
    el.className = 'cm-md-task'
    el.addEventListener('mousedown', (e) => e.preventDefault()) // 不抢焦点
    el.addEventListener('change', () => {
      const next = this.checked ? '[ ]' : '[x]'
      view.dispatch({
        changes: { from: this.from, to: this.to, insert: next },
      })
    })
    return el
  }
  ignoreEvent(e: Event) {
    return e.type !== 'change' && e.type !== 'click'
  }
}

/** 强调 mark：bold / italic / strikethrough / inline code / link 整体范围 */
const strongMark = Decoration.mark({ class: 'cm-md-strong' })
const emphasisMark = Decoration.mark({ class: 'cm-md-emphasis' })
const strikeMark = Decoration.mark({ class: 'cm-md-strike' })
const inlineCodeMark = Decoration.mark({ class: 'cm-md-inline-code' })
const linkMark = Decoration.mark({ class: 'cm-md-link' })

/**
 * 表格 widget —— 把 GFM 表格源码（| a | b |\n|---|---|\n| 1 | 2 |）
 * 解析成真 `<table>`，块级替换。
 *
 * 光标进入表格范围时不替换 —— 用户改单元格内容时直接编辑源码。
 */
class TableWidget extends WidgetType {
  readonly raw: string
  constructor(raw: string) {
    super()
    this.raw = raw
  }
  eq(other: TableWidget) {
    return other.raw === this.raw
  }
  toDOM() {
    const wrap = document.createElement('div')
    wrap.className = 'cm-md-table-widget'
    const table = document.createElement('table')

    const lines = this.raw.split('\n').filter((l) => l.trim().length > 0)
    if (lines.length < 2) {
      wrap.textContent = this.raw
      return wrap
    }
    // 第二行是分隔线 |---|:--:|... → 用它来推每列对齐
    const sepCells = splitRow(lines[1])
    const aligns = sepCells.map(parseAlignment)

    // 第一行 = 表头
    const thead = document.createElement('thead')
    const trH = document.createElement('tr')
    splitRow(lines[0]).forEach((cell, i) => {
      const th = document.createElement('th')
      th.textContent = cell.trim()
      if (aligns[i]) th.style.textAlign = aligns[i]
      trH.appendChild(th)
    })
    thead.appendChild(trH)
    table.appendChild(thead)

    // 其余行 = 数据
    const tbody = document.createElement('tbody')
    for (let i = 2; i < lines.length; i++) {
      const tr = document.createElement('tr')
      splitRow(lines[i]).forEach((cell, j) => {
        const td = document.createElement('td')
        td.textContent = cell.trim()
        if (aligns[j]) td.style.textAlign = aligns[j]
        tr.appendChild(td)
      })
      tbody.appendChild(tr)
    }
    table.appendChild(tbody)
    wrap.appendChild(table)
    return wrap
  }
  ignoreEvent() {
    // 让点击 / 选中传给 CM6 —— 用户点表格时光标会落进 widget 之后的位置
    return false
  }
}

function splitRow(line: string): string[] {
  // 去掉首尾的 | （如有），按 | 切；忽略转义 `\|`
  let s = line.trim()
  if (s.startsWith('|')) s = s.slice(1)
  if (s.endsWith('|')) s = s.slice(0, -1)
  // 简单切：不处理 \| 转义（GFM 罕见，碰到再补）
  return s.split('|')
}

function parseAlignment(sep: string): '' | 'left' | 'right' | 'center' {
  const t = sep.trim()
  const left = t.startsWith(':')
  const right = t.endsWith(':')
  if (left && right) return 'center'
  if (right) return 'right'
  if (left) return 'left'
  return ''
}

/** 数前导反引号长度（最多 4 个） */
function countLeadingBackticks(s: string): number {
  let n = 0
  while (n < s.length && s[n] === '`') n++
  return n
}
function countTrailingBackticks(s: string): number {
  let n = 0
  while (n < s.length && s[s.length - 1 - n] === '`') n++
  return n
}

// ---------------------------------------------------------------------------
// LivePreview ViewPlugin
// ---------------------------------------------------------------------------

interface DecoEntry {
  from: number
  to: number
  deco: Decoration
}

function sortDecos(entries: DecoEntry[]): DecoEntry[] {
  return entries.slice().sort((a, b) => {
    if (a.from !== b.from) return a.from - b.from
    const sa = a.deco.spec?.class ? 1 : 0
    const sb = b.deco.spec?.class ? 1 : 0
    return sa - sb
  })
}

/**
 * fenced code 围栏行（``` 开 / 闭）的范围列表 ——
 * 唯一对光标位置敏感的元素：光标在该代码块内时不 hide / 不 widget；
 * 否则 fence 行原封不动（不显示成 widget，因为代码块靠 codeHighlight.ts
 * 给 .cm-md-codeblock-line 行级装饰，fence 行的语言信息自然会显示）。
 */
function fencedCodeRanges(view: EditorView): Array<{ from: number; to: number }> {
  const out: Array<{ from: number; to: number }> = []
  syntaxTree(view.state).iterate({
    enter: (node) => {
      if (node.name === 'FencedCode') {
        out.push({ from: node.from, to: node.to })
        return false
      }
      return undefined
    },
  })
  return out
}

function buildDecorations(view: EditorView): DecorationSet {
  const entries: DecoEntry[] = []

  // 收集 fenced code 范围 —— 落在范围内的 marker 节点都不动
  const codeRanges = fencedCodeRanges(view)
  const inCodeBlock = (from: number, to: number) => {
    for (const r of codeRanges) {
      if (from >= r.from && to <= r.to) return true
    }
    return false
  }

  /**
   * 范围相交判断 —— 仅 InlineCode 用。
   * slack=1：光标贴在反引号外侧（前/后一字符）也算"在 code 里"，方便用 ←/→
   * 控制是否还在 code 范围。
   */
  const intersectsCursor = (from: number, to: number, slack = 0): boolean => {
    for (const r of view.state.selection.ranges) {
      if (r.from <= to + slack && r.to >= from - slack) return true
    }
    return false
  }

  // 全文扫描所有 heading，生成稳定的 slug 列表（TOC 锚点）
  const headingSlugs = computeHeadingSlugs(view)
  let headingIdx = 0

  for (const { from, to } of view.visibleRanges) {
    syntaxTree(view.state).iterate({
      from,
      to,
      enter: (node) => {
        const name = node.name
        const nodeFrom = node.from
        const nodeTo = node.to

        // ── 行级：heading ──────────────────────────────────────────
        if (/^ATXHeading[1-6]$/.test(name)) {
          const level = Number(name.slice(-1))
          const line = view.state.doc.lineAt(nodeFrom)
          const slug = headingSlugs[headingIdx] ?? ''
          headingIdx++
          entries.push({
            from: line.from,
            to: line.from,
            deco: Decoration.line({
              class: `cm-md-heading cm-md-h${level}`,
              attributes: slug ? { id: slug } : undefined,
            }),
          })
        }
        // ── 行级：blockquote ───────────────────────────────────────
        else if (name === 'Blockquote') {
          const startLine = view.state.doc.lineAt(nodeFrom).number
          const endLine = view.state.doc.lineAt(nodeTo).number
          for (let n = startLine; n <= endLine; n++) {
            const ln = view.state.doc.line(n)
            entries.push({
              from: ln.from,
              to: ln.from,
              deco: blockquoteLine,
            })
          }
        }
        // ── 行级：list item ────────────────────────────────────────
        else if (name === 'ListItem') {
          const line = view.state.doc.lineAt(nodeFrom)
          entries.push({
            from: line.from,
            to: line.from,
            deco: listLine,
          })
        }
        // ── 行级：hr ───────────────────────────────────────────────
        else if (name === 'HorizontalRule') {
          const line = view.state.doc.lineAt(nodeFrom)
          entries.push({ from: line.from, to: line.from, deco: hrLine })
          // 永久 widget 替换 `---` / `***`：横线视觉上一直在
          entries.push({
            from: nodeFrom,
            to: nodeTo,
            deco: Decoration.replace({ widget: new HrWidget() }),
          })
        }
        // ── 内联 marker 永久 hide（用 0 宽 widget 替换，避开 mark 边界歧义） ──
        else if (name === 'HeaderMark') {
          if (inCodeBlock(nodeFrom, nodeTo)) return undefined
          // # / ## / ### + 紧跟的一个空格全部 hide
          const text = view.state.doc.sliceString(nodeTo, nodeTo + 1)
          const trail = text === ' ' ? 1 : 0
          entries.push({
            from: nodeFrom,
            to: nodeTo + trail,
            deco: hideMarker,
          })
        } else if (
          name === 'EmphasisMark' ||
          name === 'StrikethroughMark'
        ) {
          if (inCodeBlock(nodeFrom, nodeTo)) return undefined
          entries.push({ from: nodeFrom, to: nodeTo, deco: hideMarker })
        } else if (name === 'CodeMark') {
          if (inCodeBlock(nodeFrom, nodeTo)) return undefined
          // CodeMark 在 InlineCode 内是反引号；FencedCode 内是围栏。
          // FencedCode 已被 inCodeBlock 过滤掉。这里只剩 InlineCode 的反引号。
          // ★ 由父 InlineCode 节点统一决定是否 hide（光标 intersect 时不 hide）
          // —— 不在这里处理，而是通过 InlineCode 节点把整段（含两个 CodeMark）
          // 一并接管。这里直接返回，跳过默认处理。
          return undefined
        } else if (name === 'QuoteMark') {
          // > + 可选空格全部 hide
          const next = view.state.doc.sliceString(nodeTo, nodeTo + 1)
          const trail = next === ' ' ? 1 : 0
          entries.push({
            from: nodeFrom,
            to: nodeTo + trail,
            deco: hideMarker,
          })
        } else if (name === 'ListMark') {
          // 无序列表 `- ` / `* ` / `+ ` 永久替成圆点 widget；
          // 有序列表 `1.` 保留数字
          const raw = view.state.doc.sliceString(nodeFrom, nodeTo)
          if (/^[-*+]$/.test(raw)) {
            const next = view.state.doc.sliceString(nodeTo, nodeTo + 1)
            const trailing = next === ' ' ? 1 : 0
            entries.push({
              from: nodeFrom,
              to: nodeTo + trailing,
              deco: Decoration.replace({ widget: new BulletWidget() }),
            })
          }
        } else if (name === 'TaskMarker') {
          const raw = view.state.doc.sliceString(nodeFrom, nodeTo)
          const checked = /\[[xX]\]/.test(raw)
          entries.push({
            from: nodeFrom,
            to: nodeTo,
            deco: Decoration.replace({
              widget: new TaskWidget(checked, nodeFrom, nodeTo),
            }),
          })
        }
        // ── 强调样式范围 mark ──────────────────────────────────────
        else if (name === 'StrongEmphasis') {
          entries.push({ from: nodeFrom, to: nodeTo, deco: strongMark })
        } else if (name === 'Emphasis') {
          entries.push({ from: nodeFrom, to: nodeTo, deco: emphasisMark })
        } else if (name === 'Strikethrough') {
          entries.push({ from: nodeFrom, to: nodeTo, deco: strikeMark })
        } else if (name === 'InlineCode') {
          // 整段加 inline-code 样式（mark）
          entries.push({ from: nodeFrom, to: nodeTo, deco: inlineCodeMark })
          // 反引号 marker：光标"在范围内或紧邻边界"时**不 hide**，让用户看到 ` ` `，
          // 用 ←/→ 走出反引号即可让新输入的字不再属于 inline code。
          // slack=1 让"贴在反引号外一格"也算 inside —— 给用户一个安全边界。
          if (!intersectsCursor(nodeFrom, nodeTo, 1)) {
            // 反引号在 InlineCode 节点的两端，长度通常各 1（也可能 ``code`` 时各 2）。
            // 用文本扫一下两端连续反引号的长度。
            const startTicks = countLeadingBackticks(
              view.state.doc.sliceString(nodeFrom, nodeFrom + 4),
            )
            const endTicks = countTrailingBackticks(
              view.state.doc.sliceString(nodeTo - 4, nodeTo),
            )
            if (startTicks > 0) {
              entries.push({
                from: nodeFrom,
                to: nodeFrom + startTicks,
                deco: hideMarker,
              })
            }
            if (endTicks > 0) {
              entries.push({
                from: nodeTo - endTicks,
                to: nodeTo,
                deco: hideMarker,
              })
            }
          }
          // 跳过子节点 —— 否则 CodeMark 会被外层默认逻辑误处理
          return false
        } else if (name === 'Link') {
          // [text](url)：整段加 link 样式；下面的 LinkMark / URL / LinkTitle
          // 子节点会被分别 hide
          entries.push({ from: nodeFrom, to: nodeTo, deco: linkMark })
        } else if (
          name === 'LinkMark' ||
          name === 'URL' ||
          name === 'LinkTitle'
        ) {
          // [ ] ( ) URL 全部永久 hide
          entries.push({ from: nodeFrom, to: nodeTo, deco: hideMarker })
        }
        // ── 表格：整段替成真 <table> widget ─────────────────────────
        // 注意：Table 是块级 widget，块级装饰必须由 facet provider 给出
        // （CM6 不允许 ViewPlugin 提供 block decoration），所以这里不处理 ——
        // Table 的 widget 由下面 `tableBlockDecorations` 抽到独立 extension。
        else if (name === 'Table') {
          // 让外层迭代器跳过 Table 子节点（避免里面的 EmphasisMark 被误处理）
          return false
        }
        return undefined
      },
    })
  }

  const sorted = sortDecos(entries)
  const ranges: Range<Decoration>[] = sorted.map((e) =>
    e.deco.range(e.from, e.to),
  )
  return Decoration.set(ranges, true)
}

/**
 * 主 ViewPlugin。
 *
 * 监听：
 * - `docChanged` —— 文档变了重算（语法树会跟进）
 * - `viewportChanged` —— 滚动到新区域时给新可见行装饰
 * - `selectionSet` —— **只为 InlineCode 服务**：光标进入 inline code 时
 *   暴露反引号让用户能编辑；离开时再 hide。其它 marker 都是永久 hide。
 *   （Table 也有"光标进入暴露源码"语义，但其 block widget 走下面独立的
 *   `tableBlockDecorations` extension —— CM6 不允许 ViewPlugin 提供 block 装饰。）
 */
const livePreviewPlugin = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet
    constructor(view: EditorView) {
      this.decorations = buildDecorations(view)
    }
    update(u: ViewUpdate) {
      if (u.docChanged || u.viewportChanged || u.selectionSet) {
        this.decorations = buildDecorations(u.view)
      }
    }
  },
  {
    decorations: (v) => v.decorations,
  },
)

// ---------------------------------------------------------------------------
// 表格 block widget —— 独立 extension
// ---------------------------------------------------------------------------
//
// 为什么单独抽：CM6 硬性约束 `Decoration.replace({block: true, ...})` 必须
// 通过 `EditorView.decorations.compute(...)`（facet provider，等价于 StateField）
// 提供。从 ViewPlugin 暴露 block 装饰会抛
// "Block decorations may not be specified via plugins"。README 里的
// `| 命令 | 说明 |` 表格就会触发。
//
// facet provider 拿不到 view，只能拿 state；意味着 selectionSet 也走 'selection'
// 依赖。每次重算都要扫全文 syntax tree 找 Table 节点 —— 但 Lezer 自带缓存，
// 在文档不变时 iterate 走的是已缓存的 tree，成本可接受。

function buildTableBlockDecos(state: EditorState): DecorationSet {
  const ranges: Range<Decoration>[] = []
  const intersectsCursor = (from: number, to: number) => {
    for (const r of state.selection.ranges) {
      if (r.from <= to && r.to >= from) return true
    }
    return false
  }
  syntaxTree(state).iterate({
    enter: (node) => {
      if (node.name !== 'Table') return undefined
      // 光标进入表格范围时不 widget —— 用户改单元格内容时直接编辑源码
      if (intersectsCursor(node.from, node.to)) return false
      const raw = state.doc.sliceString(node.from, node.to)
      ranges.push(
        Decoration.replace({
          widget: new TableWidget(raw),
          block: true,
        }).range(node.from, node.to),
      )
      return false
    },
  })
  return Decoration.set(ranges, true)
}

const tableBlockDecorations = EditorView.decorations.compute(
  ['doc', 'selection'],
  (state) => buildTableBlockDecos(state),
)

/**
 * 对外导出：把内部 ViewPlugin（inline 装饰）和独立 facet provider（block 表格 widget）
 * 一起作为单个 extension 暴露 —— 调用方仍像以前一样写 `livePreview`。
 */
export const livePreview = [livePreviewPlugin, tableBlockDecorations]

/**
 * 全文扫描所有 heading，返回按出现顺序的去重 slug 列表。
 *
 * Lezer iterate 没解析过的范围会按需扩展，但相对完整文档解析仍很便宜：
 * 解析结果内部有缓存 + 视图重排时才重跑。
 */
function computeHeadingSlugs(view: EditorView): string[] {
  const slugs: string[] = []
  syntaxTree(view.state).iterate({
    enter: (node) => {
      if (/^ATXHeading[1-6]$/.test(node.name)) {
        const raw = view.state.doc.sliceString(node.from, node.to)
        const text = raw.replace(/^#+\s*/, '').trimEnd()
        slugs.push(slugify(text))
        return false
      }
      return undefined
    },
  })
  return dedupSlugs(slugs)
}
