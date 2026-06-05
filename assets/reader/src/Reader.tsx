import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { ActivityBar, type ActivityKey } from './ActivityBar'
import { FileTree } from './FileTree'
import { Toolbox } from './Toolbox'
import { DiffTool } from './DiffTool'
import { JsonTool } from './JsonTool'
import { TabBar } from './TabBar'
import { CloseConfirmDialog } from './CloseConfirmDialog'
import { QuitConfirmDialog } from './QuitConfirmDialog'
import { MarkdownEditor } from './editor/MarkdownEditor'
import { PlainTextEditor } from './PlainTextEditor'
import { ImageViewer } from './ImageViewer'
import { TableOfContents } from './TableOfContents'
import { extractHeadings } from './toc'
import { Toast } from './Toast'
import { VerticalSplitter } from './Splitter'
import { MarkdownBaseDirContext } from './MarkdownIR'
import { BookOpen, Copy, Save, Sparkles } from './Icon'
import type { ImagePayload, InitialResp, ParsedDocument, RenderedDoc, Tab, ToolId } from './types'

type LoadState = { kind: 'loading' } | { kind: 'error'; message: string } | { kind: 'ready' }

/** 同时打开新文件的并发上限（防误点把内存撑爆） */
const MAX_TABS = 32

/** 工具 tab 标签上显示的名字。新增工具时在这里加一项。 */
const TOOL_TITLES: Record<ToolId, string> = {
  diff: '文本 Diff',
  json: 'JSON 查看器',
}

/** 侧栏宽度（活动栏右侧的 FileTree / Toolbox 面板） */
const SIDEBAR_DEFAULT = 270
const SIDEBAR_MIN = 180
const SIDEBAR_MAX = 560
const SIDEBAR_LS_KEY = 'jreader.sidebarWidth'

