import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { FileTree } from './FileTree'
import { TabBar } from './TabBar'
import { CloseConfirmDialog } from './CloseConfirmDialog'
import { MilkdownEditor } from './milkdown/MilkdownEditor'
import { PlainTextEditor } from './PlainTextEditor'
import { TableOfContents } from './TableOfContents'
import { extractHeadings } from './toc'
import { Toast } from './Toast'
import { MarkdownBaseDirContext } from './MarkdownIR'
import {
  BookOpen,
  Copy,
  Save,
  Sparkles,
} from './Icon'
import type {
  InitialResp,
  ParsedDocument,
  RenderedDoc,
  Tab,
} from './types'

type LoadState =
  | { kind: 'loading' }
  | { kind: 'error'; message: string }
  | { kind: 'ready' }

/** 同时打开新文件的并发上限（防误点把内存撑爆） */
const MAX_TABS = 32

export function Reader() {
  const [loadState, setLoadState] = useState<LoadState>({ kind: 'loading' })
  const [tabs, setTabs] = useState<Tab[]>([])
  const [activeTabPath, setActiveTabPath] = useState<string | null>(null)
  const [treeRoot, setTreeRoot] = useState<string>('')
  const [showHidden, setShowHidden] = useState(false)
  /** 关闭 dirty Tab 时弹出三选项确认 */
  const [closing, setClosing] = useState<{ path: string } | null>(null)
  /** 错误 / 成功提示（替代 alert） */
  const [toast, setToast] = useState<{
    message: string
    kind: 'error' | 'success' | 'info'
  } | null>(null)
  /** TOC 折叠态，持久化到 localStorage */
  const [tocCollapsed, setTocCollapsed] = useState<boolean>(() => {
    return localStorage.getItem('jreader.tocCollapsed') === '1'
  })
  const toggleToc = useCallback(() => {
    setTocCollapsed((prev) => {
      const next = !prev
      localStorage.setItem('jreader.tocCollapsed', next ? '1' : '0')
      return next
    })
  }, [])

  // —— 初始化：拉 /api/initial → （如果有 initial_path）打开 initial tab ——
  useEffect(() => {
    let cancelled = false
    ;(async () => {
      try {
        const initial = (await fetch('./api/initial').then((r) => {
          if (!r.ok) throw new Error(`initial HTTP ${r.status}`)
          return r.json()
        })) as InitialResp

        if (cancelled) return
        setTreeRoot(initial.root_dir)

        if (!initial.initial_path) {
          // 目录入口：仅显示文件树，不预选文件
          setLoadState({ kind: 'ready' })
          return
        }

        const doc = (await fetch(
          `./api/file?path=${encodeURIComponent(initial.initial_path)}`,
        ).then((r) => {
          if (!r.ok) throw new Error(`file HTTP ${r.status}`)
          return r.json()
        })) as RenderedDoc
        if (cancelled) return

        setTabs([docToTab(doc)])
        setActiveTabPath(doc.path)
        setLoadState({ kind: 'ready' })
      } catch (e) {
        if (!cancelled) setLoadState({ kind: 'error', message: String(e) })
      }
    })()
    return () => {
      cancelled = true
    }
  }, [])

  const activeTab = useMemo(
    () => tabs.find((t) => t.path === activeTabPath) ?? null,
    [tabs, activeTabPath],
  )
  const anyDirty = tabs.some((t) => t.dirty)
  /** 当前文档所在目录（用于解析 markdown 里的相对图片路径） */
  const baseDir = useMemo(() => {
    if (!activeTab) return null
    const i = activeTab.path.lastIndexOf('/')
    return i >= 0 ? activeTab.path.slice(0, i) : null
  }, [activeTab])

  // —— 标题栏 + beforeunload 同步 ——
  useDirtyTitle(activeTab, anyDirty)

  // —— Cmd+S：交给 active tab ——
  const activeTabRef = useRef<Tab | null>(null)
  activeTabRef.current = activeTab
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      const isSave =
        (e.metaKey || e.ctrlKey) && (e.key === 's' || e.key === 'S')
      if (!isSave) return
      e.preventDefault()
      const t = activeTabRef.current
      if (t) void saveTab(t.path)
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  // —— Tab 操作 ——
  const updateTab = useCallback((path: string, patch: Partial<Tab>) => {
    setTabs((prev) =>
      prev.map((t) => (t.path === path ? { ...t, ...patch } : t)),
    )
  }, [])

  const openFile = useCallback(
    async (path: string) => {
      // 已存在则切到该 tab
      if (tabs.some((t) => t.path === path)) {
        setActiveTabPath(path)
        return
      }
      if (tabs.length >= MAX_TABS) {
        setToast({
          message: `已打开 ${MAX_TABS} 个 Tab，关闭一些再试`,
          kind: 'info',
        })
        return
      }
      try {
        const doc = (await fetch(
          `./api/file?path=${encodeURIComponent(path)}`,
        ).then((r) => {
          if (!r.ok)
            return r
              .json()
              .catch(() => ({ error: `HTTP ${r.status}` }))
              .then((j) => {
                throw new Error(j.error ?? `HTTP ${r.status}`)
              })
          return r.json()
        })) as RenderedDoc
        setTabs((prev) => [...prev, docToTab(doc)])
        setActiveTabPath(doc.path)
      } catch (e) {
        setToast({ message: `打开失败：${String(e)}`, kind: 'error' })
      }
    },
    [tabs],
  )

  const requestCloseTab = useCallback(
    (path: string) => {
      const t = tabs.find((x) => x.path === path)
      if (!t) return
      if (t.dirty) {
        setClosing({ path })
        return
      }
      forceCloseTab(path)
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [tabs],
  )

  const forceCloseTab = useCallback(
    (path: string) => {
      setTabs((prev) => {
        const idx = prev.findIndex((t) => t.path === path)
        if (idx < 0) return prev
        const next = prev.filter((t) => t.path !== path)
        // 切换 active：优先右邻 → 左邻 → null
        if (activeTabPath === path) {
          const fallback = prev[idx + 1]?.path ?? prev[idx - 1]?.path ?? null
          setActiveTabPath(fallback)
        }
        return next
      })
    },
    [activeTabPath],
  )

  const saveTab = useCallback(
    async (path: string) => {
      const t = tabs.find((x) => x.path === path)
      if (!t) return
      updateTab(path, { saving: 'saving', error: undefined })
      try {
        const res = await fetch('./api/save', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ path: t.path, source: t.source }),
        })
        if (!res.ok) {
          const body = await res.json().catch(() => ({}))
          throw new Error(body.error ?? `HTTP ${res.status}`)
        }
        updateTab(path, { saving: 'idle', dirty: false, error: undefined })
        setToast({ message: '已保存', kind: 'success' })
      } catch (e) {
        updateTab(path, { saving: 'error', error: String(e) })
        setToast({ message: `保存失败：${String(e)}`, kind: 'error' })
      }
    },
    [tabs, updateTab],
  )

  const copyPath = useCallback(
    async (path: string) => {
      try {
        await navigator.clipboard.writeText(path)
        setToast({ message: '已复制路径', kind: 'success' })
      } catch (e) {
        setToast({ message: `复制失败：${String(e)}`, kind: 'error' })
      }
    },
    [],
  )

  // —— TOC ——
  const headings = useMemo(() => {
    if (!activeTab || activeTab.kind !== 'markdown' || !activeTab.doc) return []
    return extractHeadings(activeTab.doc as ParsedDocument)
  }, [activeTab])

  // —— Loading / Error 屏 ——
  if (loadState.kind === 'loading') {
    return (
      <div className="h-full flex items-center justify-center text-seeyue-fg-muted text-sm gap-2">
        <span className="inline-block w-2 h-2 rounded-full bg-seeyue-accent animate-pulse" />
        加载中…
      </div>
    )
  }
  if (loadState.kind === 'error') {
    return (
      <div className="h-full flex items-center justify-center p-8 text-seeyue-danger text-sm font-mono whitespace-pre-wrap">
        加载失败：{loadState.message}
      </div>
    )
  }

  return (
    <MarkdownBaseDirContext.Provider value={baseDir}>
      <div
        className="h-full grid bg-seeyue-bg text-seeyue-fg"
        style={{
          gridTemplateColumns: tocCollapsed
            ? '270px 1fr 28px'
            : '270px 1fr 240px',
        }}
      >
        {/* 左：文件树 */}
        <aside className="border-r border-seeyue-border overflow-hidden">
          <FileTree
            root={treeRoot}
            onChangeRoot={setTreeRoot}
            showHidden={showHidden}
            onToggleHidden={() => setShowHidden((v) => !v)}
            activePath={activeTabPath}
            onOpen={openFile}
          />
        </aside>

        {/* 中：Tab 条 + 编辑器顶栏 + 编辑区 */}
        <main className="flex flex-col overflow-hidden">
          <TabBar
            tabs={tabs}
            activePath={activeTabPath}
            onActivate={setActiveTabPath}
            onClose={requestCloseTab}
          />
          {activeTab && (
            <EditorBar
              tab={activeTab}
              onSave={() => saveTab(activeTab.path)}
              onCopyPath={() => copyPath(activeTab.path)}
            />
          )}
          <div className="flex-1 overflow-hidden">
            {activeTab ? (
              activeTab.kind === 'markdown' ? (
                <MilkdownEditor
                  key={activeTab.path}
                  tab={activeTab}
                  baseDir={baseDir}
                  onChange={(source: string) =>
                    updateTab(activeTab.path, { source, dirty: true })
                  }
                  onParsed={(doc: ParsedDocument) =>
                    updateTab(activeTab.path, { doc })
                  }
                  onSave={() => saveTab(activeTab.path)}
                />
              ) : (
                <PlainTextEditor
                  key={activeTab.path}
                  tab={activeTab}
                  onChange={(source) =>
                    updateTab(activeTab.path, { source, dirty: true })
                  }
                  onSave={() => saveTab(activeTab.path)}
                />
              )
            ) : (
              <EmptyState />
            )}
          </div>
        </main>

        {/* 右：TOC */}
        <aside className="border-l border-seeyue-border overflow-hidden">
          <TableOfContents
            headings={headings}
            collapsed={tocCollapsed}
            onToggleCollapsed={toggleToc}
          />
        </aside>

        {/* 关闭确认 */}
        {closing && (
          <CloseConfirmDialog
            filename={tabs.find((t) => t.path === closing.path)?.filename ?? ''}
            onSave={async () => {
              await saveTab(closing.path)
              setClosing(null)
              setTabs((prev) => {
                const t = prev.find((x) => x.path === closing.path)
                if (t && !t.dirty) {
                  queueMicrotask(() => forceCloseTab(closing.path))
                }
                return prev
              })
            }}
            onDiscard={() => {
              forceCloseTab(closing.path)
              setClosing(null)
            }}
            onCancel={() => setClosing(null)}
          />
        )}

        {toast && (
          <Toast
            message={toast.message}
            kind={toast.kind}
            onClose={() => setToast(null)}
          />
        )}
      </div>
    </MarkdownBaseDirContext.Provider>
  )
}

