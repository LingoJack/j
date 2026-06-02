/**
 * JSON 查看器工具。
 *
 * 工作流：
 * 1. 顶部 textarea 粘贴 JSON 原文，按 ⌘↵ 或点"格式化"按钮 → 解析。
 *    - 解析失败：显示错误位置 + 原因，仍保留原文方便排查。
 *    - 成功：进入树形视图（折叠 / 修改）。
 * 2. 树形视图里：
 *    - 每个对象 / 数组节点头部一行，前面 ▶ / ▼ 折叠态可点；
 *    - 叶子（string / number / boolean / null）值可点击进入编辑：
 *      string 用文本框，number 用数字框（保留 NaN/Infinity 不允许），
 *      boolean / null 用下拉。
 *    - 改完按 Enter / blur 提交，按 Esc 取消。
 *    - 修改会原地更新 root，并实时把"序列化后的 JSON 字符串"反写回 textarea，
 *      让用户切回去就能看到结果（也是后续复制 / 粘走的来源）。
 * 3. 顶部按钮：格式化 / 压缩 / 折叠所有 / 展开所有 / 复制 / 清空。
 * 4. 全部纯前端，不接 /api。textarea 内容用 sessionStorage 兜底，刷页不丢。
 *
 * 注意：
 * - 大对象（>100k 节点）会卡，但常规配置文件场景没问题。
 * - 修改 key 暂不支持（避免破坏对象 key 顺序 / 重名风险），先放叶子值修改。
 */
import { useCallback, useEffect, useMemo, useState } from 'react'
import { ChevronDown, ChevronRight, Copy } from './Icon'

const STORAGE_KEY = 'jreader.tool.json.text'
const SAMPLE = `{
  "name": "j-cli",
  "version": "0.0.0",
  "private": true,
  "tags": ["cli", "rust", "tools"],
  "stats": {
    "stars": 0,
    "open": true,
    "todo": null
  }
}`

type JsonValue =
  | { kind: 'null' }
  | { kind: 'bool'; value: boolean }
  | { kind: 'number'; value: number }
  | { kind: 'string'; value: string }
  | { kind: 'array'; items: JsonValue[] }
  | { kind: 'object'; entries: Array<{ key: string; value: JsonValue }> }

type Path = (string | number)[]

