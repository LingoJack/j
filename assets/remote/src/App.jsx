import { useState, useRef, useEffect, useCallback } from 'react'
import { useWebSocket } from './useWebSocket'
import { escHtml, renderMd, truncate } from './utils'

const params = new URLSearchParams(location.search)
const token = params.get('token') || ''
const wsProto = location.protocol === 'https:' ? 'wss:' : 'ws:'
const wsUrl = `${wsProto}//${location.host}/ws?token=${token}`

function Message({ role, content, streaming, dangerousHtml }) {
  const cls = role === 'user' ? 'user' : role === 'tool' ? 'tool' : 'assistant'
  return (
    <div
      className={`msg ${cls}${streaming ? ' streaming' : ''}`}
      dangerouslySetInnerHTML={{ __html: dangerousHtml ?? (role === 'user' ? escHtml(content) : renderMd(content)) }}
    />
  )
}

function ToolModal({ tools, onConfirm }) {
  if (!tools) return null
  return (
    <div className="modal-overlay">
      <div className="modal">
        <h3>🔧 工具调用确认</h3>
        <div
          className="tool-info"
          dangerouslySetInnerHTML={{
            __html: tools.map(t => `<b>${escHtml(t.name)}</b>\n${escHtml(t.confirm_message)}\n\n`).join(''),
          }}
        />
        <div className="btns">
          <button className="btn allow" onClick={() => onConfirm('allow')}>✓ 允许</button>
          <button className="btn reject" onClick={() => onConfirm('reject')}>✗ 拒绝</button>
        </div>
      </div>
    </div>
  )
}

export default function App() {
  const [messages, setMessages] = useState([])
  const [state, setState] = useState('idle')
  const [connected, setConnected] = useState(false)
  const [modelName, setModelName] = useState('--')
  const [toolConfirm, setToolConfirm] = useState(null)
  const [inputText, setInputText] = useState('')
  const streamRef = useRef(null)
  const messagesRef = useRef(null)
  const textareaRef = useRef(null)

  const scrollToBottom = useCallback(() => {
    requestAnimationFrame(() => {
      if (messagesRef.current) {
        messagesRef.current.scrollTop = messagesRef.current.scrollHeight
      }
    })
  }, [])

  const onMessage = useCallback((msg) => {
    switch (msg.type) {
      case 'stream_chunk':
        streamRef.current = renderMd(msg.content)
        setMessages(prev => {
          const last = prev[prev.length - 1]
          if (last?.streaming) {
            return [...prev.slice(0, -1), { ...last, dangerousHtml: streamRef.current }]
          }
          return [...prev, { role: 'assistant', content: '', streaming: true, dangerousHtml: streamRef.current }]
        })
        setState('loading')
        scrollToBottom()
        break

      case 'message':
        if (msg.role === 'assistant') {
          setMessages(prev => {
            const last = prev[prev.length - 1]
            if (last?.streaming) {
              return [...prev.slice(0, -1), { role: 'assistant', content: msg.content, streaming: false }]
            }
            return [...prev, { role: 'assistant', content: msg.content }]
          })
        } else {
          setMessages(prev => [...prev, { role: msg.role, content: msg.content }])
        }
        streamRef.current = null
        scrollToBottom()
        break

      case 'tool_confirm_request':
        setState('tool_confirm')
        setToolConfirm(msg.tools)
        break

      case 'tool_result': {
        const html = `<span class="tool-name">🔧 ${escHtml(msg.tool_call_id.substring(0, 8))}</span> ${msg.is_error ? '❌' : '✅'}\n${escHtml(truncate(msg.output, 200))}`
        setMessages(prev => [...prev, { role: 'tool', content: '', dangerousHtml: html }])
        scrollToBottom()
        break
      }

      case 'status':
        setState(msg.state)
        if (msg.state === 'idle') {
          setMessages(prev => {
            const last = prev[prev.length - 1]
            if (last?.streaming) {
              return [...prev.slice(0, -1), { ...last, streaming: false }]
            }
            return prev
          })
          streamRef.current = null
        }
        break

      case 'session_sync':
        streamRef.current = null
        setMessages(msg.messages.map(m => ({ role: m.role, content: m.content })))
        setModelName(msg.model || '--')
        setState(msg.status)
        scrollToBottom()
        break

      case 'error':
        break
    }
  }, [scrollToBottom])

  const onStatusChange = useCallback((isConnected) => {
    setConnected(isConnected)
  }, [])

  const send = useWebSocket(wsUrl, onMessage, onStatusChange)

  const sendMessage = useCallback(() => {
    const text = inputText.trim()
    if (!text || state === 'loading') return
    send({ type: 'send_message', content: text })
    setMessages(prev => [...prev, { role: 'user', content: text }])
    setInputText('')
    setState('loading')
    scrollToBottom()
  }, [inputText, state, send, scrollToBottom])

  const confirmTool = useCallback((action) => {
    send({ type: 'tool_confirm', action })
    setToolConfirm(null)
    setState('loading')
  }, [send])

  const cancelStream = useCallback(() => {
    send({ type: 'cancel' })
  }, [send])

  const handleKeyDown = useCallback((e) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      sendMessage()
    }
  }, [sendMessage])

  const autoResize = useCallback(() => {
    const el = textareaRef.current
    if (el) {
      el.style.height = 'auto'
      el.style.height = Math.min(el.scrollHeight, 120) + 'px'
    }
  }, [])

  useEffect(() => { autoResize() }, [inputText, autoResize])

  const isLoading = state === 'loading'
  const statusText = isLoading ? '⏳ AI 思考中...' : state === 'tool_confirm' ? '🔧 等待工具确认...' : connected ? '已连接' : '连接断开，3秒后重连...'

  return (
    <div id="app">
      <div className="header">
        <div className={`dot${connected ? ' connected' : ''}`} />
        <span className="title">🦞 Sprite</span>
        <span className="model">{modelName}</span>
      </div>

      <div className="messages" ref={messagesRef}>
        {messages.map((m, i) => (
          <Message key={i} role={m.role} content={m.content} streaming={m.streaming} dangerousHtml={m.dangerousHtml} />
        ))}
      </div>

      <div className="status-bar">{statusText}</div>

      <div className="input-area">
        <textarea
          ref={textareaRef}
          rows={1}
          placeholder="输入消息..."
          autoComplete="off"
          value={inputText}
          onChange={e => setInputText(e.target.value)}
          onKeyDown={handleKeyDown}
        />
        {isLoading ? (
          <button className="cancel-btn" onClick={cancelStream}>■</button>
        ) : (
          <button className="send-btn" disabled={false} onClick={sendMessage}>↑</button>
        )}
      </div>

      <ToolModal tools={toolConfirm} onConfirm={confirmTool} />
    </div>
  )
}
