import { useCallback, useEffect, useMemo, useState } from 'react'
import type { DirEntry, ListResp } from './types'
import {
  ChevronDown,
  ChevronRight,
  EyeOff,
  Eye,
  FileCode,
  FileGeneric,
  FileImage,
  FileMd,
  FileText,
  FilePlus,
  FolderClosed,
  FolderOpen,
  FolderRoot,
  Files,
  Search,
  pickFileIconKind,
} from './Icon'
import { PromptDialog } from './PromptDialog'

interface Props {
  root: string
  onChangeRoot: (path: string) => void
  showHidden: boolean
  onToggleHidden: () => void
  activePath: string | null
  onOpen: (path: string) => void
  /** 新建文件请求：父级把空文件创建在 dir 下，并把新建的绝对路径回传打开。 */
  onCreateFile?: (dir: string, name: string) => Promise<string>
}

/** 每个目录节点维护的状态 */
interface NodeState {
  loading: boolean
  expanded: boolean
  entries: DirEntry[] | null
  truncated: boolean
  error: string | null
}

/**
 * Typora 风文件树。
 *
 * - 顶部：头部 tab（文件 / 大纲——大纲走右侧栏，这里只是装饰提示），路径面包屑
 * - 中部：搜索框 + 隐藏文件切换按钮
 * - 主体：层级文件树，文件夹折叠/展开有 caret + folder icon 双重提示，
 *   active 文件用主色块 + 末尾 endLine 装饰
 */