export function JsonTool() {
  const [raw, setRaw] = useState<string>(
    () => sessionStorage.getItem(STORAGE_KEY) ?? SAMPLE,
  )
  const [tree, setTree] = useState<JsonValue | null>(null)
  const [error, setError] = useState<string | null>(null)
  /** 折叠状态：path 序列化为 "a.b.0" 形式做 key */
  const [collapsed, setCollapsed] = useState<Set<string>>(() => new Set())
  const [toast, setToast] = useState<string | null>(null)

  // 持久化原文
  useEffect(() => {
    sessionStorage.setItem(STORAGE_KEY, raw)
  }, [raw])

  // 首屏自动解析一次（拿到 sample 或上次的内容）
  useEffect(() => {
    try {
      const parsed = JSON.parse(raw)
      setTree(toTree(parsed))
      setError(null)
    } catch (e) {
      setTree(null)
      setError(formatParseError(e, raw))
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const parse = useCallback(() => {
    try {
      const parsed = JSON.parse(raw)
      setTree(toTree(parsed))
      setError(null)
      setCollapsed(new Set())
    } catch (e) {
      setTree(null)
      setError(formatParseError(e, raw))
    }
  }, [raw])

  const format = useCallback(() => {
    try {
      const parsed = JSON.parse(raw)
      const next = JSON.stringify(parsed, null, 2)
      setRaw(next)
      setTree(toTree(parsed))
      setError(null)
    } catch (e) {
      setError(formatParseError(e, raw))
    }
  }, [raw])

  const minify = useCallback(() => {
    try {
      const parsed = JSON.parse(raw)
      setRaw(JSON.stringify(parsed))
      setTree(toTree(parsed))
      setError(null)
    } catch (e) {
      setError(formatParseError(e, raw))
    }
  }, [raw])

  const copyAll = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(raw)
      flashToast('已复制 JSON', setToast)
    } catch {
      flashToast('复制失败', setToast)
    }
  }, [raw])

  const clear = useCallback(() => {
    setRaw('')
    setTree(null)
    setError(null)
    setCollapsed(new Set())
  }, [])

  /** 把当前 tree 序列化回 raw —— 节点修改后调用 */
  const flushTreeToRaw = useCallback((next: JsonValue) => {
    setTree(next)
    try {
      const obj = fromTree(next)
      setRaw(JSON.stringify(obj, null, 2))
    } catch {
      /* 不该出错，理论上 tree 永远 round-trip 安全 */
    }
  }, [])

  const updateAtPath = useCallback(
    (path: Path, mut: (cur: JsonValue) => JsonValue) => {
      if (!tree) return
      const next = updateTree(tree, path, mut)
      flushTreeToRaw(next)
    },
    [tree, flushTreeToRaw],
  )

  const toggleCollapse = useCallback((path: Path) => {
    const k = pathKey(path)
    setCollapsed((prev) => {
      const n = new Set(prev)
      if (n.has(k)) n.delete(k)
      else n.add(k)
      return n
    })
  }, [])

  const collapseAll = useCallback(() => {
    if (!tree) return
    const all = new Set<string>()
    walkTree(tree, [], (v, p) => {
      if (v.kind === 'object' || v.kind === 'array') {
        all.add(pathKey(p))
      }
    })
    setCollapsed(all)
  }, [tree])

  const expandAll = useCallback(() => {
    setCollapsed(new Set())
  }, [])

  // ⌘↵ 触发解析
  const onTextareaKey = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
        e.preventDefault()
        parse()
      }
    },
    [parse],
  )

  const stats = useMemo(() => (tree ? countTree(tree) : null), [tree])

  return (
    <div className="h-full flex flex-col bg-seeyue-bg">
      <div className="flex items-center gap-2.5 px-3.5 py-2 border-b border-seeyue-border bg-seeyue-sidebar-strong text-xs">
        <span className="text-[13px] font-medium text-seeyue-fg-strong">JSON 查看器</span>
        {stats && (
          <>
            <span className="font-mono text-[11.5px] px-2 py-0.5 rounded-full bg-seeyue-elevated text-seeyue-fg-muted data-[tone=add]:text-seeyue-success data-[tone=add]:bg-[rgba(163,190,140,0.14)] data-[tone=del]:text-seeyue-danger data-[tone=del]:bg-[rgba(191,97,106,0.16)] data-[tone=warn]:text-seeyue-warn data-[tone=warn]:bg-[rgba(208,135,112,0.18)] data-[tone=accent]:text-seeyue-accent data-[tone=accent]:bg-seeyue-accent-mute" data-tone="accent">
              对象 {stats.objects}
            </span>
            <span className="font-mono text-[11.5px] px-2 py-0.5 rounded-full bg-seeyue-elevated text-seeyue-fg-muted data-[tone=add]:text-seeyue-success data-[tone=add]:bg-[rgba(163,190,140,0.14)] data-[tone=del]:text-seeyue-danger data-[tone=del]:bg-[rgba(191,97,106,0.16)] data-[tone=warn]:text-seeyue-warn data-[tone=warn]:bg-[rgba(208,135,112,0.18)] data-[tone=accent]:text-seeyue-accent data-[tone=accent]:bg-seeyue-accent-mute" data-tone="accent">
              数组 {stats.arrays}
            </span>
            <span className="font-mono text-[11.5px] px-2 py-0.5 rounded-full bg-seeyue-elevated text-seeyue-fg-muted data-[tone=add]:text-seeyue-success data-[tone=add]:bg-[rgba(163,190,140,0.14)] data-[tone=del]:text-seeyue-danger data-[tone=del]:bg-[rgba(191,97,106,0.16)] data-[tone=warn]:text-seeyue-warn data-[tone=warn]:bg-[rgba(208,135,112,0.18)] data-[tone=accent]:text-seeyue-accent data-[tone=accent]:bg-seeyue-accent-mute">叶子 {stats.leaves}</span>
          </>
        )}
        {error && (
          <span className="font-mono text-[11.5px] px-2 py-0.5 rounded-full bg-seeyue-elevated text-seeyue-fg-muted data-[tone=add]:text-seeyue-success data-[tone=add]:bg-[rgba(163,190,140,0.14)] data-[tone=del]:text-seeyue-danger data-[tone=del]:bg-[rgba(191,97,106,0.16)] data-[tone=warn]:text-seeyue-warn data-[tone=warn]:bg-[rgba(208,135,112,0.18)] data-[tone=accent]:text-seeyue-accent data-[tone=accent]:bg-seeyue-accent-mute" data-tone="warn" title={error}>
            ⚠ 解析错误
          </span>
        )}
        <div className="flex-1" />
        <button type="button" className="px-3.5 py-1.5 text-[12.5px] font-cjk rounded-md border border-transparent cursor-pointer bg-transparent text-seeyue-fg-muted transition-all duration-150 hover:text-seeyue-fg-strong hover:bg-seeyue-elevated" onClick={format} title="格式化（缩进 2）">
          格式化
        </button>
        <button type="button" className="px-3.5 py-1.5 text-[12.5px] font-cjk rounded-md border border-transparent cursor-pointer bg-transparent text-seeyue-fg-muted transition-all duration-150 hover:text-seeyue-fg-strong hover:bg-seeyue-elevated" onClick={minify} title="压缩">
          压缩
        </button>
        <button
          type="button"
          className="px-3.5 py-1.5 text-[12.5px] font-cjk rounded-md border border-transparent cursor-pointer bg-transparent text-seeyue-fg-muted transition-all duration-150 hover:text-seeyue-fg-strong hover:bg-seeyue-elevated"
          onClick={collapseAll}
          disabled={!tree}
          title="折叠所有"
        >
          折叠
        </button>
        <button
          type="button"
          className="px-3.5 py-1.5 text-[12.5px] font-cjk rounded-md border border-transparent cursor-pointer bg-transparent text-seeyue-fg-muted transition-all duration-150 hover:text-seeyue-fg-strong hover:bg-seeyue-elevated"
          onClick={expandAll}
          disabled={!tree}
          title="展开所有"
        >
          展开
        </button>
        <button
          type="button"
          className="px-3.5 py-1.5 text-[12.5px] font-cjk rounded-md border border-transparent cursor-pointer bg-transparent text-seeyue-fg-muted transition-all duration-150 hover:text-seeyue-fg-strong hover:bg-seeyue-elevated"
          onClick={copyAll}
          title="复制 JSON 文本"
        >
          <Copy size={12} /> 复制
        </button>
        <button type="button" className="px-3.5 py-1.5 text-[12.5px] font-cjk rounded-md border border-transparent cursor-pointer bg-transparent text-seeyue-fg-muted transition-all duration-150 hover:text-seeyue-fg-strong hover:bg-seeyue-elevated" onClick={clear} title="清空">
          清空
        </button>
      </div>

      <div className="flex-1 grid grid-cols-[380px_1fr] gap-px bg-seeyue-border min-h-0">
        <div className="flex flex-col bg-seeyue-bg min-w-0 min-h-0">
          <div className="flex items-center justify-between px-3 py-1.5 text-[11.5px] text-seeyue-fg-muted border-b border-seeyue-border bg-seeyue-sidebar tracking-[0.04em]">
            <span>原文（⌘↵ 解析）</span>
            <span className="text-[11px] text-seeyue-fg-dim">{raw.length} 字符</span>
          </div>
          <textarea
            className="seeyue-textarea flex-1 w-full px-2.5 py-2.5 text-[13px] leading-[1.6]"
            value={raw}
            spellCheck={false}
            placeholder="粘贴 JSON…"
            onChange={(e) => setRaw(e.target.value)}
            onKeyDown={onTextareaKey}
            onBlur={parse}
          />
        </div>

        <div className="flex flex-col bg-seeyue-bg min-w-0 min-h-0">
          <div className="flex items-center justify-between px-3 py-1.5 text-[11.5px] text-seeyue-fg-muted border-b border-seeyue-border bg-seeyue-sidebar tracking-[0.04em]">
            <span>树视图</span>
            <span className="text-[11px] text-seeyue-fg-dim">点击叶子值可修改</span>
          </div>
          <div className="flex-1 overflow-auto px-3 pb-4 font-mono text-[12.5px] leading-[1.65]">
            {error ? (
              <pre className="text-seeyue-danger whitespace-pre-wrap font-mono text-[12.5px] m-0 px-2 py-2">{error}</pre>
            ) : tree ? (
              <JsonNode
                value={tree}
                path={[]}
                collapsed={collapsed}
                onToggle={toggleCollapse}
                onUpdate={updateAtPath}
              />
            ) : (
              <div className="text-seeyue-fg-dim font-cjk text-[12.5px] px-4 py-4">输入 JSON 后这里会出现树形结构</div>
            )}
          </div>
        </div>
      </div>

      {toast && <div className="absolute right-[18px] top-16 px-3 py-1.5 bg-seeyue-elevated border border-seeyue-border-strong border-l-[3px] border-l-seeyue-success rounded-md text-xs text-seeyue-fg shadow-[0_4px_12px_rgba(0,0,0,0.3)] animate-seeyue-fade-in">{toast}</div>}
    </div>
  )
}

