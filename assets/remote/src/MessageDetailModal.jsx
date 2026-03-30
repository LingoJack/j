import { useEffect, useRef, useState } from 'react'
import Markdown from './Markdown'

export default function MessageDetailModal({ message, onClose }) {
  const contentRef = useRef(null)
  const [copied, setCopied] = useState(false)

  useEffect(() => {
    const handleKey = (e) => {
      if (e.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', handleKey)
    return () => window.removeEventListener('keydown', handleKey)
  }, [onClose])

  if (!message) return null

  const isUser = message.role === 'user'
  const isAssistant = message.role === 'assistant'
  const isToolCall = message.role === 'tool_call'
  const isToolResult = message.role === 'tool_result'

  const getRoleInfo = () => {
    if (isUser) return { icon: '👤', label: '用户消息', color: 'text-accent' }
    if (isAssistant) return { icon: '🦞', label: '助手回复', color: 'text-ok' }
    if (isToolCall) return { icon: '🔧', label: `工具调用: ${message.name}`, color: 'text-warn' }
    if (isToolResult) return { icon: '📋', label: `工具结果: ${message.toolName}`, color: message.isError ? 'text-err' : 'text-fg2' }
    return { icon: '💬', label: '消息', color: 'text-fg' }
  }

  const roleInfo = getRoleInfo()

  const handleCopy = async () => {
    const text = isToolCall 
      ? message.arguments 
      : isToolResult 
        ? message.output 
        : message.content || ''
    
    try {
      // 优先使用现代 Clipboard API
      if (navigator.clipboard && window.isSecureContext) {
        await navigator.clipboard.writeText(text)
        setCopied(true)
        setTimeout(() => setCopied(false), 2000)
        return
      }
      
      // 备用方案：使用 execCommand (兼容非 HTTPS 环境)
      const textarea = document.createElement('textarea')
      textarea.value = text
      textarea.style.position = 'fixed'
      textarea.style.left = '-9999px'
      textarea.style.top = '-9999px'
      document.body.appendChild(textarea)
      textarea.focus()
      textarea.select()
      const success = document.execCommand('copy')
      document.body.removeChild(textarea)
      
      if (success) {
        setCopied(true)
        setTimeout(() => setCopied(false), 2000)
      } else {
        alert('复制失败，请手动复制')
      }
    } catch (e) {
      console.error('Copy failed:', e)
      alert('复制失败: ' + e.message)
    }
  }

  const renderContent = () => {
    if (isToolCall) {
      let parsed = null
      try { parsed = JSON.parse(message.arguments) } catch {}
      return (
        <div className="space-y-4">
          <div className="flex items-center gap-2">
            <span className={`px-3 py-1 rounded-full text-xs font-medium ${message.completed ? 'bg-ok/20 text-ok' : 'bg-warn/20 text-warn'}`}>
              {message.completed ? '已完成' : '执行中'}
            </span>
          </div>
          {parsed ? (
            <div className="space-y-3">
              {Object.entries(parsed).map(([k, v]) => (
                <div key={k} className="bg-bg2 rounded-lg p-3 border border-border">
                  <div className="text-xs text-fg3 mb-1 font-medium">{k}</div>
                  <div className="text-sm text-fg whitespace-pre-wrap break-all">
                    {typeof v === 'string' ? v : JSON.stringify(v, null, 2)}
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div className="bg-bg2 rounded-lg p-3 border border-border text-sm text-fg whitespace-pre-wrap break-all">
              {message.arguments}
            </div>
          )}
        </div>
      )
    }

    if (isToolResult) {
      return (
        <div className="space-y-4">
          {message.isError && (
            <div className="bg-err/10 border border-err/30 rounded-lg p-3 text-err text-sm">
              执行出错
            </div>
          )}
          <div className="bg-bg2 rounded-lg p-3 border border-border text-sm text-fg whitespace-pre-wrap break-all max-h-[60vh] overflow-auto">
            {message.output}
          </div>
        </div>
      )
    }

    return (
      <div className="md-msg max-w-none">
        <Markdown content={message.content || ''} />
      </div>
    )
  }

  return (
    <div 
      className="fixed inset-0 bg-black/70 backdrop-blur-sm flex items-center justify-center z-[200] p-4"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose()
      }}
    >
      <div className="bg-bg3 border border-border rounded-2xl w-full max-w-3xl max-h-[85vh] flex flex-col shadow-2xl">
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-border shrink-0">
          <div className="flex items-center gap-3">
            <span className="text-xl">{roleInfo.icon}</span>
            <div>
              <div className={`font-semibold ${roleInfo.color}`}>{roleInfo.label}</div>
              <div className="text-xs text-fg3 mt-0.5">
                {isToolResult && `${message.output?.length || 0} 字符`}
                {(isUser || isAssistant) && `${message.content?.length || 0} 字符`}
                {isToolCall && `${message.arguments?.length || 0} 字符`}
              </div>
            </div>
          </div>
          <button 
            className="w-8 h-8 flex items-center justify-center rounded-lg hover:bg-bg3 transition-colors text-fg2 hover:text-fg"
            onClick={onClose}
          >
            ✕
          </button>
        </div>

        {/* Content */}
        <div ref={contentRef} className="flex-1 overflow-auto p-5">
          {renderContent()}
        </div>

        {/* Footer */}
        <div className="px-5 py-3 border-t border-border flex justify-end gap-2 shrink-0">
          <button
            className={`px-4 py-2 rounded-lg text-sm font-medium transition-all ${copied ? 'bg-ok/20 text-ok' : 'bg-bg3 text-fg2 hover:text-fg'}`}
            onClick={handleCopy}
          >
            {copied ? '✓ 已复制' : '📋 复制内容'}
          </button>
          <button
            className="px-4 py-2 rounded-lg text-sm font-medium bg-accent text-[#111] hover:opacity-90 transition-opacity"
            onClick={onClose}
          >
            关闭
          </button>
        </div>
      </div>
    </div>
  )
}