export function FileTree(props: Props) {
  const { root, onChangeRoot, showHidden, onToggleHidden, activePath, onOpen, onCreateFile } =
    props
  // 以路径为 key 存储每个已访问目录的状态
  const [nodes, setNodes] = useState<Record<string, NodeState>>({})
  const [filter, setFilter] = useState('')
  const [pickingRoot, setPickingRoot] = useState(false)
  /** 新建文件对话框：null 表示未打开；非 null 时记录目标父目录绝对路径 */
  const [creatingIn, setCreatingIn] = useState<string | null>(null)
  const [creatingError, setCreatingError] = useState<string | null>(null)

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
      return {
        [root]: prev[root] ?? {
          loading: true,
          expanded: true,
          entries: null,
          truncated: false,
          error: null,
        },
      }
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

  /**
   * 「新建文件」对话框确认后实际调用。
   *
   * 失败：把错误回写到 dialog 上方红字提示，dialog 不关，方便用户改名重试。
   * 成功：刷新目标父目录的列表（让新文件出现），并通过 onOpen 直接打开它。
   */
  const submitCreate = useCallback(
    async (parentDir: string, name: string) => {
      if (!onCreateFile) {
        setCreatingError('当前环境不支持新建文件')
        return
      }
      try {
        const newPath = await onCreateFile(parentDir, name)
        setCreatingIn(null)
        setCreatingError(null)
        // 重载父目录，把新文件刷出来
        await loadDir(parentDir)
        onOpen(newPath)
      } catch (e) {
        setCreatingError(String(e))
      }
    },
    [onCreateFile, loadDir, onOpen],
  )

  // 路径面包屑分段
  const crumbs = useMemo(() => splitPath(root), [root])
  const filterLower = filter.trim().toLowerCase()

  return (
    <div className="h-full flex flex-col text-[13px] text-seeyue-fg seeyue-sidebar-shell">
      {/* —— 顶部："文件" 标题 + 视图切换 —— */}
      <div className="flex items-center gap-2 px-3 pt-3 pb-2 border-b border-seeyue-border">
        <button className="seeyue-tab" data-active="true">
          <Files size={14} />
          <span>文件</span>
        </button>
        <div className="flex-1" />
        {onCreateFile && (
          <button
            className="seeyue-icon-btn"
            onClick={() => {
              setCreatingError(null)
              setCreatingIn(root)
            }}
            title="在当前根目录新建文件"
          >
            <FilePlus size={15} />
          </button>
        )}
        <button
          className="seeyue-icon-btn"
          onClick={() => setPickingRoot(true)}
          title="切换根目录"
        >
          <FolderRoot size={15} />
        </button>
        <button
          className="seeyue-icon-btn"
          onClick={onToggleHidden}
          data-active={showHidden ? 'true' : undefined}
          title={showHidden ? '隐藏 dotfile' : '显示 dotfile'}
        >
          {showHidden ? <Eye size={15} /> : <EyeOff size={15} />}
        </button>
      </div>

      {/* —— 路径面包屑 —— */}
      <div
        className="px-3 pt-2 pb-1.5 text-[11px] text-seeyue-fg-dim flex items-center gap-1 truncate"
        title={root}
      >
        {crumbs.map((seg, i) => (
          <span key={i} className="flex items-center gap-1 truncate">
            {i > 0 && <span className="opacity-50">/</span>}
            <span
              className={
                i === crumbs.length - 1
                  ? 'text-seeyue-fg-strong truncate font-medium'
                  : 'truncate'
              }
            >
              {seg}
            </span>
          </span>
        ))}
      </div>

      {/* —— 搜索框 —— */}
      <div className="px-3 pt-2 pb-2">
        <div className="seeyue-search-box">
          <Search size={13} />
          <input
            type="text"
            value={filter}
            placeholder="过滤当前目录"
            onChange={(e) => setFilter(e.target.value)}
            spellCheck={false}
          />
          {filter && (
            <button
              className="seeyue-icon-btn"
              style={{ width: 18, height: 18 }}
              onClick={() => setFilter('')}
              title="清除"
            >
              <span style={{ fontSize: 11 }}>×</span>
            </button>
          )}
        </div>
      </div>

      {/* —— 树体 —— */}
      <div className="flex-1 overflow-y-auto pl-3 pr-2 pb-3">
        <DirNode
          path={root}
          depth={0}
          nodes={nodes}
          onToggle={toggleDir}
          onOpen={onOpen}
          activePath={activePath}
          filter={filterLower}
          onRequestCreate={
            onCreateFile
              ? (dir) => {
                  setCreatingError(null)
                  setCreatingIn(dir)
                }
              : undefined
          }
        />
      </div>

      {pickingRoot && (
        <PromptDialog
          title="切换文件树根目录"
          description="输入要展示的绝对路径："
          initialValue={root}
          placeholder="/Users/.../docs"
          onCancel={() => setPickingRoot(false)}
          onConfirm={(next) => {
            setPickingRoot(false)
            if (next && next !== root) onChangeRoot(next)
          }}
        />
      )}

      {creatingIn && (
        <PromptDialog
          title="新建文件"
          description={`将在 ${creatingIn} 下创建新文件：`}
          initialValue=""
          placeholder="例如 notes.md"
          confirmLabel="创建"
          error={creatingError ?? undefined}
          onCancel={() => {
            setCreatingIn(null)
            setCreatingError(null)
          }}
          onConfirm={(name) => {
            void submitCreate(creatingIn, name)
          }}
        />
      )}
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
  filter: string
  onRequestCreate?: (dir: string) => void
}

function DirNode({
  path,
  depth,
  nodes,
  onToggle,
  onOpen,
  activePath,
  filter,
  onRequestCreate,
}: DirNodeProps) {
  const state = nodes[path]
  const indent = (lvl: number) => 8 + lvl * 14

  // 在 early-return 之前调用 hooks，避免 react-hooks/rules-of-hooks 违例
  const entries = state?.entries
  const filtered = useMemo(() => {
    if (!entries) return null
    if (!filter) return entries
    return entries.filter((e) => e.name.toLowerCase().includes(filter))
  }, [entries, filter])

  if (!state) return null

  return (
    <div className={depth > 0 ? 'seeyue-tree-branch' : undefined}>
      {state.loading && !state.entries && (
        <div
          className="py-1 text-seeyue-fg-dim text-xs flex items-center gap-1"
          style={{ paddingLeft: indent(depth) }}
        >
          <span className="inline-block w-2 h-2 rounded-full bg-seeyue-fg-dim animate-pulse" />
          <span>加载中…</span>
        </div>
      )}
      {state.error && (
        <div
          className="py-1 text-seeyue-danger text-xs whitespace-pre-wrap"
          style={{ paddingLeft: indent(depth) }}
        >
          {state.error}
        </div>
      )}
      {state.expanded &&
        filtered?.map((entry) => (
          <EntryRow
            key={entry.path}
            entry={entry}
            depth={depth + 1}
            nodes={nodes}
            onToggle={onToggle}
            onOpen={onOpen}
            activePath={activePath}
            filter={filter}
            onRequestCreate={onRequestCreate}
          />
        ))}
      {state.expanded && filter && filtered && filtered.length === 0 && (
        <div
          className="py-1 text-seeyue-fg-dim text-xs italic"
          style={{ paddingLeft: indent(depth + 1) }}
        >
          无匹配项
        </div>
      )}
      {state.truncated && (
        <div
          className="py-1 text-seeyue-fg-dim text-xs italic"
          style={{ paddingLeft: indent(depth + 1) }}
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
  filter,
  onRequestCreate,
}: {
  entry: DirEntry
  depth: number
  nodes: Record<string, NodeState>
  onToggle: (path: string) => void
  onOpen: (path: string) => void
  activePath: string | null
  filter: string
  onRequestCreate?: (dir: string) => void
}) {
  const sub = nodes[entry.path]
  const isActive = !entry.is_dir && entry.path === activePath
  const indent = 4 + depth * 14
  return (
    <>
      <div
        className="seeyue-tree-row"
        data-active={isActive ? 'true' : undefined}
        style={{ paddingLeft: indent }}
        title={entry.path}
        role="button"
        tabIndex={0}
        onClick={() => (entry.is_dir ? onToggle(entry.path) : onOpen(entry.path))}
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault()
            entry.is_dir ? onToggle(entry.path) : onOpen(entry.path)
          }
        }}
      >
        <span className="seeyue-tree-caret">
          {entry.is_dir ? (
            sub?.expanded ? (
              <ChevronDown size={12} />
            ) : (
              <ChevronRight size={12} />
            )
          ) : null}
        </span>
        <span
          className="seeyue-tree-icon"
          data-kind={
            entry.is_dir
              ? sub?.expanded
                ? 'folder-open'
                : 'folder'
              : pickFileIconKind(entry.name)
          }
        >
          <FileGlyph
            name={entry.name}
            isDir={entry.is_dir}
            expanded={!!sub?.expanded}
          />
        </span>
        <span className="seeyue-tree-label">{entry.name}</span>
        {entry.is_dir && onRequestCreate && (
          <button
            type="button"
            className="seeyue-tree-action"
            title="在该目录新建文件"
            onClick={(e) => {
              e.stopPropagation()
              onRequestCreate(entry.path)
            }}
          >
            <FilePlus size={12} />
          </button>
        )}
      </div>
      {entry.is_dir && sub?.expanded && (
        <DirNode
          path={entry.path}
          depth={depth}
          nodes={nodes}
          onToggle={onToggle}
          onOpen={onOpen}
          activePath={activePath}
          filter={filter}
          onRequestCreate={onRequestCreate}
        />
      )}
    </>
  )
}

function FileGlyph({
  name,
  isDir,
  expanded,
}: {
  name: string
  isDir: boolean
  expanded: boolean
}) {
  if (isDir) {
    return expanded ? <FolderOpen size={15} /> : <FolderClosed size={15} />
  }
  const kind = pickFileIconKind(name)
  switch (kind) {
    case 'markdown':
      return <FileMd size={15} />
    case 'text':
      return <FileText size={15} />
    case 'code':
      return <FileCode size={15} />
    case 'image':
      return <FileImage size={15} />
    default:
      return <FileGeneric size={15} />
  }
}

function splitPath(p: string): string[] {
  if (!p) return ['(empty)']
  const parts = p.split('/').filter(Boolean)
  if (parts.length === 0) return ['/']
  if (parts.length <= 4) return parts
  return ['…', ...parts.slice(-3)]
}
