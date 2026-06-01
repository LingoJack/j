/**
 * 在保存 markdown 时，把 Milkdown 序列化压扁的"块间空行"按 initial 文本恢复。
 *
 * 背景：CommonMark AST 不记录"段间几个空行" —— 任意正数空行都被视作同一种块边界。
 * 所以一旦经过 ProseMirror（Milkdown）一来一回，原文件里手工排版的多余空行就被压成
 * 1 个。如果用户只改了文档中段的一个字，文档其他位置的空行布局也会被全量重写。
 *
 * 这里的策略是"忠实原始内容"：
 *   1. 把 initial / current 都按"块"切（块 = 连续非空行；fenced code 内部不切）
 *   2. 用块的 normalized content 做 key，建 initial 的"此块之后空行数"队列
 *   3. 重建 current：每段之间的空行数从该队列里按出现顺序取
 *   4. 队列空了（即用户新增的块、或被改得 normalized 也对不上的块）→ 退回 1 个空行
 *   5. 文件首部空行 / 末尾换行数都沿用 initial
 *
 * 这不是完美方案：用户改过的块自然走 fallback。但绝大多数典型场景（打开看几眼、
 * 改一个标题，然后保存）能保住排版。
 */

type Token =
  | { kind: 'block'; lines: string[] }
  | { kind: 'blank'; count: number }

interface Tokenized {
  tokens: Token[]
  /** 文件末尾紧跟的 `\n` 数（不计入 tokens，重建时单独补） */
  trailingNewlines: number
}

export function preserveBlankLines(initial: string, current: string): string {
  if (initial === current) return current
  if (initial.length === 0) return current

  const orig = tokenize(initial)
  const cur = tokenize(current)

  // 文件首部 leading 空行（initial 视角）
  const origLeading =
    orig.tokens[0]?.kind === 'blank' ? orig.tokens[0].count : 0

  // 给每个 orig 的 block 建一个"该 block 后的空行数"队列；同 content 出现 N 次按顺序入队
  const gapAfterMap = new Map<string, number[]>()
  for (let i = 0; i < orig.tokens.length; i++) {
    const tk = orig.tokens[i]
    if (tk.kind !== 'block') continue
    const next = orig.tokens[i + 1]
    const after = next && next.kind === 'blank' ? next.count : 0
    const key = blockKey(tk.lines)
    let arr = gapAfterMap.get(key)
    if (!arr) {
      arr = []
      gapAfterMap.set(key, arr)
    }
    arr.push(after)
  }

  // 重建 cur 的 token 序列
  const out: Token[] = []
  let prevBlockKey: string | null = null

  for (let i = 0; i < cur.tokens.length; i++) {
    const tk = cur.tokens[i]
    if (tk.kind === 'block') {
      out.push(tk)
      prevBlockKey = blockKey(tk.lines)
      continue
    }
    // blank
    if (prevBlockKey === null) {
      // cur 文件开头的 blank：用 initial 的 leading（即使 initial 没 leading 也保留 cur 的）
      out.push({ kind: 'blank', count: origLeading > 0 ? origLeading : tk.count })
      continue
    }
    const arr = gapAfterMap.get(prevBlockKey)
    let count = tk.count
    if (arr && arr.length > 0) {
      count = arr.shift()!
    }
    // 块间至少留 1 个空行（CommonMark 块边界）；
    // arr 命中 0 的情况只在 orig 中两个 block 紧贴出现，复原后让 cur 也紧贴并不会改变 AST
    if (count < 1) count = 1
    out.push({ kind: 'blank', count })
  }

  // 如果 cur 不以 blank 开头但 initial 有 leading，补回 leading
  if (origLeading > 0 && (out.length === 0 || out[0].kind === 'block')) {
    out.unshift({ kind: 'blank', count: origLeading })
  }

  return reconstruct(out, orig.trailingNewlines)
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

function tokenize(src: string): Tokenized {
  // 把末尾连续 \n 单独提走，便于 reconstruct 时精确还原
  const trailingMatch = src.match(/\n*$/)
  const trailingNewlines = trailingMatch ? trailingMatch[0].length : 0
  const body = src.slice(0, src.length - trailingNewlines)
  const lines = body.length > 0 ? body.split('\n') : []

  const tokens: Token[] = []
  let curBlock: string[] | null = null
  let blankCount = 0
  let inFence = false
  let fenceTick: string | null = null

  const flushBlock = () => {
    if (curBlock) {
      tokens.push({ kind: 'block', lines: curBlock })
      curBlock = null
    }
  }
  const flushBlank = () => {
    if (blankCount > 0) {
      tokens.push({ kind: 'blank', count: blankCount })
      blankCount = 0
    }
  }

  for (const line of lines) {
    if (inFence) {
      // 围栏内不做任何切分，包括空行
      curBlock!.push(line)
      const close = line.match(/^\s*(`{3,}|~{3,})\s*$/)
      if (
        close &&
        fenceTick &&
        close[1][0] === fenceTick[0] &&
        close[1].length >= fenceTick.length
      ) {
        inFence = false
        fenceTick = null
      }
      continue
    }

    const open = line.match(/^\s*(`{3,}|~{3,})/)
    if (open) {
      flushBlank()
      flushBlock()
      curBlock = [line]
      inFence = true
      fenceTick = open[1]
      continue
    }

    if (line.trim() === '') {
      flushBlock()
      blankCount++
    } else {
      flushBlank()
      if (!curBlock) curBlock = []
      curBlock.push(line)
    }
  }
  flushBlock()
  flushBlank()

  return { tokens, trailingNewlines }
}

/**
 * 块身份键：忽略行尾空格 + trim 整体首尾空白。这样 Milkdown 把行尾多余空格抹掉
 * 也不会让我们对不齐 orig。
 */
function blockKey(lines: string[]): string {
  return lines
    .map((l) => l.replace(/[ \t]+$/, ''))
    .join('\n')
    .trim()
}

function reconstruct(tokens: Token[], trailingNewlines: number): string {
  const lines: string[] = []
  for (const tk of tokens) {
    if (tk.kind === 'block') {
      for (const l of tk.lines) lines.push(l)
    } else {
      for (let i = 0; i < tk.count; i++) lines.push('')
    }
  }
  // join('\n') 不会在末尾加 \n；按 trailingNewlines 显式补
  return lines.join('\n') + '\n'.repeat(trailingNewlines)
}
