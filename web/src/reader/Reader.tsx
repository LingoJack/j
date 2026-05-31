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
  /** 监听 doc 更新（每次 /api/parse 完成就 +1）—— 给 TOC 触发重算 */
  const [docVersion, setDocVersion] = useState(0)

  // —— 高频内容用 ref 而不是 state，避免按键触发整树 re-render ——
  /** 每个 tab 的最新文本内容；按 path 索引 */
  const sourcesRef = useRef<Record<string, string>>({})
  /** 每个 markdown tab 的最新 IR；按 path 索引 */
  const docsRef = useRef<Record<string, ParsedDocument>>({})

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

        ingestDoc(doc, sourcesRef, docsRef)
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

  // —— 心跳：每 5 秒发一次，让服务端确认页面还活着 ——
  // 浏览器窗口异常关闭（电源、强制退出、app 模式 ⌘W 偶发）时
  // beforeunload / sendBeacon 不一定会触发；服务端心跳超时（30s 没收到
  // 就自动 shutdown）是兜底。
  useEffect(() => {
    let cancelled = false
    const tick = () => {
      if (cancelled) return
      void fetch('./api/heartbeat', { method: 'POST', keepalive: true }).catch(
        () => {},
      )
    }
    tick() // 启动时先打一次，避免 60s 宽限期被白白消耗
    const id = window.setInterval(tick, 5000)
    return () => {
      cancelled = true
      window.clearInterval(id)
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

  // —— Tab 操作 ——
  const updateTab = useCallback((path: string, patch: Partial<Tab>) => {
    setTabs((prev) =>
      prev.map((t) => (t.path === path ? { ...t, ...patch } : t)),
    )
  }, [])

  /**
   * 编辑器报告 source 变化。这是高频回调（按键级别）。
   * 只把内容写进 ref；只有第一次 dirty 翻转时才碰 setState。
   */
  const handleSourceChange = useCallback(
    (path: string, source: string) => {
      sourcesRef.current[path] = source
      setTabs((prev) => {
        const t = prev.find((x) => x.path === path)
        if (!t || t.dirty) return prev // 已经 dirty，不再 setState
        return prev.map((x) => (x.path === path ? { ...x, dirty: true } : x))
      })
    },
    [],
  )

  /**
   * MilkdownEditor 内部 debounce 调 /api/parse 后回调，更新 IR。
   * 同样写 ref，再 bump docVersion 触发 TOC 重算（TOC 依赖 path + version）。
   */
  const handleDocParsed = useCallback(
    (path: string, doc: ParsedDocument) => {
      docsRef.current[path] = doc
      setDocVersion((v) => v + 1)
    },
    [],
  )

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
        ingestDoc(doc, sourcesRef, docsRef)
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
      // 清掉 ref 桶里的内容，不让已关 tab 占内存
      delete sourcesRef.current[path]
      delete docsRef.current[path]
    },
    [activeTabPath],
  )

  const saveTab = useCallback(
    async (path: string) => {
      const t = tabs.find((x) => x.path === path)
      if (!t) return
      const source = sourcesRef.current[path] ?? ''
      updateTab(path, { saving: 'saving', error: undefined })
      try {
        const res = await fetch('./api/save', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ path: t.path, source }),
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

  const copyPath = useCallback(async (path: string) => {
    try {
      await navigator.clipboard.writeText(path)
      setToast({ message: '已复制路径', kind: 'success' })
    } catch (e) {
      setToast({ message: `复制失败：${String(e)}`, kind: 'error' })
    }
  }, [])

  /**
   * 关掉整个 reader：通知服务端 shutdown + 关闭浏览器窗口。
   *
   * - app 模式（`j read` 默认）：window.close() 真的会关掉那个窗口
   * - 普通标签页：window.close() 只对脚本打开的窗口生效，所以兜底
   *   呈现一个 "已断开" 占位页 —— 服务端那边已经收到 shutdown 信号、
   *   cli 也会退出，用户手动关标签即可
   */
  const quitReader = useCallback(() => {
    // 用 keepalive 而非 sendBeacon，让服务端有更高概率收到（sendBeacon
    // 在某些 Chrome 版本里在 window.close 之前已经被 cancel）
    try {
      void fetch('./api/shutdown', { method: 'POST', keepalive: true })
    } catch {
      /* 忽略 */
    }
    // 给请求几十毫秒发出去再关窗口
    window.setTimeout(() => {
      window.close()
      // window.close() 在用户直接打开的标签里被忽略 —— 兜底改个占位
      document.title = 'reader 已退出'
      const root = document.getElementById('reader-root')
      if (root) {
        root.innerHTML =
          '<div style="height:100%;display:flex;align-items:center;justify-content:center;color:#88c0d0;font-family:system-ui;font-size:14px;">📖 reader 已退出，可以关闭此页面</div>'
      }
    }, 80)
  }, [])

  /** 切到相对当前 active 的下一个/上一个 tab；环形 */
  const cycleTab = useCallback(
    (delta: 1 | -1) => {
      setTabs((prev) => {
        if (prev.length === 0) return prev
        const idx = prev.findIndex((t) => t.path === activeTabPath)
        const baseIdx = idx < 0 ? 0 : idx
        const next = (baseIdx + delta + prev.length) % prev.length
        setActiveTabPath(prev[next].path)
        return prev
      })
    },
    [activeTabPath],
  )

  // —— 全局快捷键：(⌘|⌃) S / W / ⇧← / ⇧→ ——
  // 同时接受 metaKey（macOS ⌘）与 ctrlKey（macOS ⌃ 或 Win/Linux Ctrl）。
  // 这是因为普通 Chrome 标签页里 ⌘W 会被浏览器吞掉关掉标签，根本传不到 JS；
  // 此时用户可以退而用 ⌃W 关闭当前 reader tab。
  // app 模式（`j read` 默认走 Chrome --app=URL）无标签栏，⌘W 才能被网页接收。
  // 用 ref 保存最新引用，避免每次 tabs 变化都重绑 listener
  const handlersRef = useRef({
    saveTab,
    requestCloseTab,
    cycleTab,
    activeTabPath,
    quitReader,
  })
  handlersRef.current = {
    saveTab,
    requestCloseTab,
    cycleTab,
    activeTabPath,
    quitReader,
  }
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      const mod = e.metaKey || e.ctrlKey
      if (!mod) return
      const k = e.key.toLowerCase()

      // ⌘S 保存
      if (!e.shiftKey && k === 's') {
        e.preventDefault()
        const p = handlersRef.current.activeTabPath
        if (p) void handlersRef.current.saveTab(p)
        return
      }
      // ⌘W 关 tab；没有 tab 时关掉整个 reader
      if (!e.shiftKey && k === 'w') {
        e.preventDefault()
        const p = handlersRef.current.activeTabPath
        if (p) {
          handlersRef.current.requestCloseTab(p)
        } else {
          handlersRef.current.quitReader()
        }
        return
      }
      // ⌘⇧← / ⌘⇧→ 切 tab
      if (e.shiftKey && (e.key === 'ArrowLeft' || e.key === 'ArrowRight')) {
        e.preventDefault()
        handlersRef.current.cycleTab(e.key === 'ArrowLeft' ? -1 : 1)
        return
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [])

  // —— TOC ——
  // docsRef 是 ref（变化不触发渲染），所以 deps 用 path + docVersion
  const headings = useMemo(() => {
    if (!activeTab || activeTab.kind !== 'markdown') return []
    const doc = docsRef.current[activeTab.path]
    if (!doc) return []
    return extractHeadings(doc)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeTab?.path, docVersion])

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
                  path={activeTab.path}
                  baseDir={baseDir}
                  initialSource={sourcesRef.current[activeTab.path] ?? ''}
                  onChange={handleSourceChange}
                  onParsed={handleDocParsed}
                  onSave={() => saveTab(activeTab.path)}
                />
              ) : (
                <PlainTextEditor
                  key={activeTab.path}
                  path={activeTab.path}
                  initialSource={sourcesRef.current[activeTab.path] ?? ''}
                  onChange={handleSourceChange}
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
    dirty: false,
    saving: 'idle',
  }
}

/**
 * 把一份 RenderedDoc 拆进 sources / docs 两个 ref 桶。
 * 与 docToTab 配套使用。
 */
function ingestDoc(
  doc: RenderedDoc,
  sourcesRef: React.RefObject<Record<string, string>>,
  docsRef: React.RefObject<Record<string, ParsedDocument>>,
) {
  sourcesRef.current![doc.path] = doc.source
  if (doc.kind === 'markdown' && doc.payload) {
    docsRef.current![doc.path] = doc.payload as ParsedDocument
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
          <kbd>⌘</kbd>
          <kbd>W</kbd>
          <span>/</span>
          <kbd>⌃</kbd>
          <kbd>W</kbd>
          <span>关闭当前 Tab；空时退出 reader</span>
        </div>
        <div className="row">
          <kbd>⌘</kbd>
          <kbd>⇧</kbd>
          <kbd>←</kbd>
          <span>/</span>
          <kbd>→</kbd>
          <span>切换前后 Tab</span>
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
        // sendBeacon 在某些 Chrome 版本（特别是 app 模式关窗口时）会被
        // cancel；用 keepalive fetch 替代，配合服务端心跳超时双保险。
        try {
          void fetch('./api/shutdown', {
            method: 'POST',
            keepalive: true,
          })
        } catch {
          /* 忽略 */
        }
        // 兜底：旧浏览器不支持 keepalive 时退化到 sendBeacon
        if (typeof navigator.sendBeacon === 'function') {
          navigator.sendBeacon('./api/shutdown')
        }
      }
    }
    window.addEventListener('beforeunload', handler)
    // pagehide 在 bfcache 关页时仍会 fire，比 beforeunload 更可靠
    window.addEventListener('pagehide', () => {
      if (!anyDirty) {
        try {
          navigator.sendBeacon?.('./api/shutdown')
        } catch {
          /* 忽略 */
        }
      }
    })
    return () => window.removeEventListener('beforeunload', handler)
  }, [anyDirty])
}
