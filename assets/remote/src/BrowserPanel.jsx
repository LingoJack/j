import { useState, useCallback, useRef } from 'react'

const QUICK_LINKS = [
  { label: 'localhost:3000', url: 'http://localhost:3000' },
  { label: 'localhost:8080', url: 'http://localhost:8080' },
  { label: 'localhost:5173', url: 'http://localhost:5173' },
  { label: 'localhost:4173', url: 'http://localhost:4173' },
  { label: 'localhost:4000', url: 'http://localhost:4000' },
  { label: 'localhost:8000', url: 'http://localhost:8000' },
]

function isLocalhost(url) {
  return /^https?:\/\/(localhost|127\.0\.0\.1)(:\d+)?/.test(url)
}

function looksLikeUrl(input) {
  const s = input.trim()
  if (/^https?:\/\//i.test(s)) return true
  return /^[\w.-]+\.\w{2,}(\/|$)/.test(s)
}

function googleSearchUrl(query) {
  return 'https://www.google.com/search?q=' + encodeURIComponent(query)
}

export default function BrowserPanel({ send, onBack }) {
  const [url, setUrl] = useState('')
  const [currentUrl, setCurrentUrl] = useState('')
  const [loading, setLoading] = useState(false)
  const [externalNotice, setExternalNotice] = useState(null)
  const iframeRef = useRef(null)

  const navigate = useCallback((targetUrl) => {
    let normalized = targetUrl.trim()
    if (!normalized) return

    if (!looksLikeUrl(normalized)) {
      normalized = googleSearchUrl(normalized)
    } else if (!/^https?:\/\//i.test(normalized)) {
      normalized = 'http://' + normalized
    }

    setUrl(normalized)
    setLoading(true)
    setExternalNotice(null)

    if (isLocalhost(normalized)) {
      setCurrentUrl(normalized)
    } else {
      setCurrentUrl('')
      setLoading(false)
      window.open(normalized, '_blank', 'noopener,noreferrer')
      setExternalNotice(normalized)
    }
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
    } else if (externalNotice) {
      window.open(externalNotice, '_blank', 'noopener,noreferrer')
    }
  }, [currentUrl, externalNotice])

  const goBack = useCallback(() => {
    try {
      if (iframeRef.current?.contentWindow) {
        iframeRef.current.contentWindow.history.back()
      }
    } catch {
      // cross-origin 限制，忽略
    }
  }, [])

  const openInNewTab = useCallback(() => {
    const target = currentUrl || externalNotice || url
    if (target) window.open(target, '_blank', 'noopener,noreferrer')
  }, [currentUrl, externalNotice, url])

  return (
    <div className="flex flex-col h-full">
      {/* URL 栏 */}
      <div className="flex items-center gap-2 px-3 py-2 bg-bg2 border-b border-border shrink-0">
        <button
          className="text-fg3 hover:text-fg active:text-accent active:scale-[0.9] text-[13px] p-1 rounded hover:bg-bg3 transition-all duration-100 select-none"
          onClick={onBack}
          title="回到聊天"
        >←</button>
        <button
          className="text-fg3 hover:text-fg active:text-accent active:scale-[0.9] text-[12px] px-1.5 py-1 rounded hover:bg-bg3 transition-all duration-100 select-none"
          onClick={goBack}
          title="后退"
        >←</button>
        <button
          className="text-fg3 hover:text-fg active:text-accent active:scale-[0.9] text-[12px] px-1.5 py-1 rounded hover:bg-bg3 transition-all duration-100 select-none"
          onClick={refresh}
          title="刷新"
        >↻</button>
        <input
          className="flex-1 bg-bg border border-border rounded-md px-3 py-1.5 text-[12px] text-fg outline-none focus:border-accent transition-colors"
          placeholder="输入 URL 或搜索..."
          value={url}
          onChange={e => setUrl(e.target.value)}
          onKeyDown={handleKeyDown}
        />
        <button
          className="text-fg3 hover:text-fg active:text-accent text-[12px] px-1.5 py-1.5 rounded hover:bg-bg3 transition-all duration-100 select-none"
          onClick={openInNewTab}
          disabled={!url.trim()}
          title="在新标签页打开"
        >⤴</button>
        <button
          className={`text-accent text-[11px] px-3 py-1.5 rounded-md transition-all duration-100 select-none ${url.trim() ? 'hover:bg-accent/10 active:bg-accent/20 active:scale-[0.95] border border-accent/30' : 'opacity-30 cursor-default border border-border'}`}
          onClick={() => navigate(url)}
          disabled={!url.trim()}
        >前往</button>
      </div>

      {/* 快捷链接 */}
      <div className="flex flex-wrap gap-1 px-3 py-1.5 bg-bg2 border-b border-border shrink-0">
        {QUICK_LINKS.map(link => (
          <button
            key={link.url}
            className={`text-[10px] px-1.5 py-0.5 rounded transition-all duration-100 select-none active:scale-[0.93] ${currentUrl === link.url ? 'bg-accent/20 text-accent' : 'bg-bg3 text-fg3 hover:text-fg hover:bg-border active:bg-border-light'}`}
            onClick={() => navigate(link.url)}
          >
            {link.label}
          </button>
        ))}
      </div>

      {/* iframe / 外链提示 */}
      <div className="flex-1 relative bg-white">
        {loading && (
          <div className="absolute inset-0 flex items-center justify-center bg-bg2/80 z-10">
            <div className="w-5 h-5 border-2 border-accent border-t-transparent rounded-full animate-spin" />
          </div>
        )}
        {currentUrl ? (
          <iframe
            ref={iframeRef}
            name="jstudio-browser"
            src={currentUrl}
            className="w-full h-full border-0"
            onLoad={handleIframeLoad}
            sandbox="allow-scripts allow-same-origin allow-forms allow-popups allow-storage-access-by-user-activation"
            allow="clipboard-read; clipboard-write"
            title="Browser"
          />
        ) : externalNotice ? (
          <div className="flex flex-col items-center justify-center h-full text-fg3 bg-bg gap-3 px-6">
            <svg className="w-12 h-12 text-accent/40" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M13.5 6H5.25A2.25 2.25 0 003 8.25v10.5A2.25 2.25 0 005.25 21h10.5A2.25 2.25 0 0018 18.75V10.5m-10.5 6L21 3m0 0h-5.25M21 3v5.25" />
            </svg>
            <div className="text-center">
              <div className="text-[13px] text-fg mb-1">已在新的浏览器标签页中打开</div>
              <div className="text-[11px] text-fg3 break-all max-w-xs">{externalNotice}</div>
            </div>
            <div className="text-[10px] text-fg3 text-center max-w-xs leading-relaxed">
              外部网站在新标签页打开以保持 Cookie 会话，避免触发异常流量检测
            </div>
            <button
              className="text-[11px] px-3 py-1.5 rounded-md border border-accent/30 text-accent hover:bg-accent/10 active:scale-[0.95] transition-all duration-100 select-none"
              onClick={() => window.open(externalNotice, '_blank', 'noopener,noreferrer')}
            >重新打开</button>
          </div>
        ) : (
          <div className="flex flex-col items-center justify-center h-full text-fg3 bg-bg">
            <svg className="w-16 h-16 mb-4 text-fg3/30" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9" />
            </svg>
            <span className="text-[13px]">输入 URL、搜索词或选择快捷链接</span>
            <span className="text-[10px] text-fg3/60 mt-1.5">localhost 地址在面板内打开，外部网站在新标签页打开</span>
          </div>
        )}
      </div>
    </div>
  )
}
