import { useCallback, useEffect, useState } from 'react'
import type { DirEntry, ListResp } from './types'

interface Props {
  root: string
  onChangeRoot: (path: string) => void
  showHidden: boolean
  onToggleHidden: () => void
  activePath: string | null
  onOpen: (path: string) => void
}

/** 每个目录节点维护的状态 */
interface NodeState {
  loading: boolean
  expanded: boolean
  entries: DirEntry[] | null
  truncated: boolean
  error: string | null
}

export function FileTree(props: Props) {
  const { root, onChangeRoot, showHidden, onToggleHidden, activePath, onOpen } =
    props
  // 以路径为 key 存储每个已访问目录的状态
  const [nodes, setNodes] = useState<Record<string, NodeState>>({})

  const loadDir = useCallback(
    async (dir: string) => {
      setNodes((prev) => ({
        ...prev,
        [dir]: {
          loading: true,
          expanded: true,
          entries: prev[dir]?.entries ?? null,
          truncated: prev[dir]?.truncated ?? false,
          error: null,
        },
      }))
      try {
        const params = new URLSearchParams({
          dir,
          hidden: showHidden ? '1' : '0',
        })
        const res = await fetch(`./api/list?${params.toString()}`)
        if (!res.ok) {
          const body = await res.json().catch(() => ({}))
          throw new Error(body.error ?? `HTTP ${res.status}`)
        }
        const data = (await res.json()) as ListResp
        setNodes((prev) => ({
          ...prev,
          [dir]: {
            loading: false,
            expanded: true,
            entries: data.entries,
            truncated: data.truncated,
            error: null,
          },
        }))
      } catch (e) {
        setNodes((prev) => ({
          ...prev,
          [dir]: {
            loading: false,
            expanded: true,
            entries: null,
            truncated: false,
            error: String(e),
          },
        }))
      }
    },
    [showHidden],
  )

  // 切根目录或 showHidden 翻转 → 重载根目录
  useEffect(() => {
    if (!root) return
    void loadDir(root)
    // showHidden 变了，已展开的节点也需要刷新；这里清空缓存让 useEffect 重跑根目录，
    // 子目录在用户重新展开时按需加载
    setNodes((prev) => {
      // 仅保留 root，其他清掉
      return { [root]: prev[root] ?? { loading: true, expanded: true, entries: null, truncated: false, error: null } }
    })
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [root, showHidden])

  const toggleDir = useCallback(
    (dir: string) => {
      const state = nodes[dir]
      if (!state || !state.entries) {
        void loadDir(dir)
        return
      }
      setNodes((prev) => ({
        ...prev,
        [dir]: { ...state, expanded: !state.expanded },
      }))
    },
    [nodes, loadDir],
  )

  const handlePickRoot = () => {
    const next = window.prompt('切换文件树根目录（输入绝对路径）：', root)
    if (!next || next === root) return
    onChangeRoot(next.trim())
  }

  return (
    <div className="h-full flex flex-col text-[13px] text-seeyue-fg">
      {/* 顶部工具栏 */}
      <div className="flex items-center gap-1 px-3 py-2 border-b border-seeyue-border">
        <button
          onClick={handlePickRoot}
          className="text-seeyue-fg-muted hover:text-seeyue-fg-strong text-xs truncate flex-1 text-left"
          title={root}
        >
          {pathTail(root)}
        </button>
        <button
          onClick={onToggleHidden}
          className={`text-xs px-1.5 py-0.5 rounded transition-colors ${
            showHidden
              ? 'text-seeyue-accent bg-seeyue-accent-soft'
              : 'text-seeyue-fg-dim hover:text-seeyue-fg-muted'
          }`}
          title="切换显示隐藏文件（dotfile）"
        >
          .*
        </button>
      </div>
      {/* 树体 */}
      <div className="flex-1 overflow-y-auto py-1">
        <DirNode
          path={root}
          depth={0}
          nodes={nodes}
          onToggle={toggleDir}
          onOpen={onOpen}
          activePath={activePath}
        />
      </div>
    </div>
  )
}

interface DirNodeProps {
  path: string
  depth: number
  nodes: Record<string, NodeState>
  onToggle: (path: string) => void
  onOpen: (path: string) => void
  activePath: string | null
}

function DirNode({
  path,
  depth,
  nodes,
  onToggle,
  onOpen,
  activePath,
}: DirNodeProps) {
  const state = nodes[path]
  if (!state) return null

  return (
    <div>
      {state.loading && !state.entries && (
        <div
          className="px-3 py-1 text-seeyue-fg-dim text-xs"
          style={{ paddingLeft: 12 + depth * 14 }}
        >
          加载中…
        </div>
      )}
      {state.error && (
        <div
          className="px-3 py-1 text-seeyue-danger text-xs whitespace-pre-wrap"
          style={{ paddingLeft: 12 + depth * 14 }}
        >
          {state.error}
        </div>
      )}
      {state.expanded &&
        state.entries?.map((entry) => (
          <EntryRow
            key={entry.path}
            entry={entry}
            depth={depth + 1}
            nodes={nodes}
            onToggle={onToggle}
            onOpen={onOpen}
            activePath={activePath}
          />
        ))}
      {state.truncated && (
        <div
          className="px-3 py-1 text-seeyue-fg-dim text-xs italic"
          style={{ paddingLeft: 12 + (depth + 1) * 14 }}
        >
          目录过大，仅显示前 2000 项
        </div>
      )}
    </div>
  )
}

function EntryRow({
  entry,
  depth,
  nodes,
  onToggle,
  onOpen,
  activePath,
}: {
  entry: DirEntry
  depth: number
  nodes: Record<string, NodeState>
  onToggle: (path: string) => void
  onOpen: (path: string) => void
  activePath: string | null
}) {
  const sub = nodes[entry.path]
  const isActive = !entry.is_dir && entry.path === activePath
  return (
    <>
      <button
        onClick={() => (entry.is_dir ? onToggle(entry.path) : onOpen(entry.path))}
        className={`w-full text-left flex items-center gap-1 py-0.5 pr-2 transition-colors ${
          isActive
            ? 'bg-seeyue-accent-soft text-seeyue-fg-strong'
            : 'text-seeyue-fg hover:bg-seeyue-panel'
        }`}
        style={{ paddingLeft: 8 + depth * 14 }}
        title={entry.path}
      >
        <span className="w-3 text-seeyue-fg-dim text-[10px]">
          {entry.is_dir ? (sub?.expanded ? '▾' : '▸') : ''}
        </span>
        <span className="w-4 text-seeyue-fg-dim text-[11px]">
          {entry.is_dir ? '📁' : <FileIcon name={entry.name} />}
        </span>
        <span className="truncate text-[13px]">{entry.name}</span>
      </button>
      {entry.is_dir && sub?.expanded && (
        <DirNode
          path={entry.path}
          depth={depth}
          nodes={nodes}
          onToggle={onToggle}
          onOpen={onOpen}
          activePath={activePath}
        />
      )}
    </>
  )
}

/** 把绝对路径裁成「.../parent/name」 */
function pathTail(p: string): string {
  if (!p) return ''
  const parts = p.split('/').filter(Boolean)
  if (parts.length <= 2) return p
  return '…/' + parts.slice(-2).join('/')
}

/** 文件图标：.md / .markdown 显示「M↓」蓝徽章；其它扩展用 📄 */
function FileIcon({ name }: { name: string }) {
  const lower = name.toLowerCase()
  if (lower.endsWith('.md') || lower.endsWith('.markdown')) {
    return <span className="seeyue-md-badge">M↓</span>
  }
  return <span>📄</span>
}
