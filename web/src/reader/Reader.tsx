import { useEffect, useMemo, useState } from 'react'
import { MarkdownIR } from './MarkdownIR'
import { TableOfContents, extractHeadings } from './TableOfContents'
import type { ParsedDocument, RenderedDoc } from './types'

type LoadState =
  | { kind: 'loading' }
  | { kind: 'error'; message: string }
  | { kind: 'ready'; doc: RenderedDoc }

export function Reader() {
  const [state, setState] = useState<LoadState>({ kind: 'loading' })

  // 所有 hooks 必须在条件 return 之前调用（React Hooks 规则）
  const docKind = state.kind === 'ready' ? state.doc.kind : null
  const docPayload = state.kind === 'ready' ? state.doc.payload : null

  const headings = useMemo(() => {
    if (docKind !== 'markdown' || !docPayload) return []
    return extractHeadings(docPayload as ParsedDocument)
  }, [docKind, docPayload])

  useEffect(() => {
    let cancelled = false
    fetch('./api/doc')
      .then(async (res) => {
        if (!res.ok) {
          throw new Error(`HTTP ${res.status}: ${await res.text()}`)
        }
        return (await res.json()) as RenderedDoc
      })
      .then((doc) => {
        if (!cancelled) setState({ kind: 'ready', doc })
      })
      .catch((err) => {
        if (!cancelled) setState({ kind: 'error', message: String(err) })
      })
    return () => {
      cancelled = true
    }
  }, [])

  // 页面关闭时通知后端 shutdown
  useEffect(() => {
    const handleUnload = () => {
      navigator.sendBeacon('/api/shutdown')
    }
    window.addEventListener('beforeunload', handleUnload)
    return () => window.removeEventListener('beforeunload', handleUnload)
  }, [])

  if (state.kind === 'loading') {
    return (
      <div className="min-h-screen bg-[#faf9f6] text-stone-500 flex items-center justify-center text-sm tracking-wide">
        加载中…
      </div>
    )
  }

  if (state.kind === 'error') {
    return (
      <div className="min-h-screen bg-[#faf9f6] text-red-600 flex items-center justify-center p-8 text-sm font-mono whitespace-pre-wrap">
        加载失败：{state.message}
      </div>
    )
  }

  const { filename, kind, payload } = state.doc
  const hasToc = headings.length > 0

  return (
    <div className="min-h-screen bg-[#faf9f6] text-stone-800">
      <header className="fixed top-0 left-0 right-0 z-30 bg-[#faf9f6]/90 backdrop-blur-sm border-b border-stone-200/60">
        <div className="px-6 py-3.5 flex items-center justify-between max-w-[1400px] mx-auto">
          <div className="flex items-center gap-3 min-w-0">
            <span className="text-xl font-bold text-stone-900 leading-none">j</span>
            <span className="text-stone-300">/</span>
            <span className="text-sm text-stone-700 truncate">{filename}</span>
            <span className="text-[10px] px-1.5 py-0.5 rounded bg-stone-100 text-stone-500 uppercase tracking-wider font-medium">
              {kind}
            </span>
          </div>
          <span className="text-xs text-stone-400 hidden sm:inline">
            关闭页面将自动停止服务
          </span>
        </div>
      </header>
      <main className="max-w-3xl mx-auto px-6 pt-24 pb-24">
        {renderPayload(kind, payload)}
      </main>
      {hasToc && (
        <div className="hidden lg:block fixed right-0 top-16 bottom-0 z-20">
          <TableOfContents headings={headings} />
        </div>
      )}
    </div>
  )
}

function renderPayload(kind: RenderedDoc['kind'], payload: unknown): React.ReactNode {
  if (kind === 'markdown') {
    return <MarkdownIR doc={payload as ParsedDocument} />
  }
  if (kind === 'plain_text') {
    const text = (payload as { text: string }).text
    return (
      <pre className="text-[13.5px] font-mono whitespace-pre-wrap text-stone-700 bg-white/60 border border-stone-200 rounded-lg p-5 leading-7">
        {text}
      </pre>
    )
  }
  return (
    <div className="text-stone-500 text-sm">
      暂不支持的文档类型：<code className="font-mono">{kind}</code>
    </div>
  )
}
