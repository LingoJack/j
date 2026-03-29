import { useState, useRef, useEffect, useCallback } from 'react'
import { useWebSocket } from './useWebSocket'
import { truncate } from './utils'
import Markdown from './Markdown'
import ToolModal from './ToolModal'

const params = new URLSearchParams(location.search)
const token = params.get('token') || ''
const wsProto = location.protocol === 'https:' ? 'wss:' : 'ws:'
const wsUrl = `${wsProto}//${location.host}/ws?token=${token}`

function Message({ role, content, streaming }) {
  const isUser = role === 'user'
  const isTool = role === 'tool'
  const base = 'max-w-[85%] px-4 py-3 rounded-2xl leading-relaxed break-words text-sm'
  const cls = isUser
    ? `${base} self-end bg-bubble-user text-white rounded-br-md whitespace-pre-wrap`
    : isTool
      ? `${base} self-start bg-transparent border border-border rounded-xl px-3.5 py-2 text-xs`
      : `${base} self-start bg-bubble-ai rounded-bl-md border border-border md-msg${streaming ? ' streaming' : ''}`
  return (
    <div className="flex flex-col gap-0.5">
      {!isTool && (
        <span className={`text-[11px] font-medium ${isUser ? 'self-end text-label-user' : 'self-start text-label-ai'}`}>
          {isUser ? '你' : 'AI'}
        </span>
      )}
      <div className={cls}>
        {isUser ? content : <Markdown content={content || ''} />}
      </div>
    </div>
  )
}

function ToolResultMsg({ toolName, output, isError }) {
  return (
    <div className="self-start bg-transparent border border-border rounded-xl px-3.5 py-2 text-xs max-w-[85%] break-words">
      <span className="font-semibold text-xs">{isError ? '❌' : '✅'} {toolName}</span>
      {output && <div className="text-fg2 text-[11px] mt-1 whitespace-pre-wrap break-all">{truncate(output, 300)}</div>}
    </div>
  )
}

export default function App() {
  const [messages, setMessages] = useState([])
  const [state, setState] = useState('idle')
  const [connected, setConnected] = useState(false)
  const [modelName, setModelName] = useState('--')
  const [toolConfirm, setToolConfirm] = useState(null)
  const [toolConfirmIdx, setToolConfirmIdx] = useState(0)
  const [inputText, setInputText] = useState('')
  const streamContentRef = useRef('')
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
        streamContentRef.current = msg.content
        setMessages(prev => {
          const last = prev[prev.length - 1]
          if (last?.streaming) {
            return [...prev.slice(0, -1), { ...last, content: msg.content }]
          }
          return [...prev, { role: 'assistant', content: msg.content, streaming: true }]
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
        streamContentRef.current = ''
        scrollToBottom()
        break

      case 'tool_confirm_request':
        setState('tool_confirm')
        setToolConfirm(msg.tools)
        setToolConfirmIdx(0)
        break

      case 'tool_result': {
        const name = msg.tool_call_id?.substring(0, 10) || 'tool'
        setMessages(prev => [...prev, {
          role: 'tool_result',
          toolName: name,
          output: msg.output,
          isError: msg.is_error,
        }])
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
          streamContentRef.current = ''
        }
        break

      case 'session_sync':
        streamContentRef.current = ''
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

  const confirmTool = useCallback((action, reason) => {
    const payload = { type: 'tool_confirm', action }
    if (reason) payload.reason = reason
    send(payload)

    if (toolConfirm && toolConfirmIdx < toolConfirm.length - 1) {
      setToolConfirmIdx(prev => prev + 1)
    } else {
      setToolConfirm(null)
      setToolConfirmIdx(0)
      setState('loading')
    }
  }, [send, toolConfirm, toolConfirmIdx])

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
  const msgCount = messages.length
  const statusText = isLoading
    ? 'AI 思考中...'
    : state === 'tool_confirm'
      ? '等待工具确认...'
      : connected
        ? '已连接'
        : '连接断开，重连中...'

  return (
    <div className="flex flex-col h-dvh max-w-[680px] mx-auto pl-[env(safe-area-inset-left)] pr-[env(safe-area-inset-right)]">
      {/* Header */}
      <div className="flex items-center gap-2.5 px-4.5 pt-[calc(14px+env(safe-area-inset-top))] pb-3.5 bg-bg2 border-b border-border shrink-0">
        <div className={`w-2 h-2 rounded-full shrink-0 transition-colors duration-300 ${connected ? 'bg-ok shadow-[0_0_6px_var(--color-ok)]' : 'bg-fg3'}`} />
        <span className="text-[17px]">🦞</span>
        <span className="font-bold text-[17px] tracking-wide">Sprite</span>
        <span className="text-border-light mx-0.5">│</span>
        <span className="text-label-ai text-sm font-medium">{modelName}</span>
        <span className="ml-auto text-fg2 text-xs">📬 {msgCount}</span>
      </div>

      {/* Messages */}
      <div className="flex-1 overflow-y-auto px-[calc(16px+env(safe-area-inset-left))] pr-[calc(16px+env(safe-area-inset-right))] py-4 flex flex-col gap-3 [-webkit-overflow-scrolling:touch]" ref={messagesRef}>
        {messages.length === 0 && (
          <div className="text-center text-fg3 mt-[40%] text-sm">发送消息开始对话</div>
        )}
        {messages.map((m, i) =>
          m.role === 'tool_result' ? (
            <ToolResultMsg key={i} toolName={m.toolName} output={m.output} isError={m.isError} />
          ) : (
            <Message key={i} role={m.role} content={m.content} streaming={m.streaming} />
          )
        )}
      </div>

      {/* Status Bar */}
      <div className={`px-4.5 py-1.5 text-center text-xs bg-bg2 border-t border-border shrink-0 flex items-center justify-center gap-1.5 ${isLoading ? 'text-warn' : 'text-fg3'}`}>
        {isLoading && <span className="w-1.5 h-1.5 rounded-full bg-warn animate-[pulse_1.2s_ease-in-out_infinite]" />}
        {statusText}
      </div>

      {/* Input Area */}
      <div className="flex gap-2.5 items-end px-3.5 pt-2.5 pb-[calc(10px+env(safe-area-inset-bottom))] bg-bg2 border-t border-border shrink-0">
        <textarea
          ref={textareaRef}
          rows={1}
          placeholder="输入消息..."
          autoComplete="off"
          value={inputText}
          onChange={e => setInputText(e.target.value)}
          onKeyDown={handleKeyDown}
          className={`flex-1 bg-bg3 border rounded-[22px] px-4.5 py-2.5 text-fg text-[15px] resize-none outline-none max-h-[120px] font-[inherit] leading-snug transition-colors duration-200 placeholder:text-fg3 ${isLoading ? 'border-[#786432]' : 'border-[#3c6450] focus:border-accent'}`}
        />
        {isLoading ? (
          <button
            className="w-11 h-11 rounded-full border-none text-sm cursor-pointer flex items-center justify-center shrink-0 transition-all duration-150 bg-danger text-white active:scale-[0.92]"
            onClick={cancelStream}
            title="取消"
          >■</button>
        ) : (
          <button
            className="w-11 h-11 rounded-full border-none text-xl cursor-pointer flex items-center justify-center shrink-0 transition-all duration-150 bg-label-user text-white disabled:opacity-30 disabled:cursor-default enabled:active:scale-[0.92]"
            onClick={sendMessage}
            disabled={!inputText.trim()}
            title="发送"
          >↑</button>
        )}
      </div>

      <ToolModal tools={toolConfirm} currentIndex={toolConfirmIdx} onConfirm={confirmTool} />
    </div>
  )
}