// ---------------------------------------------------------------------------
// JsonNode：递归渲染单个节点
// ---------------------------------------------------------------------------

interface NodeProps {
  value: JsonValue
  path: Path
  /** 当节点是 object/array 的子项时，父级会传 keyLabel：对象 key 或 [i] */
  keyLabel?: string
  collapsed: Set<string>
  onToggle: (path: Path) => void
  onUpdate: (path: Path, mut: (cur: JsonValue) => JsonValue) => void
}

function JsonNode({ value, path, keyLabel, collapsed, onToggle, onUpdate }: NodeProps) {
  if (value.kind === 'object' || value.kind === 'array') {
    const isObj = value.kind === 'object'
    const items = isObj ? value.entries : value.items
    const len = items.length
    const isCollapsed = collapsed.has(pathKey(path))
    const open = isObj ? '{' : '['
    const close = isObj ? '}' : ']'
    return (
      <div className="relative">
        <div className="flex items-center gap-1 py-px cursor-default rounded-sm hover:bg-[rgba(94,129,172,0.08)]" onClick={() => onToggle(path)}>
          <span className="shrink-0 inline-flex items-center justify-center w-[13px] text-seeyue-fg-dim cursor-pointer">
            {len > 0 ? (
              isCollapsed ? <ChevronRight size={11} /> : <ChevronDown size={11} />
            ) : (
              <span style={{ display: 'inline-block', width: 11 }} />
            )}
          </span>
          {keyLabel !== undefined && (
            <span className="text-seeyue-accent">{keyLabel}</span>
          )}
          {keyLabel !== undefined && <span className="text-seeyue-fg-dim mr-0.5">:</span>}
          <span className="text-seeyue-fg-muted">{open}</span>
          {isCollapsed && len > 0 && (
            <span className="text-seeyue-fg-dim italic mx-1">
              {len} {isObj ? '项' : '元素'}
            </span>
          )}
          {isCollapsed && <span className="text-seeyue-fg-muted">{close}</span>}
        </div>
        {!isCollapsed && (
          <>
            <div className="ml-3.5 pl-1.5 border-l border-dashed border-[rgba(76,86,106,0.55)]">
              {isObj
                ? value.entries.map((e, i) => (
                    <JsonNode
                      key={`${e.key}-${i}`}
                      value={e.value}
                      path={[...path, e.key]}
                      keyLabel={JSON.stringify(e.key)}
                      collapsed={collapsed}
                      onToggle={onToggle}
                      onUpdate={onUpdate}
                    />
                  ))
                : value.items.map((it, i) => (
                    <JsonNode
                      key={i}
                      value={it}
                      path={[...path, i]}
                      keyLabel={`[${i}]`}
                      collapsed={collapsed}
                      onToggle={onToggle}
                      onUpdate={onUpdate}
                    />
                  ))}
            </div>
            <div className="flex items-center gap-1 py-px cursor-default rounded-sm hover:bg-[rgba(94,129,172,0.08)] cursor-default">
              <span className="shrink-0 inline-flex items-center justify-center w-[13px] text-seeyue-fg-dim cursor-pointer">
                <span style={{ display: 'inline-block', width: 11 }} />
              </span>
              <span className="text-seeyue-fg-muted">{close}</span>
            </div>
          </>
        )}
      </div>
    )
  }
  // 叶子
  return (
    <div className="relative">
      <div className="flex items-center gap-1 py-px cursor-default rounded-sm hover:bg-[rgba(94,129,172,0.08)]">
        <span className="shrink-0 inline-flex items-center justify-center w-[13px] text-seeyue-fg-dim cursor-pointer">
          <span style={{ display: 'inline-block', width: 11 }} />
        </span>
        {keyLabel !== undefined && (
          <span className="text-seeyue-accent">{keyLabel}</span>
        )}
        {keyLabel !== undefined && <span className="text-seeyue-fg-dim mr-0.5">:</span>}
        <LeafEditor
          value={value}
          onCommit={(next) => onUpdate(path, () => next)}
        />
      </div>
    </div>
  )
}

