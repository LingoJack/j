import { useState, useCallback, useRef } from 'react'

const QUICK_LINKS = [
  { label: 'localhost:3000', url: 'http://localhost:3000' },
  { label: 'localhost:8080', url: 'http://localhost:8080' },
  { label: 'localhost:5173', url: 'http://localhost:5173' },
  { label: 'localhost:4173', url: 'http://localhost:4173' },
  { label: 'localhost:4000', url: 'http://localhost:4000' },
  { label: 'localhost:8000', url: 'http://localhost:8000' },
]

export default function BrowserSection({ send, onCollapse }) {
  const [url, setUrl] = useState('')
  const [currentUrl, setCurrentUrl] = useState('')
  const [loading, setLoading] = useState(false)
  const iframeRef = useRef(null)

  const navigate = useCallback((targetUrl) => {
    let normalized = targetUrl.trim()
    if (!normalized) return
    // 自动加 http:// 前缀
    if (!normalized.startsWith('http://') && !normalized.startsWith('https://')) {
      normalized = 'http://' + normalized
    }
    setUrl(normalized)
    setCurrentUrl(normalized)
    setLoading(true)
  }, [])

  const handleKeyDown = useCallback((e) => {
    if (e.key === 'Enter') {
      e.preventDefault()
      navigate(url)
    }
  }, [url, navigate])

  const handleIframeLoad = useCallback(() => {
    setLoading(false)
  }, [])

  const refresh = useCallback(() => {
    if (iframeRef.current && currentUrl) {
      setLoading(true)
      iframeRef.current.src = currentUrl
    }
  }, [currentUrl])

  const goBack = useCallback(() => {
    try {
      if (iframeRef.current?.contentWindow) {
        iframeRef.current.contentWindow.history.back()
      }
    } catch {
      // cross-origin 限制，忽略
    }
  }, [])

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="sidebar-section-header">
        <span className="font-semibold text-[13px]">浏览器</span>
        <button
          className="text-fg3 hover:text-fg p-1 rounded-md hover:bg-bg3 transition-colors"
          onClick={onCollapse}
          title="收起侧边栏"
        >
          <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M15 19l-7-7 7-7" />
          </svg>
        </button>
      </div>

      {/* URL bar */}
      <div className="px-3 py-2 border-b border-border shrink-0 space-y-2">
        <div className="flex items-center gap-1">
          <button
            className="text-fg3 hover:text-fg text-[12px] px-1.5 py-1 rounded hover:bg-bg3 transition-colors shrink-0"
            onClick={goBack}
            title="后退"
          >←</button>
          <button
            className="text-fg3 hover:text-fg text-[12px] px-1.5 py-1 rounded hover:bg-bg3 transition-colors shrink-0"
            onClick={refresh}
            title="刷新"
          >↻</button>
          <input
            className="flex-1 bg-bg border border-border rounded px-2 py-1 text-[11px] text-fg outline-none focus:border-accent"
            placeholder="输入 URL..."
            value={url}
            onChange={e => setUrl(e.target.value)}
            onKeyDown={handleKeyDown}
          />
          <button
            className="text-accent text-[11px] px-2 py-1 rounded hover:bg-accent/10 transition-colors shrink-0"
            onClick={() => navigate(url)}
          >前往</button>
        </div>
        {/* Quick links */}
        <div className="flex flex-wrap gap-1">
          {QUICK_LINKS.map(link => (
            <button
              key={link.url}
              className={`text-[10px] px-1.5 py-0.5 rounded transition-colors ${currentUrl === link.url ? 'bg-accent/20 text-accent' : 'bg-bg3 text-fg3 hover:text-fg hover:bg-border'}`}
              onClick={() => navigate(link.url)}
            >
              {link.label}
            </button>
          ))}
        </div>
      </div>

      {/* iframe */}
      <div className="flex-1 relative bg-white">
        {loading && (
          <div className="absolute inset-0 flex items-center justify-center bg-bg2/80 z-10">
            <div className="w-5 h-5 border-2 border-accent border-t-transparent rounded-full animate-spin" />
          </div>
        )}
        {currentUrl ? (
          <iframe
            ref={iframeRef}
            src={currentUrl}
            className="w-full h-full border-0"
            onLoad={handleIframeLoad}
            sandbox="allow-scripts allow-same-origin allow-forms allow-popups"
            title="Browser"
          />
        ) : (
          <div className="flex flex-col items-center justify-center h-full text-fg3">
            <svg className="w-12 h-12 mb-3 text-fg3/50" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9" />
            </svg>
            <span className="text-[12px]">输入 URL 或选择快捷链接</span>
          </div>
        )}
      </div>
    </div>
  )
}