export function Reader() {
  const [loadState, setLoadState] = useState<LoadState>({ kind: 'loading' })
  const [tabs, setTabs] = useState<Tab[]>([])
  const [activeTabPath, setActiveTabPath] = useState<string | null>(null)
  const [treeRoot, setTreeRoot] = useState<string>('')
  const [showHidden, setShowHidden] = useState(false)
  /**
   * 左侧"活动栏"当前选中的视图。文件 / 工具箱二选一。
   * 持久化到 localStorage —— 用户上次留在工具箱，下次打开还是工具箱。
   */
  const [activeActivity, setActiveActivity] = useState<ActivityKey>(() => {
    const v = localStorage.getItem('jreader.activity')
    return v === 'toolbox' ? 'toolbox' : 'files'
  })
  /** 关闭 dirty Tab 时弹出三选项确认 */
  const [closing, setClosing] = useState<{ path: string } | null>(null)
  /**
   * 关闭整个 reader 前的二次确认。痛点：dirty tab 弹窗里连按"不保存"，
   * 最后一下落到空态再触发 ⌘W 时会直接 quitReader 关掉窗口，用户措手不及。
   * 现在所有触发 quitReader 的入口（⌘W 空态 + 顶栏 Power 按钮）都先打开这个 modal。
   */
  const [quitting, setQuitting] = useState(false)
  /** 错误 / 成功提示（替代 alert） */
  const [toast, setToast] = useState<{
    message: string
    kind: 'error' | 'success' | 'info'
  } | null>(null)
  /** TOC 折叠态，持久化到 localStorage */
  const [tocCollapsed, setTocCollapsed] = useState<boolean>(() => {
    return localStorage.getItem('jreader.tocCollapsed') === '1'
  })
  /**
   * 侧栏宽度（活动栏右侧那一列）—— 用户可拖动调节，持久化到 localStorage。
   * 初值会被夹紧到 [SIDEBAR_MIN, SIDEBAR_MAX]，防止上一次恶意 / 意外的 0 值。
   */
  const [sidebarWidth, setSidebarWidth] = useState<number>(() => {
    const raw = localStorage.getItem(SIDEBAR_LS_KEY)
    const n = raw ? Number(raw) : SIDEBAR_DEFAULT
    if (!Number.isFinite(n)) return SIDEBAR_DEFAULT
    return Math.max(SIDEBAR_MIN, Math.min(SIDEBAR_MAX, n))
  })
  const handleSidebarResize = useCallback((next: number) => {
    setSidebarWidth(next)
    localStorage.setItem(SIDEBAR_LS_KEY, String(Math.round(next)))
  }, [])
  /** 监听 doc 更新（每次 /api/parse 完成就 +1）—— 给 TOC 触发重算 */
  const [docVersion, setDocVersion] = useState(0)

  // —— 高频内容用 ref 而不是 state，避免按键触发整树 re-render ——
  /** 每个 tab 的最新文本内容；按 path 索引 */
  const sourcesRef = useRef<Record<string, string>>({})
  /** 每个文件的原始内容（打开/保存时快照），用于 dirty 判断 */
  const originalSourcesRef = useRef<Record<string, string>>({})
  /** 每个 markdown tab 的最新 IR；按 path 索引 */
  const docsRef = useRef<Record<string, ParsedDocument>>({})
  /** 每个 image tab 的元信息（mime / size）；按 path 索引 */
  const imagesRef = useRef<Record<string, ImagePayload>>({})

  const toggleToc = useCallback(() => {
    setTocCollapsed((prev) => {
      const next = !prev
      localStorage.setItem('jreader.tocCollapsed', next ? '1' : '0')
      return next
    })
  }, [])

  const selectActivity = useCallback((key: ActivityKey) => {
    setActiveActivity(key)
    localStorage.setItem('jreader.activity', key)
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
          `./api/file?path=${encodeURIComponent(initial.initial_path)}`
        ).then((r) => {
          if (!r.ok) throw new Error(`file HTTP ${r.status}`)
          return r.json()
        })) as RenderedDoc
        if (cancelled) return

        ingestDoc(doc, sourcesRef, docsRef, imagesRef, originalSourcesRef)
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
      void fetch('./api/heartbeat', { method: 'POST', keepalive: true }).catch(() => {})
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
    [tabs, activeTabPath]
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
    setTabs((prev) => prev.map((t) => (t.path === path ? { ...t, ...patch } : t)))
  }, [])

  /**
   * 编辑器报告 source 变化。这是高频回调（按键级别）。
   * 只把内容写进 ref；只有 dirty 翻转时才碰 setState。
   * 对比原始 source 判断是否真的脏了：内容恢复原样时自动清除 dirty。
   */
  const handleSourceChange = useCallback((path: string, source: string) => {
    sourcesRef.current[path] = source
    const isDirty = source !== (originalSourcesRef.current[path] ?? '')
    setTabs((prev) => {
      const t = prev.find((x) => x.path === path)
      if (!t) return prev
      if (t.dirty === isDirty) return prev // 状态没变化，不 setState
      return prev.map((x) => (x.path === path ? { ...x, dirty: isDirty } : x))
    })
  }, [])

  /**
   * MilkdownEditor 内部 debounce 调 /api/parse 后回调，更新 IR。
   * 同样写 ref，再 bump docVersion 触发 TOC 重算（TOC 依赖 path + version）。
   */
  const handleDocParsed = useCallback((path: string, doc: ParsedDocument) => {
    docsRef.current[path] = doc
    setDocVersion((v) => v + 1)
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
        const doc = (await fetch(`./api/file?path=${encodeURIComponent(path)}`).then((r) => {
          if (!r.ok)
            return r
              .json()
              .catch(() => ({ error: `HTTP ${r.status}` }))
              .then((j) => {
                throw new Error(j.error ?? `HTTP ${r.status}`)
              })
          return r.json()
        })) as RenderedDoc
        ingestDoc(doc, sourcesRef, docsRef, imagesRef, originalSourcesRef)
        setTabs((prev) => [...prev, docToTab(doc)])
        setActiveTabPath(doc.path)
      } catch (e) {
        setToast({ message: `打开失败：${String(e)}`, kind: 'error' })
      }
    },
    [tabs]
  )

  /**
   * 打开一个内置工具作为 tab。每个工具同时只允许一个实例 —— 二次点击只是切回去。
   * 工具 tab 的 path 用伪 URI（`tool://<id>`）作为唯一 key，跟磁盘文件路径
   * 永远不会冲突。
   */
  const openTool = useCallback(
    (toolId: ToolId) => {
      const path = `tool://${toolId}`
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
      const tab: Tab = {
        path,
        filename: TOOL_TITLES[toolId] ?? toolId,
        kind: 'tool',
        toolId,
        dirty: false,
        saving: 'idle',
      }
      setTabs((prev) => [...prev, tab])
      setActiveTabPath(path)
    },
    [tabs]
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
    [tabs]
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
      delete imagesRef.current[path]
    },
    [activeTabPath]
  )

  const saveTab = useCallback(
    async (path: string) => {
      const t = tabs.find((x) => x.path === path)
      if (!t) return
      // 图片是只读视图：⌘S 直接 no-op，避免把空 source 覆盖回去毁掉文件
      // 工具 tab 没有"文件内容"概念，⌘S 同样直接忽略
      if (t.kind === 'image' || t.kind === 'tool') return
      // CM6 文档即源码本身，不再需要 preserveBlankLines 那一套"忠实重建"
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
        sourcesRef.current[path] = source
        originalSourcesRef.current[path] = source
        updateTab(path, { saving: 'idle', dirty: false, error: undefined })
        setToast({ message: '已保存', kind: 'success' })
      } catch (e) {
        updateTab(path, { saving: 'error', error: String(e) })
        setToast({ message: `保存失败：${String(e)}`, kind: 'error' })
      }
    },
    [tabs, updateTab]
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
   * 调 /api/create 在指定父目录里创建一个新文件，成功返回新文件绝对路径。
   * 失败时把后端 message 透给上层，让 PromptDialog 弹红字让用户改名重试。
   */
  const createFile = useCallback(async (dir: string, name: string): Promise<string> => {
    const res = await fetch('./api/create', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ dir, name }),
    })
    if (!res.ok) {
      const body = await res.json().catch(() => ({}))
      throw new Error(body.error ?? `HTTP ${res.status}`)
    }
    const data = (await res.json()) as { path: string }
    return data.path
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
          '<div style="height:100%;display:flex;align-items:center;justify-content:center;color:#8a7e72;font-family:system-ui;font-size:14px;">reader 已退出，可以关闭此页面</div>'
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
    [activeTabPath]
  )

  // —— 全局快捷键：(⌘|⌃) S / W / ⌥← / ⌥→ / 1 / 2 ——
  // 同时接受 metaKey（macOS ⌘）与 ctrlKey（macOS ⌃ 或 Win/Linux Ctrl）。
  // 这是因为普通 Chrome 标签页里 ⌘W 会被浏览器吞掉关掉标签，根本传不到 JS；
  // 此时用户可以退而用 ⌃W 关闭当前 reader tab。
  // app 模式（`j read` 默认走 Chrome --app=URL）无标签栏，⌘W 才能被网页接收。
  // 用 ref 保存最新引用，避免每次 tabs 变化都重绑 listener
  const requestQuit = useCallback(() => setQuitting(true), [])
  const handlersRef = useRef({
    saveTab,
    requestCloseTab,
    cycleTab,
    activeTabPath,
    requestQuit,
    selectActivity,
  })
  handlersRef.current = {
    saveTab,
    requestCloseTab,
    cycleTab,
    activeTabPath,
    requestQuit,
    selectActivity,
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
      // ⌘W 关 tab；没有 tab 时弹关闭确认（不直接关窗，防误触）
      if (!e.shiftKey && k === 'w') {
        e.preventDefault()
        const p = handlersRef.current.activeTabPath
        if (p) {
          handlersRef.current.requestCloseTab(p)
        } else {
          handlersRef.current.requestQuit()
        }
        return
      }
      // VS Code 风格：⌘⌥← / ⌘⌥→ 切 tab（Win/Linux 用 Ctrl+Alt+←/→）
      if (e.altKey && !e.shiftKey && (e.key === 'ArrowLeft' || e.key === 'ArrowRight')) {
        e.preventDefault()
        handlersRef.current.cycleTab(e.key === 'ArrowLeft' ? -1 : 1)
        return
      }
      // ⌘1 / ⌘2 切活动栏（VSCode 习惯）
      if (!e.shiftKey && (k === '1' || k === '2')) {
        e.preventDefault()
        handlersRef.current.selectActivity(k === '1' ? 'files' : 'toolbox')
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

  /** 当前激活 tab 是否是工具 tab —— 工具 tab 没有 TOC，把右栏整列收掉 */
  const showToc = activeTab?.kind === 'markdown'

  /** 当前活跃工具 id（用于 Toolbox 高亮选中态） */
  const activeToolId = activeTab?.kind === 'tool' ? (activeTab.toolId ?? null) : null

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
          // 4 列：[44px 活动栏] [{sidebarWidth}px 侧栏面板] [5px 分割条] [1fr 主区]
          gridTemplateColumns: `44px ${sidebarWidth}px 5px 1fr`,
        }}
      >
        {/* 最左：垂直活动栏 */}
        <ActivityBar active={activeActivity} onSelect={selectActivity} />

        {/* 左：侧栏（按 activeActivity 切换内容） */}
        <aside className="overflow-hidden">
          {activeActivity === 'files' ? (
            <FileTree
              root={treeRoot}
              onChangeRoot={setTreeRoot}
              showHidden={showHidden}
              onToggleHidden={() => setShowHidden((v) => !v)}
              activePath={activeTabPath}
              onOpen={openFile}
              onCreateFile={createFile}
            />
          ) : (
            <Toolbox activeToolId={activeToolId} onOpen={openTool} />
          )}
        </aside>

        {/* 侧栏宽度调节 splitter */}
        <VerticalSplitter
          width={sidebarWidth}
          min={SIDEBAR_MIN}
          max={SIDEBAR_MAX}
          defaultWidth={SIDEBAR_DEFAULT}
          onResize={handleSidebarResize}
          ariaLabel="调节侧栏宽度"
        />

        {/* 中：Tab 条 + 编辑器顶栏 + 编辑区（TOC 浮于其上） */}
        <main className="flex flex-col overflow-hidden relative">
          <TabBar
            tabs={tabs}
            activePath={activeTabPath}
            onActivate={setActiveTabPath}
            onClose={requestCloseTab}
            onQuit={requestQuit}
          />
          {activeTab && activeTab.kind !== 'tool' && (
            <EditorBar
              tab={activeTab}
              onSave={() => saveTab(activeTab.path)}
              onCopyPath={() => copyPath(activeTab.path)}
            />
          )}
          <div className="flex-1 overflow-hidden relative">
            {activeTab ? (
              activeTab.kind === 'tool' ? (
                <ToolHost toolId={activeTab.toolId ?? null} />
              ) : activeTab.kind === 'markdown' ? (
                <MarkdownEditor
                  key={activeTab.path}
                  path={activeTab.path}
                  baseDir={baseDir}
                  initialSource={sourcesRef.current[activeTab.path] ?? ''}
                  initialDoc={docsRef.current[activeTab.path] ?? { blocks: [] }}
                  onChange={handleSourceChange}
                  onParsed={handleDocParsed}
                  onSave={() => saveTab(activeTab.path)}
                />
              ) : activeTab.kind === 'image' ? (
                <ImageViewer
                  key={activeTab.path}
                  path={activeTab.path}
                  filename={activeTab.filename}
                  payload={imagesRef.current[activeTab.path] ?? null}
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
            {/* TOC：浮于内容区右侧，自然融入 */}
            {showToc && (
              <TableOfContents
                headings={headings}
                collapsed={tocCollapsed}
                onToggleCollapsed={toggleToc}
              />
            )}
          </div>
        </main>

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

        {quitting && (
          <QuitConfirmDialog
            dirtyCount={tabs.filter((t) => t.dirty).length}
            onConfirm={() => {
              setQuitting(false)
              quitReader()
            }}
            onCancel={() => setQuitting(false)}
          />
        )}

        {toast && (
          <Toast message={toast.message} kind={toast.kind} onClose={() => setToast(null)} />
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
      doc.kind === 'markdown' || doc.kind === 'plain_text' || doc.kind === 'image'
        ? doc.kind
        : 'plain_text',
    dirty: false,
    saving: 'idle',
  }
}

/**
 * 工具 tab 渲染入口。按 toolId 分发到具体工具组件。
 * 列出来 + 路由集中在这里，新增工具时只在这里加 case，Reader 主流程不动。
 */
function ToolHost({ toolId }: { toolId: ToolId | null }) {
  switch (toolId) {
    case 'diff':
      return <DiffTool />
    case 'json':
      return <JsonTool />
    default:
      return (
        <div className="h-full flex flex-col items-center justify-center p-12 text-seeyue-fg-dim text-[13px] text-center">
          <div className="text-seeyue-fg text-[15px] font-medium mb-1.5">未知工具</div>
          <div className="text-seeyue-fg-muted mb-4 leading-[1.7]">toolId = {String(toolId)}</div>
        </div>
      )
  }
}

/**
 * 把一份 RenderedDoc 拆进 sources / docs / images 三个 ref 桶。
 * 与 docToTab 配套使用。
 *
 * sourcesRef：编辑器当前内容（会随按键更新）
 */
function ingestDoc(
  doc: RenderedDoc,
  sourcesRef: React.RefObject<Record<string, string>>,
  docsRef: React.RefObject<Record<string, ParsedDocument>>,
  imagesRef: React.RefObject<Record<string, ImagePayload>>,
  originalSourcesRef: React.RefObject<Record<string, string>>
) {
  sourcesRef.current![doc.path] = doc.source
  originalSourcesRef.current![doc.path] = doc.source
  if (doc.kind === 'markdown' && doc.payload) {
    docsRef.current![doc.path] = doc.payload as ParsedDocument
  } else if (doc.kind === 'image' && doc.payload) {
    imagesRef.current![doc.path] = doc.payload as ImagePayload
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
  // 图片是只读视图：不显示 dirty / 保存按钮（一旦触发 /api/save 会用空 source 覆盖原文件）
  const editable = tab.kind !== 'image'
  return (
    <div className="flex items-center h-8 px-3 bg-seeyue-bg text-xs text-seeyue-fg-dim gap-2">
      <div className="flex-1 min-w-0 flex items-center gap-1 overflow-hidden" title={tab.path}>
        {segs.map((s, i) => (
          <span
            key={i}
            className={`whitespace-nowrap overflow-hidden text-ellipsis ${i === segs.length - 1 ? 'text-seeyue-fg' : ''}`}
          >
            {i > 0 && <span className="opacity-50"> / </span>}
            {s}
          </span>
        ))}
      </div>
      {editable && tab.dirty && (
        <span className="inline-flex items-center gap-1 text-[11px] px-2 py-0.5 rounded-full bg-[rgba(208,135,112,0.18)] text-seeyue-warn">
          ● 未保存
        </span>
      )}
      {editable && tab.saving === 'saving' && (
        <span className="inline-flex items-center gap-1 text-[11px] px-2 py-0.5 rounded-full bg-seeyue-accent-mute text-seeyue-accent">
          保存中…
        </span>
      )}
      {editable && tab.saving === 'error' && (
        <span
          className="inline-flex items-center gap-1 text-[11px] px-2 py-0.5 rounded-full bg-[rgba(191,97,106,0.18)] text-seeyue-danger"
          title={tab.error}
        >
          保存失败
        </span>
      )}
      <button
        className="inline-flex items-center justify-center w-[26px] h-[26px] rounded-md text-seeyue-fg-dim bg-transparent border-0 cursor-pointer transition-all duration-150 hover:text-seeyue-fg-strong hover:bg-seeyue-elevated disabled:opacity-30 disabled:cursor-not-allowed"
        onClick={onCopyPath}
        title="复制路径"
      >
        <Copy size={14} />
      </button>
      {editable && (
        <button
          className="inline-flex items-center justify-center w-[26px] h-[26px] rounded-md text-seeyue-fg-dim bg-transparent border-0 cursor-pointer transition-all duration-150 hover:text-seeyue-fg-strong hover:bg-seeyue-elevated disabled:opacity-30 disabled:cursor-not-allowed"
          onClick={onSave}
          title="保存（⌘S）"
        >
          <Save size={14} />
        </button>
      )}
    </div>
  )
}

function EmptyState() {
  return (
    <div className="h-full flex flex-col items-center justify-center p-12 text-seeyue-fg-dim text-[13px] text-center">
      <span className="flex items-center justify-center w-[84px] h-[84px] rounded-full bg-[linear-gradient(135deg,rgba(94,129,172,0.12),rgba(163,190,140,0.08))] text-seeyue-accent mb-[18px] shadow-[inset_0_0_0_1px_rgba(94,129,172,0.25)]">
        <BookOpen size={36} />
      </span>
      <div className="text-seeyue-fg text-[15px] font-medium mb-1.5 flex items-center gap-1.5">
        <Sparkles size={14} className="opacity-70" />
        从左侧选一个文件开始阅读
      </div>
      <div className="text-seeyue-fg-muted mb-[18px] leading-[1.7]">
        也可以直接在终端运行 <code>j read &lt;file&gt;</code> 打开指定文件。
      </div>
      <div className="grid gap-1.5 text-xs text-seeyue-fg-dim">
        <div className="flex items-center gap-2 justify-center">
          <kbd className="inline-flex items-center justify-center min-w-[22px] h-[22px] px-1.5 font-mono text-[11px] text-seeyue-fg bg-seeyue-elevated border border-seeyue-border-strong border-b-2 rounded">
            ⌘
          </kbd>
          <kbd className="inline-flex items-center justify-center min-w-[22px] h-[22px] px-1.5 font-mono text-[11px] text-seeyue-fg bg-seeyue-elevated border border-seeyue-border-strong border-b-2 rounded">
            S
          </kbd>
          <span>保存当前文件</span>
        </div>
        <div className="flex items-center gap-2 justify-center">
          <kbd className="inline-flex items-center justify-center min-w-[22px] h-[22px] px-1.5 font-mono text-[11px] text-seeyue-fg bg-seeyue-elevated border border-seeyue-border-strong border-b-2 rounded">
            ⌘
          </kbd>
          <kbd className="inline-flex items-center justify-center min-w-[22px] h-[22px] px-1.5 font-mono text-[11px] text-seeyue-fg bg-seeyue-elevated border border-seeyue-border-strong border-b-2 rounded">
            W
          </kbd>
          <span>/</span>
          <kbd className="inline-flex items-center justify-center min-w-[22px] h-[22px] px-1.5 font-mono text-[11px] text-seeyue-fg bg-seeyue-elevated border border-seeyue-border-strong border-b-2 rounded">
            ⌃
          </kbd>
          <kbd className="inline-flex items-center justify-center min-w-[22px] h-[22px] px-1.5 font-mono text-[11px] text-seeyue-fg bg-seeyue-elevated border border-seeyue-border-strong border-b-2 rounded">
            W
          </kbd>
          <span>关闭当前 Tab；空时退出 reader</span>
        </div>
        <div className="flex items-center gap-2 justify-center">
          <kbd className="inline-flex items-center justify-center min-w-[22px] h-[22px] px-1.5 font-mono text-[11px] text-seeyue-fg bg-seeyue-elevated border border-seeyue-border-strong border-b-2 rounded">
            ⌘
          </kbd>
          <kbd className="inline-flex items-center justify-center min-w-[22px] h-[22px] px-1.5 font-mono text-[11px] text-seeyue-fg bg-seeyue-elevated border border-seeyue-border-strong border-b-2 rounded">
            ⌥
          </kbd>
          <kbd className="inline-flex items-center justify-center min-w-[22px] h-[22px] px-1.5 font-mono text-[11px] text-seeyue-fg bg-seeyue-elevated border border-seeyue-border-strong border-b-2 rounded">
            ←
          </kbd>
          <span>/</span>
          <kbd className="inline-flex items-center justify-center min-w-[22px] h-[22px] px-1.5 font-mono text-[11px] text-seeyue-fg bg-seeyue-elevated border border-seeyue-border-strong border-b-2 rounded">
            →
          </kbd>
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