// ---------------------------------------------------------------------------
// LeafEditor：叶子值的就地编辑
// ---------------------------------------------------------------------------

function LeafEditor({
  value,
  onCommit,
}: {
  value: JsonValue
  onCommit: (next: JsonValue) => void
}) {
  const [editing, setEditing] = useState(false)
  const [draft, setDraft] = useState<string>('')

  const startEdit = () => {
    setDraft(serializeLeaf(value))
    setEditing(true)
  }

  const commitText = (text: string) => {
    setEditing(false)
    const parsed = parseLeafText(text, value.kind)
    if (parsed) onCommit(parsed)
  }

  if (!editing) {
    return (
      <span
        className={`cursor-pointer px-1 rounded-sm border border-dashed border-transparent transition-all duration-150 hover:border-seeyue-border-strong hover:bg-seeyue-elevated ${value.kind === 'string' ? 'text-seeyue-success' : value.kind === 'number' ? 'text-seeyue-warn' : value.kind === 'bool' ? 'text-seeyue-purple' : value.kind === 'null' ? 'text-seeyue-fg-dim italic' : ''}`}
        onClick={startEdit}
        title="点击修改"
      >
        {renderLeaf(value)}
      </span>
    )
  }

  // 编辑态：根据原 kind 给不同输入控件
  if (value.kind === 'bool') {
    return (
      <select
        className="inline-flex items-center h-[22px] px-1.5 bg-seeyue-bg-deep border border-seeyue-accent rounded text-seeyue-fg font-mono text-[12.5px] outline-none min-w-[80px] cursor-pointer"
        autoFocus
        defaultValue={String(value.value)}
        onBlur={(e) => commitText(e.currentTarget.value)}
        onChange={(e) => commitText(e.currentTarget.value)}
      >
        <option value="true">true</option>
        <option value="false">false</option>
      </select>
    )
  }
  if (value.kind === 'null') {
    // null 不让编辑成别的 —— 用户可以点击切换成 string，再转
    // 简化：null 显示一个能切到 string 的小提示
    return (
      <select
        className="inline-flex items-center h-[22px] px-1.5 bg-seeyue-bg-deep border border-seeyue-accent rounded text-seeyue-fg font-mono text-[12.5px] outline-none min-w-[80px] cursor-pointer"
        autoFocus
        defaultValue="null"
        onBlur={(e) => {
          const v = e.currentTarget.value
          if (v === 'null') {
            setEditing(false)
            return
          }
          if (v === 'true' || v === 'false') {
            onCommit({ kind: 'bool', value: v === 'true' })
          } else if (v === '""') {
            onCommit({ kind: 'string', value: '' })
          } else if (v === '0') {
            onCommit({ kind: 'number', value: 0 })
          }
          setEditing(false)
        }}
      >
        <option value="null">null</option>
        <option value="true">true</option>
        <option value="false">false</option>
        <option value='""'>"" (空字符串)</option>
        <option value="0">0 (数字)</option>
      </select>
    )
  }
  return (
    <input
      className="inline-flex items-center h-[22px] px-1.5 bg-seeyue-bg-deep border border-seeyue-accent rounded text-seeyue-fg font-mono text-[12.5px] outline-none min-w-[80px]"
      autoFocus
      type="text"
      value={draft}
      onChange={(e) => setDraft(e.target.value)}
      onBlur={(e) => commitText(e.currentTarget.value)}
      onKeyDown={(e) => {
        if (e.key === 'Enter') {
          e.preventDefault()
          commitText(draft)
        } else if (e.key === 'Escape') {
          e.preventDefault()
          setEditing(false)
        }
      }}
    />
  )
}

