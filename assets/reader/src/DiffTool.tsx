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
    <div className="h-full flex flex-col bg-seeyue-bg">
      <div className="flex items-center gap-2.5 px-3.5 py-2 border-b border-seeyue-border bg-seeyue-sidebar-strong text-xs">
        <span className="text-[13px] font-medium text-seeyue-fg-strong">文本 Diff</span>
        <span className="font-mono text-[11.5px] px-2 py-0.5 rounded-full bg-seeyue-elevated text-seeyue-fg-muted data-[tone=add]:text-seeyue-success data-[tone=add]:bg-[rgba(163,190,140,0.14)] data-[tone=del]:text-seeyue-danger data-[tone=del]:bg-[rgba(191,97,106,0.16)] data-[tone=warn]:text-seeyue-warn data-[tone=warn]:bg-[rgba(208,135,112,0.18)] data-[tone=accent]:text-seeyue-accent data-[tone=accent]:bg-seeyue-accent-mute" data-tone="del">
          - {delCount}
        </span>
        <span className="font-mono text-[11.5px] px-2 py-0.5 rounded-full bg-seeyue-elevated text-seeyue-fg-muted data-[tone=add]:text-seeyue-success data-[tone=add]:bg-[rgba(163,190,140,0.14)] data-[tone=del]:text-seeyue-danger data-[tone=del]:bg-[rgba(191,97,106,0.16)] data-[tone=warn]:text-seeyue-warn data-[tone=warn]:bg-[rgba(208,135,112,0.18)] data-[tone=accent]:text-seeyue-accent data-[tone=accent]:bg-seeyue-accent-mute" data-tone="add">
          + {addCount}
        </span>
        {truncated && (
          <span className="font-mono text-[11.5px] px-2 py-0.5 rounded-full bg-seeyue-elevated text-seeyue-fg-muted data-[tone=add]:text-seeyue-success data-[tone=add]:bg-[rgba(163,190,140,0.14)] data-[tone=del]:text-seeyue-danger data-[tone=del]:bg-[rgba(191,97,106,0.16)] data-[tone=warn]:text-seeyue-warn data-[tone=warn]:bg-[rgba(208,135,112,0.18)] data-[tone=accent]:text-seeyue-accent data-[tone=accent]:bg-seeyue-accent-mute" data-tone="warn" title={`输入超过 ${MAX_LINES} 行，已截断`}>
            ⚠ 已截断
          </span>
        )}
        <div className="flex-1" />
        <button type="button" className="px-3.5 py-1.5 text-[12.5px] font-cjk rounded-md border border-transparent cursor-pointer bg-transparent text-seeyue-fg-muted transition-all duration-150 hover:text-seeyue-fg-strong hover:bg-seeyue-elevated data-[tone=primary]:bg-seeyue-accent-strong data-[tone=primary]:text-seeyue-fg-strong data-[tone=primary]:hover:bg-seeyue-accent data-[tone=primary]:hover:text-seeyue-bg data-[tone=danger]:text-seeyue-danger data-[tone=danger]:hover:bg-[rgba(191,97,106,0.18)]" onClick={swap} title="交换左右两边">
          ⇄ 交换
        </button>
        <button type="button" className="px-3.5 py-1.5 text-[12.5px] font-cjk rounded-md border border-transparent cursor-pointer bg-transparent text-seeyue-fg-muted transition-all duration-150 hover:text-seeyue-fg-strong hover:bg-seeyue-elevated data-[tone=primary]:bg-seeyue-accent-strong data-[tone=primary]:text-seeyue-fg-strong data-[tone=primary]:hover:bg-seeyue-accent data-[tone=primary]:hover:text-seeyue-bg data-[tone=danger]:text-seeyue-danger data-[tone=danger]:hover:bg-[rgba(191,97,106,0.18)]" onClick={clear} title="清空两边">
          清空
        </button>
      </div>

      <div className="grid grid-cols-2 gap-px bg-seeyue-border border-b border-seeyue-border flex-[0_0_240px] min-h-[180px]">
        <div className="flex flex-col bg-seeyue-bg min-w-0">
          <div className="flex items-center gap-1.5 px-3 py-1.5 text-[11.5px] text-seeyue-fg-muted border-b border-seeyue-border bg-seeyue-sidebar">
            <span className="w-2 h-2 rounded-full data-[tone=add]:bg-seeyue-success data-[tone=del]:bg-seeyue-danger" data-tone="del" />
            原始文本（A）
          </div>
          <textarea
            className="seeyue-textarea flex-1 w-full px-2.5 py-2.5 text-[13px] leading-[1.6]"
            value={left}
            spellCheck={false}
            placeholder="粘贴原始文本…"
            onChange={(e) => setLeft(e.target.value)}
          />
        </div>
        <div className="flex flex-col bg-seeyue-bg min-w-0">
          <div className="flex items-center gap-1.5 px-3 py-1.5 text-[11.5px] text-seeyue-fg-muted border-b border-seeyue-border bg-seeyue-sidebar">
            <span className="w-2 h-2 rounded-full data-[tone=add]:bg-seeyue-success data-[tone=del]:bg-seeyue-danger" data-tone="add" />
            修改后文本（B）
          </div>
          <textarea
            className="seeyue-textarea flex-1 w-full px-2.5 py-2.5 text-[13px] leading-[1.6]"
            value={right}
            spellCheck={false}
            placeholder="粘贴新版本文本…"
            onChange={(e) => setRight(e.target.value)}
          />
        </div>
      </div>

      <div className="flex-1 flex flex-col min-h-0 overflow-hidden">
        <div className="px-3.5 py-1.5 text-[11.5px] text-seeyue-fg-muted bg-seeyue-sidebar border-b border-seeyue-border tracking-[0.04em]">差异对照</div>
        {rows.length === 0 ? (
          <div className="flex-1 flex items-center justify-center text-seeyue-fg-dim text-[13px]">两边都为空</div>
        ) : addCount === 0 && delCount === 0 ? (
          <div className="flex-1 flex items-center justify-center text-seeyue-success text-[13px]">✓ 两段文本完全相同</div>
        ) : (
          <div className="flex-1 overflow-auto font-mono text-[12.5px] leading-[1.55]">
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
    <div className="grid grid-cols-2 border-b border-[rgba(76,86,106,0.12)] hover:bg-[rgba(94,129,172,0.04)]" data-op={op}>
      <div className="group grid grid-cols-[48px_14px_1fr] items-start py-px border-r border-seeyue-border min-w-0 last:border-r-0 data-[op=del]:bg-[rgba(191,97,106,0.14)] data-[op=add]:bg-[rgba(163,190,140,0.14)] data-[op=pad]:bg-[rgba(76,86,106,0.18)]" data-side="left" data-op={op === 'add' ? 'pad' : op}>
        <span className="text-right px-2 text-seeyue-fg-dim text-[11px] select-none">{leftLine ?? ''}</span>
        <span className="text-center text-seeyue-fg-dim select-none group-data-[op=del]:text-seeyue-danger group-data-[op=del]:font-semibold group-data-[op=add]:text-seeyue-success group-data-[op=add]:font-semibold">{op === 'del' ? '-' : op === 'eq' ? ' ' : ''}</span>
        <pre className="m-0 pr-2 font-mono text-[12.5px] leading-[1.55] whitespace-pre-wrap break-all text-seeyue-fg bg-transparent group-data-[op=pad]:text-seeyue-fg-dim">{leftText}</pre>
      </div>
      <div className="group grid grid-cols-[48px_14px_1fr] items-start py-px border-r border-seeyue-border min-w-0 last:border-r-0 data-[op=del]:bg-[rgba(191,97,106,0.14)] data-[op=add]:bg-[rgba(163,190,140,0.14)] data-[op=pad]:bg-[rgba(76,86,106,0.18)]" data-side="right" data-op={op === 'del' ? 'pad' : op}>
        <span className="text-right px-2 text-seeyue-fg-dim text-[11px] select-none">{rightLine ?? ''}</span>
        <span className="text-center text-seeyue-fg-dim select-none group-data-[op=del]:text-seeyue-danger group-data-[op=del]:font-semibold group-data-[op=add]:text-seeyue-success group-data-[op=add]:font-semibold">{op === 'add' ? '+' : op === 'eq' ? ' ' : ''}</span>
        <pre className="m-0 pr-2 font-mono text-[12.5px] leading-[1.55] whitespace-pre-wrap break-all text-seeyue-fg bg-transparent group-data-[op=pad]:text-seeyue-fg-dim">{rightText}</pre>
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
