import { useEffect, useState } from 'react'
import { MarkdownIR } from './MarkdownIR'
import type { ParsedDocument, RenderedDoc } from './types'

type LoadState =
  | { kind: 'loading' }
  | { kind: 'error'; message: string }
  | { kind: 'ready'; doc: RenderedDoc }

export function Reader() {
  const [state, setState] = useState<LoadState>({ kind: 'loading' })

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

  if (state.kind === 'loading') {
    return (
      <div className="min-h-screen bg-[#faf9f6] text-stone-500 flex items-center justify-center text-sm">
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

  return (
    <div className="min-h-screen bg-[#faf9f6] text-stone-800">
      <header className="border-b border-stone-200 px-6 py-3 flex items-center justify-between sticky top-0 bg-[#faf9f6]/90 backdrop-blur z-10">
        <div className="flex items-center gap-3">
          <span className="text-sm font-medium text-stone-900">{filename}</span>
          <span className="text-xs px-2 py-0.5 rounded bg-stone-100 text-stone-500 uppercase">
            {kind}
          </span>
        </div>
        <span className="text-xs text-stone-400">在终端按 Ctrl-C 关闭</span>
      </header>
      <main className="max-w-3xl mx-auto px-6 py-8">
        {renderPayload(kind, payload)}
      </main>
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
      <pre className="text-sm font-mono whitespace-pre-wrap text-stone-700 bg-white border border-stone-200 rounded p-4">
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
