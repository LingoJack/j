/**
 * 文本 Diff 工具。
 *
 * 左右两个 textarea 输入两份文本，下方并排展示行级 diff：
 *   - 删除（- 红）   仅出现在左侧
 *   - 新增（+ 绿）   仅出现在右侧
 *   - 不变（空 灰）  两侧都有
 *
 * 算法：经典 LCS（Longest Common Subsequence）的 O(N*M) DP，再回溯出
 * 编辑脚本。对几千行级别的 diff 足够流畅；上万行才会肉眼感觉到。
 *
 * 设计考量：
 * - 不接 /api，纯前端，刷页会丢内容 —— 用 sessionStorage 兜一下，
 *   切到别的 tab 再回来不会清空。
 * - "对照查看"复用同一份滚动容器，让左右两侧行号严格对齐（用占位行
 *   补齐编辑脚本里"另一侧没有的"那行）。
 */
import { useEffect, useMemo, useState } from 'react'

type Op = 'eq' | 'add' | 'del'

interface Row {
  op: Op
  /** 该 op 在左/右两侧分别对应哪行（1-based）；不存在则 null */
  leftLine: number | null
  rightLine: number | null
  leftText: string
  rightText: string
}

const STORAGE_LEFT = 'jreader.tool.diff.left'
const STORAGE_RIGHT = 'jreader.tool.diff.right'

/** 输入超过这个行数时，提示"已截断"避免 O(N²) DP 卡死浏览器 */
const MAX_LINES = 4000

export function DiffTool() {
  const [left, setLeft] = useState<string>(
    () => sessionStorage.getItem(STORAGE_LEFT) ?? '',
  )
  const [right, setRight] = useState<string>(
    () => sessionStorage.getItem(STORAGE_RIGHT) ?? '',
  )

  useEffect(() => {
    sessionStorage.setItem(STORAGE_LEFT, left)
  }, [left])
  useEffect(() => {
    sessionStorage.setItem(STORAGE_RIGHT, right)
  }, [right])

  const { rows, addCount, delCount, truncated } = useMemo(
    () => computeDiff(left, right),
    [left, right],
  )

  const swap = () => {
    setLeft(right)
    setRight(left)
  }
  const clear = () => {
    setLeft('')
    setRight('')
  }

  return (
    <div className="seeyue-diff-tool">
      <div className="seeyue-diff-toolbar">
        <span className="title">文本 Diff</span>
        <span className="stat" data-tone="del">
          - {delCount}
        </span>
        <span className="stat" data-tone="add">
          + {addCount}
        </span>
        {truncated && (
          <span className="stat" data-tone="warn" title={`输入超过 ${MAX_LINES} 行，已截断`}>
            ⚠ 已截断
          </span>
        )}
        <div className="flex-1" />
        <button type="button" className="seeyue-btn" onClick={swap} title="交换左右两边">
          ⇄ 交换
        </button>
        <button type="button" className="seeyue-btn" onClick={clear} title="清空两边">
          清空
        </button>
      </div>

      <div className="seeyue-diff-inputs">
        <div className="pane">
          <div className="pane-head">
            <span className="dot" data-tone="del" />
            原始文本（A）
          </div>
          <textarea
            className="seeyue-textarea"
            value={left}
            spellCheck={false}
            placeholder="粘贴原始文本…"
            onChange={(e) => setLeft(e.target.value)}
          />
        </div>
        <div className="pane">
          <div className="pane-head">
            <span className="dot" data-tone="add" />
            修改后文本（B）
          </div>
          <textarea
            className="seeyue-textarea"
            value={right}
            spellCheck={false}
            placeholder="粘贴新版本文本…"
            onChange={(e) => setRight(e.target.value)}
          />
        </div>
      </div>

      <div className="seeyue-diff-result">
        <div className="result-head">差异对照</div>
        {rows.length === 0 ? (
          <div className="result-empty">两边都为空</div>
        ) : addCount === 0 && delCount === 0 ? (
          <div className="result-empty result-equal">✓ 两段文本完全相同</div>
        ) : (
          <div className="result-grid">
            {rows.map((r, i) => (
              <DiffRow key={i} row={r} />
            ))}
          </div>
        )}
      </div>
    </div>
  )
}

