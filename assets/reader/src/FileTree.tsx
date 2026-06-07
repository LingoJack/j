import { useCallback, useEffect, useMemo, useState } from 'react'
import type { DirEntry, ListResp } from './types'
import {
  ChevronDown,
  ChevronRight,
  FileCode,
  FileGeneric,
  FileImage,
  FileMd,
  FileText,
  FilePlus,
  FolderClosed,
  FolderOpen,
  Search,
} from './Icon'
import { pickFileIconKind } from './fileIconKind'
import { PromptDialog } from './PromptDialog'

interface Props {
  root: string
  activePath: string | null
  onOpen: (path: string) => void
  /** 新建文件请求：父级把空文件创建在 dir 下，并把新建的绝对路径回传打开。 */
  onCreateFile?: (dir: string, name: string) => Promise<string>
  /** 新建文件夹请求：父级把新目录创建在 dir 下，并返回新目录绝对路径。 */
  onCreateFolder?: (dir: string, name: string) => Promise<string>
}

type CreateKind = 'file' | 'folder'

interface CreateTarget {
  dir: string
  kind: CreateKind
}

interface ContextMenuState {
  x: number
  y: number
  dir: string
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
 * VS Code 风文件树。
 *
 * - 顶部保留根路径面包屑和过滤框，不额外放置操作按钮
 * - 主体：层级文件树，文件夹折叠/展开有 caret + folder icon 双重提示
 * - 目录行的新建文件入口仅在 hover/focus 时出现，避免常驻工具栏干扰
 */
export function FileTree(props: Props) {
  const { root, activePath, onOpen, onCreateFile, onCreateFolder } = props
  // 以路径为 key 存储每个已访问目录的状态
  const [nodes, setNodes] = useState<Record<string, NodeState>>({})
  const [filter, setFilter] = useState('')
  /** 新建对话框：null 表示未打开；非 null 时记录目标父目录和创建类型 */
  const [creating, setCreating] = useState<CreateTarget | null>(null)
  const [creatingError, setCreatingError] = useState<string | null>(null)
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null)

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
          hidden: '0',
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
    []
  )

  // 切根目录 → 重载根目录
  useEffect(() => {
    if (!root) return
    void loadDir(root)
    setNodes((prev) => {
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
  }, [root])

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
    [nodes, loadDir]
  )

  const openCreateDialog = useCallback((dir: string, kind: CreateKind) => {
    setContextMenu(null)
    setCreatingError(null)
    setCreating({ dir, kind })
  }, [])

  /**
   * 「新建」对话框确认后实际调用。
   *
   * 失败：把错误回写到 dialog 上方红字提示，dialog 不关，方便用户改名重试。
   * 成功：刷新目标父目录的列表；新文件直接打开，新文件夹保持在树中可见。
   */
  const submitCreate = useCallback(
    async (target: CreateTarget, name: string) => {
      const creator = target.kind === 'file' ? onCreateFile : onCreateFolder
      if (!creator) {
        setCreatingError(target.kind === 'file' ? '当前环境不支持新建文件' : '当前环境不支持新建文件夹')
        return
      }
      try {
        const newPath = await creator(target.dir, name)
        setCreating(null)
        setCreatingError(null)
        await loadDir(target.dir)
        if (target.kind === 'file') {
          onOpen(newPath)
        }
      } catch (e) {
        setCreatingError(String(e))
      }
    },
    [onCreateFile, onCreateFolder, loadDir, onOpen]
  )

  // 路径面包屑分段
  const crumbs = useMemo(() => splitPath(root), [root])
  const filterLower = filter.trim().toLowerCase()

  return (
    <div
      className="h-full flex flex-col text-[13px] text-seeyue-fg bg-seeyue-sidebar"
      onContextMenu={(e) => {
        e.preventDefault()
        if (onCreateFile || onCreateFolder) {
          setContextMenu({ x: e.clientX, y: e.clientY, dir: root })
        }
      }}
      onClick={() => setContextMenu(null)}
    >
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
                i === crumbs.length - 1 ? 'text-seeyue-fg-strong truncate font-medium' : 'truncate'
              }
            >
              {seg}
            </span>
          </span>
        ))}
      </div>

      {/* —— 搜索框 —— */}
      <div className="px-3 pt-2 pb-2">
        <div className="flex items-center gap-1.5 h-7 px-2 bg-seeyue-bg border border-seeyue-border rounded text-seeyue-fg-muted transition-colors duration-150 focus-within:border-seeyue-accent focus-within:bg-seeyue-bg">
          <Search size={13} />
          <input
            type="text"
            value={filter}
            placeholder="过滤当前目录"
            onChange={(e) => setFilter(e.target.value)}
            spellCheck={false}
            className="flex-1 min-w-0 bg-transparent border-0 outline-none text-seeyue-fg text-[12.5px] font-cjk placeholder:text-seeyue-fg-dim"
          />
          {filter && (
            <button
              className="inline-flex items-center justify-center w-[26px] h-[26px] rounded text-seeyue-fg-dim bg-transparent border-0 cursor-pointer transition-all duration-150 hover:text-seeyue-fg-strong hover:bg-seeyue-elevated disabled:opacity-30 disabled:cursor-not-allowed"
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
          onRequestCreate={onCreateFile || onCreateFolder ? openCreateDialog : undefined}
          onRequestMenu={onCreateFile || onCreateFolder ? setContextMenu : undefined}
        />
      </div>

      {creating && (
        <PromptDialog
          title={creating.kind === 'file' ? '新建文件' : '新建文件夹'}
          description={`将在 ${creating.dir} 下创建${creating.kind === 'file' ? '新文件' : '新文件夹'}：`}
          initialValue=""
          placeholder={creating.kind === 'file' ? '例如 notes.md' : '例如 assets'}
          confirmLabel="创建"
          error={creatingError ?? undefined}
          onCancel={() => {
            setCreating(null)
            setCreatingError(null)
          }}
          onConfirm={(name) => {
            void submitCreate(creating, name)
          }}
        />
      )}

      {contextMenu && (
        <div
          className="fixed z-50 min-w-[150px] overflow-hidden rounded-md border border-seeyue-border bg-seeyue-bg/95 py-1 text-[13px] text-seeyue-fg shadow-[0_12px_28px_rgba(0,0,0,0.22)] backdrop-blur"
          style={{ left: contextMenu.x, top: contextMenu.y }}
          onClick={(e) => e.stopPropagation()}
          onContextMenu={(e) => e.preventDefault()}
        >
          {onCreateFile && (
            <button
              type="button"
              className="flex w-full items-center gap-2 border-0 bg-transparent px-3 py-1.5 text-left text-seeyue-fg-muted cursor-pointer hover:bg-seeyue-elevated hover:text-seeyue-fg-strong"
              onClick={() => openCreateDialog(contextMenu.dir, 'file')}
            >
              <FilePlus size={14} />
              <span>新建文件</span>
            </button>
          )}
          {onCreateFolder && (
            <button
              type="button"
              className="flex w-full items-center gap-2 border-0 bg-transparent px-3 py-1.5 text-left text-seeyue-fg-muted cursor-pointer hover:bg-seeyue-elevated hover:text-seeyue-fg-strong"
              onClick={() => openCreateDialog(contextMenu.dir, 'folder')}
            >
              <FolderClosed size={14} />
              <span>新建文件夹</span>
            </button>
          )}
        </div>
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
  onRequestCreate?: (dir: string, kind: CreateKind) => void
  onRequestMenu?: (menu: ContextMenuState) => void
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
  onRequestMenu,
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
    <div
      className={
        depth > 0
          ? "relative before:content-[''] before:absolute before:top-0 before:bottom-0 before:w-px before:bg-seeyue-border"
          : undefined
      }
    >
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
            onRequestMenu={onRequestMenu}
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
  onRequestMenu,
}: {
  entry: DirEntry
  depth: number
  nodes: Record<string, NodeState>
  onToggle: (path: string) => void
  onOpen: (path: string) => void
  activePath: string | null
  filter: string
  onRequestCreate?: (dir: string, kind: CreateKind) => void
  onRequestMenu?: (menu: ContextMenuState) => void
}) {
  const sub = nodes[entry.path]
  const isActive = !entry.is_dir && entry.path === activePath
  const indent = 4 + depth * 14
  return (
    <>
      <div
        className="group flex min-h-7 w-full items-center gap-1 rounded-sm border-0 bg-transparent py-[3px] pr-1.5 text-left text-[13px] leading-snug text-seeyue-fg-muted cursor-pointer relative transition-colors duration-150 hover:bg-seeyue-elevated hover:text-seeyue-fg data-[active=true]:bg-seeyue-accent-mute data-[active=true]:text-seeyue-accent data-[active=true]:font-medium before:content-[''] before:absolute before:left-0 before:top-1 before:bottom-1 before:w-[3px] before:rounded-r before:bg-transparent data-[active=true]:before:bg-seeyue-accent"
        data-active={isActive ? 'true' : undefined}
        style={{ paddingLeft: indent }}
        title={entry.path}
        role="button"
        tabIndex={0}
        onClick={() => (entry.is_dir ? onToggle(entry.path) : onOpen(entry.path))}
        onContextMenu={(e) => {
          e.preventDefault()
          e.stopPropagation()
          if (onRequestMenu) {
            onRequestMenu({ x: e.clientX, y: e.clientY, dir: entry.is_dir ? entry.path : parentDir(entry.path) })
          }
        }}
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault()
            if (entry.is_dir) {
              onToggle(entry.path)
            } else {
              onOpen(entry.path)
            }
          }
        }}
      >
        <span className="shrink-0 text-seeyue-fg-dim inline-flex items-center justify-center w-[14px] h-[14px] transition-colors duration-150 group-hover:text-seeyue-fg-muted group-data-[active=true]:text-seeyue-accent">
          {entry.is_dir ? (
            sub?.expanded ? (
              <ChevronDown size={12} />
            ) : (
              <ChevronRight size={12} />
            )
          ) : null}
        </span>
        <span
          className="shrink-0 inline-flex items-center justify-center w-[18px] h-[18px] text-seeyue-fg-dim transition-colors duration-150 data-[kind=folder]:text-seeyue-fg-dim data-[kind=folder-open]:text-seeyue-fg-muted data-[kind=markdown]:text-seeyue-fg-muted data-[kind=text]:text-seeyue-fg-dim data-[kind=code]:text-seeyue-fg-muted data-[kind=image]:text-seeyue-fg-muted data-[kind=generic]:text-seeyue-fg-dim group-hover:text-seeyue-fg-muted group-data-[active=true]:text-seeyue-accent"
          data-kind={
            entry.is_dir ? (sub?.expanded ? 'folder-open' : 'folder') : pickFileIconKind(entry.name)
          }
        >
          <FileGlyph name={entry.name} isDir={entry.is_dir} expanded={!!sub?.expanded} />
        </span>
        <span className="flex-1 min-w-0 whitespace-nowrap overflow-hidden text-ellipsis">
          {entry.name}
        </span>
        {entry.is_dir && onRequestCreate && (
          <button
            type="button"
            className="shrink-0 inline-flex items-center justify-center w-[18px] h-[18px] rounded border-0 bg-transparent text-seeyue-fg-dim cursor-pointer opacity-0 transition-all duration-150 mr-1 group-hover:opacity-100 group-focus-within:opacity-100 hover:text-seeyue-success hover:bg-[rgba(163,190,140,0.15)] group-data-[active=true]:text-seeyue-fg-strong"
            title="在该目录新建文件"
              onClick={(e) => {
                e.stopPropagation()
                onRequestCreate(entry.path, 'file')
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
          onRequestMenu={onRequestMenu}
        />
      )}
    </>
  )
}

function FileGlyph({ name, isDir, expanded }: { name: string; isDir: boolean; expanded: boolean }) {
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

function parentDir(path: string): string {
  const normalized = path.replace(/\/+$/, '')
  const idx = normalized.lastIndexOf('/')
  if (idx <= 0) return '/'
  return normalized.slice(0, idx)
}

function splitPath(p: string): string[] {
  if (!p) return ['(empty)']
  const parts = p.split('/').filter(Boolean)
  if (parts.length === 0) return ['/']
  if (parts.length <= 4) return parts
  return ['…', ...parts.slice(-3)]
}