// ---------------------------------------------------------------------------
// helpers / sub-components
// ---------------------------------------------------------------------------

function docToTab(doc: RenderedDoc): Tab {
  return {
    path: doc.path,
    filename: doc.filename,
    kind:
      doc.kind === 'markdown' || doc.kind === 'plain_text'
        ? doc.kind
        : 'plain_text',
    source: doc.source,
    doc:
      doc.kind === 'markdown' && doc.payload
        ? (doc.payload as ParsedDocument)
        : null,
    dirty: false,
    saving: 'idle',
  }
}

/**
 * 编辑器顶栏：面包屑 + 状态徽章 + 保存 / 复制路径快捷按钮。
 *
 * 这块取代了「中央区只有 Tab 条」的潦草感，给用户：
 * - 当前文件的路径上下文
 * - 一眼可见的 dirty / saving 状态
 * - 不必去 menu bar 找的常用操作
 */
function EditorBar({
  tab,
  onSave,
  onCopyPath,
}: {
  tab: Tab
  onSave: () => void
  onCopyPath: () => void
}) {
  const segs = breadcrumb(tab.path)
  return (
    <div className="seeyue-editor-bar">
      <div className="breadcrumb" title={tab.path}>
        {segs.map((s, i) => (
          <span
            key={i}
            className={`crumb ${i === segs.length - 1 ? 'crumb-leaf' : ''}`}
          >
            {i > 0 && <span className="crumb-sep"> / </span>}
            {s}
          </span>
        ))}
      </div>
      {tab.dirty && (
        <span className="status-pill" data-tone="warn">
          ● 未保存
        </span>
      )}
      {tab.saving === 'saving' && (
        <span className="status-pill" data-tone="accent">
          保存中…
        </span>
      )}
      {tab.saving === 'error' && (
        <span className="status-pill" data-tone="danger" title={tab.error}>
          保存失败
        </span>
      )}
      <button className="seeyue-icon-btn" onClick={onCopyPath} title="复制路径">
        <Copy size={14} />
      </button>
      <button
        className="seeyue-icon-btn"
        onClick={onSave}
        title="保存（⌘S）"
      >
        <Save size={14} />
      </button>
    </div>
  )
}