function DiffRow({ row }: { row: Row }) {
  const { op, leftLine, rightLine, leftText, rightText } = row
  return (
    <div className="diff-row" data-op={op}>
      <div className="cell" data-side="left" data-op={op === 'add' ? 'pad' : op}>
        <span className="lineno">{leftLine ?? ''}</span>
        <span className="marker">{op === 'del' ? '-' : op === 'eq' ? ' ' : ''}</span>
        <pre className="text">{leftText}</pre>
      </div>
      <div className="cell" data-side="right" data-op={op === 'del' ? 'pad' : op}>
        <span className="lineno">{rightLine ?? ''}</span>
        <span className="marker">{op === 'add' ? '+' : op === 'eq' ? ' ' : ''}</span>
        <pre className="text">{rightText}</pre>
      </div>
    </div>
  )
}

// ---------------------------------------------------------------------------
// LCS-based 行级 diff
// ---------------------------------------------------------------------------

interface DiffResult {
  rows: Row[]
  addCount: number
  delCount: number
  truncated: boolean
}

function computeDiff(leftText: string, rightText: string): DiffResult {
  let a = splitLines(leftText)
  let b = splitLines(rightText)
  let truncated = false
  if (a.length > MAX_LINES) {
    a = a.slice(0, MAX_LINES)
    truncated = true
  }
  if (b.length > MAX_LINES) {
    b = b.slice(0, MAX_LINES)
    truncated = true
  }

  // dp[i][j] = LCS(a[0..i], b[0..j])
  // 用 Uint32Array 节省内存；i*(b.len+1)+j 索引
  const m = a.length
  const n = b.length
  const w = n + 1
  const dp = new Uint32Array((m + 1) * w)
  for (let i = m - 1; i >= 0; i--) {
    for (let j = n - 1; j >= 0; j--) {
      if (a[i] === b[j]) {
        dp[i * w + j] = dp[(i + 1) * w + j + 1] + 1
      } else {
        const x = dp[(i + 1) * w + j]
        const y = dp[i * w + j + 1]
        dp[i * w + j] = x > y ? x : y
      }
    }
  }

  // 回溯生成编辑脚本
  const rows: Row[] = []
  let i = 0
  let j = 0
  let addCount = 0
  let delCount = 0
  while (i < m && j < n) {
    if (a[i] === b[j]) {
      rows.push({
        op: 'eq',
        leftLine: i + 1,
        rightLine: j + 1,
        leftText: a[i],
        rightText: b[j],
      })
      i++
      j++
    } else if (dp[(i + 1) * w + j] >= dp[i * w + j + 1]) {
      rows.push({
        op: 'del',
        leftLine: i + 1,
        rightLine: null,
        leftText: a[i],
        rightText: '',
      })
      delCount++
      i++
    } else {
      rows.push({
        op: 'add',
        leftLine: null,
        rightLine: j + 1,
        leftText: '',
        rightText: b[j],
      })
      addCount++
      j++
    }
  }
  while (i < m) {
    rows.push({
      op: 'del',
      leftLine: i + 1,
      rightLine: null,
      leftText: a[i],
      rightText: '',
    })
    delCount++
    i++
  }
  while (j < n) {
    rows.push({
      op: 'add',
      leftLine: null,
      rightLine: j + 1,
      leftText: '',
      rightText: b[j],
    })
    addCount++
    j++
  }

  return { rows, addCount, delCount, truncated }
}

/** 按 \n 切；保留尾部空行的语义（"abc\n" 切成 ["abc"]，"abc\n\n" 切成 ["abc", ""]） */
function splitLines(s: string): string[] {
  if (!s) return []
  // 统一 \r\n / \r → \n
  const norm = s.replace(/\r\n?/g, '\n')
  const parts = norm.split('\n')
  // 去掉最末的空行（split 总会多一个）—— 但只有当原文以 \n 结束时才去
  if (parts.length > 0 && parts[parts.length - 1] === '' && norm.endsWith('\n')) {
    parts.pop()
  }
  return parts
}