function serializeLeaf(v: JsonValue): string {
  switch (v.kind) {
    case 'string':
      return v.value
    case 'number':
      return String(v.value)
    case 'bool':
      return String(v.value)
    case 'null':
      return 'null'
    default:
      return ''
  }
}

function parseLeafText(text: string, originKind: JsonValue['kind']): JsonValue | null {
  if (originKind === 'number') {
    const n = Number(text.trim())
    if (!Number.isFinite(n)) return null
    return { kind: 'number', value: n }
  }
  if (originKind === 'bool') {
    if (text === 'true') return { kind: 'bool', value: true }
    if (text === 'false') return { kind: 'bool', value: false }
    return null
  }
  if (originKind === 'null') {
    return { kind: 'null' }
  }
  // string：原样保留（用户没办法在 input 里输入换行，简单处理）
  return { kind: 'string', value: text }
}

function renderLeaf(v: JsonValue) {
  switch (v.kind) {
    case 'string':
      return `"${escapeForDisplay(v.value)}"`
    case 'number':
      return String(v.value)
    case 'bool':
      return v.value ? 'true' : 'false'
    case 'null':
      return 'null'
    default:
      return ''
  }
}

function escapeForDisplay(s: string): string {
  // 仅做最小化转义，便于阅读
  return s.replace(/\\/g, '\\\\').replace(/"/g, '\\"')
}

// ---------------------------------------------------------------------------
// 工具函数
// ---------------------------------------------------------------------------

function toTree(v: unknown): JsonValue {
  if (v === null) return { kind: 'null' }
  if (typeof v === 'boolean') return { kind: 'bool', value: v }
  if (typeof v === 'number') return { kind: 'number', value: v }
  if (typeof v === 'string') return { kind: 'string', value: v }
  if (Array.isArray(v)) {
    return { kind: 'array', items: v.map(toTree) }
  }
  if (typeof v === 'object') {
    const obj = v as Record<string, unknown>
    return {
      kind: 'object',
      entries: Object.keys(obj).map((k) => ({ key: k, value: toTree(obj[k]) })),
    }
  }
  // undefined / function / symbol → null（JSON 不支持）
  return { kind: 'null' }
}

function fromTree(v: JsonValue): unknown {
  switch (v.kind) {
    case 'null':
      return null
    case 'bool':
      return v.value
    case 'number':
      return v.value
    case 'string':
      return v.value
    case 'array':
      return v.items.map(fromTree)
    case 'object': {
      const obj: Record<string, unknown> = {}
      for (const e of v.entries) obj[e.key] = fromTree(e.value)
      return obj
    }
  }
}

function updateTree(root: JsonValue, path: Path, mut: (cur: JsonValue) => JsonValue): JsonValue {
  if (path.length === 0) return mut(root)
  const [head, ...rest] = path
  if (root.kind === 'object' && typeof head === 'string') {
    return {
      kind: 'object',
      entries: root.entries.map((e) =>
        e.key === head ? { key: e.key, value: updateTree(e.value, rest, mut) } : e,
      ),
    }
  }
  if (root.kind === 'array' && typeof head === 'number') {
    return {
      kind: 'array',
      items: root.items.map((it, i) =>
        i === head ? updateTree(it, rest, mut) : it,
      ),
    }
  }
  return root
}

function walkTree(
  v: JsonValue,
  path: Path,
  cb: (v: JsonValue, p: Path) => void,
) {
  cb(v, path)
  if (v.kind === 'object') {
    for (const e of v.entries) walkTree(e.value, [...path, e.key], cb)
  } else if (v.kind === 'array') {
    v.items.forEach((it, i) => walkTree(it, [...path, i], cb))
  }
}

function countTree(v: JsonValue) {
  let objects = 0
  let arrays = 0
  let leaves = 0
  walkTree(v, [], (n) => {
    if (n.kind === 'object') objects++
    else if (n.kind === 'array') arrays++
    else leaves++
  })
  return { objects, arrays, leaves }
}

function pathKey(p: Path): string {
  return p.map((seg) => (typeof seg === 'number' ? `[${seg}]` : `.${seg}`)).join('') || '$'
}

function formatParseError(e: unknown, raw: string): string {
  const msg = e instanceof Error ? e.message : String(e)
  // 试着从 message 里挖 "position 12" 这种，标个行列
  const m = /position\s+(\d+)/.exec(msg)
  if (m) {
    const pos = Number(m[1])
    const before = raw.slice(0, pos)
    const line = before.split('\n').length
    const col = before.length - before.lastIndexOf('\n')
    return `${msg}\n位置：行 ${line}，列 ${col}`
  }
  return msg
}

function flashToast(message: string, set: (s: string | null) => void) {
  set(message)
  window.setTimeout(() => set(null), 1400)
}