function EmptyState() {
  return (
    <div className="seeyue-empty">
      <span className="glyph">
        <BookOpen size={36} />
      </span>
      <div className="title flex items-center gap-1.5">
        <Sparkles size={14} className="opacity-70" />
        从左侧选一个文件开始阅读
      </div>
      <div className="subtitle">
        也可以直接在终端运行 <code>j read &lt;file&gt;</code> 打开指定文件。
      </div>
      <div className="shortcuts">
        <div className="row">
          <kbd>⌘</kbd>
          <kbd>S</kbd>
          <span>保存当前文件</span>
        </div>
        <div className="row">
          <kbd>Click</kbd>
          <span>左侧目录的 dotfile 按钮可显示隐藏文件</span>
        </div>
      </div>
    </div>
  )
}

function breadcrumb(p: string): string[] {
  if (!p) return ['(empty)']
  const parts = p.split('/').filter(Boolean)
  if (parts.length <= 4) return parts
  return ['…', ...parts.slice(-3)]
}

/** 同步 document.title 与 beforeunload 拦截。 */
function useDirtyTitle(activeTab: Tab | null, anyDirty: boolean) {
  // title
  useEffect(() => {
    const base = activeTab ? `${activeTab.filename} · j reader` : 'j reader'
    document.title = (activeTab?.dirty ? '● ' : '') + base
  }, [activeTab, activeTab?.dirty])

  // beforeunload + shutdown beacon
  useEffect(() => {
    function handler(e: BeforeUnloadEvent) {
      if (anyDirty) {
        e.preventDefault()
        // Chrome 仍要求 returnValue 设值
        e.returnValue = ''
      } else {
        navigator.sendBeacon('./api/shutdown')
      }
    }
    window.addEventListener('beforeunload', handler)
    return () => window.removeEventListener('beforeunload', handler)
  }, [anyDirty])
}
