import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { FileTree } from './FileTree'
import { TabBar } from './TabBar'
import { CloseConfirmDialog } from './CloseConfirmDialog'
import { WysiwygEditor } from './wysiwyg/WysiwygEditor'
import { PlainTextEditor } from './PlainTextEditor'
import { TableOfContents, extractHeadings } from './TableOfContents'
import { Toast } from './Toast'
import { MarkdownBaseDirContext } from './MarkdownIR'
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
  const [toast, setToast] = useState<{ message: string; kind: 'error' | 'success' | 'info' } | null>(null)
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
        setToast({ message: `已打开 ${MAX_TABS} 个 Tab，关闭一些再试`, kind: 'info' })
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
      } catch (e) {
        updateTab(path, { saving: 'error', error: String(e) })
        setToast({ message: `保存失败：${String(e)}`, kind: 'error' })
      }
    },
    [tabs, updateTab],
  )

  // —— TOC ——
  const headings = useMemo(() => {
    if (!activeTab || activeTab.kind !== 'markdown' || !activeTab.doc) return []
    return extractHeadings(activeTab.doc as ParsedDocument)
  }, [activeTab])

  // —— Loading / Error 屏 ——
  if (loadState.kind === 'loading') {
    return (
      <div className="h-full flex items-center justify-center text-seeyue-fg-muted text-sm">
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
          ? '260px 1fr 24px'
          : '260px 1fr 240px',
      }}
    >
      {/* 左：文件树 */}
      <aside className="border-r border-seeyue-border bg-seeyue-sidebar overflow-hidden">
        <FileTree
          root={treeRoot}
          onChangeRoot={setTreeRoot}
          showHidden={showHidden}
          onToggleHidden={() => setShowHidden((v) => !v)}
          activePath={activeTabPath}
          onOpen={openFile}
        />
      </aside>

      {/* 中：Tab 条 + 编辑区 */}
      <main className="flex flex-col overflow-hidden">
        <TabBar
          tabs={tabs}
          activePath={activeTabPath}
          onActivate={setActiveTabPath}
          onClose={requestCloseTab}
        />
        <div className="flex-1 overflow-hidden">
          {activeTab ? (
            activeTab.kind === 'markdown' ? (
              <MarkdownLiveEditor
                key={activeTab.path}
                tab={activeTab}
                onChange={(source) =>
                  updateTab(activeTab.path, { source, dirty: true })
                }
                onParsed={(doc) => updateTab(activeTab.path, { doc })}
                onSave={() => saveTab(activeTab.path)}
                editingBlockIdx={activeTab.editingBlockIdx ?? null}
                setEditingBlockIdx={(idx) =>
                  updateTab(activeTab.path, { editingBlockIdx: idx })
                }
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
            <div className="h-full flex items-center justify-center text-seeyue-fg-dim text-sm">
              没有打开的文件，左侧选一个吧
            </div>
          )}
        </div>
      </main>

      {/* 右：TOC */}
      <aside className="border-l border-seeyue-border bg-seeyue-sidebar overflow-y-auto">
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
            // 仅在确实已保存时才关
            const latest = activeTabRef.current // 仅作占位，真正要看 tabs
            void latest
            setClosing(null)
            // 用 setState 闭包内最新 tabs 判断
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
// helpers
// ---------------------------------------------------------------------------

function docToTab(doc: RenderedDoc): Tab {
  return {
    path: doc.path,
    filename: doc.filename,
    kind: doc.kind === 'markdown' || doc.kind === 'plain_text' ? doc.kind : 'plain_text',
    source: doc.source,
    doc:
      doc.kind === 'markdown' && doc.payload ? (doc.payload as ParsedDocument) : null,
    dirty: false,
    saving: 'idle',
  }
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
