import { useState, useRef, useEffect, useCallback } from 'react'
import { useWebSocket } from './useWebSocket'
import { truncate } from './utils'
import Markdown from './Markdown'
import ToolModal from './ToolModal'
import AskModal from './AskModal'

const params = new URLSearchParams(location.search)
const token = params.get('token') || ''
const wsProto = location.protocol === 'https:' ? 'wss:' : 'ws:'
const wsUrl = `${wsProto}//${location.host}/ws?token=${token}`

function Message({ role, content, streaming }) {
  const isUser = role === 'user'
  const base = 'max-w-[85%] px-4 py-3 rounded-2xl leading-relaxed break-words text-sm'
  const cls = isUser
    ? `${base} self-end bg-bubble-user text-white rounded-br-md whitespace-pre-wrap`
    : `${base} self-start bg-bubble-ai rounded-bl-md border border-border md-msg${streaming ? ' streaming' : ''}`
  return (
    <div className="flex flex-col gap-0.5">
      <span className={`text-[11px] font-medium ${isUser ? 'self-end text-label-user' : 'self-start text-label-ai'}`}>
        {isUser ? '你' : 'AI'}
      </span>
      <div className={cls}>
        {isUser ? content : <Markdown content={content || ''} />}
      </div>
    </div>
  )
}

function ToolCallMsg({ name, arguments: args }) {
  const [expanded, setExpanded] = useState(false)
  let parsed = null
  try { parsed = JSON.parse(args) } catch {}

  return (
    <div
      className="self-start max-w-[85%] rounded-xl border border-border bg-bg2 overflow-hidden cursor-pointer active:opacity-80 transition-opacity"
      onClick={() => setExpanded(e => !e)}
    >
      <div className="flex items-center gap-2 px-3 py-2 text-xs">
        <span className="w-3 h-3 rounded-full border-2 border-warn animate-spin border-t-transparent inline-block shrink-0" />
        <span className="font-semibold text-warn">{name}</span>
        <span className="text-fg3 text-[11px] ml-auto">执行中</span>
      </div>
      {expanded && parsed && (
        <div className="px-3 pb-2 text-[11px] text-fg2 border-t border-border pt-2">
          {Object.entries(parsed).map(([k, v]) => (
            <div key={k} className="mb-1 last:mb-0">
              <span className="text-fg3">{k}: </span>
              <span className="whitespace-pre-wrap break-all">{typeof v === 'string' ? truncate(v, 500) : JSON.stringify(v)}</span>
            </div>
          ))}
        </div>
      )}
      {expanded && !parsed && args && (
        <div className="px-3 pb-2 text-[11px] text-fg2 border-t border-border pt-2 whitespace-pre-wrap break-all">
          {truncate(args, 500)}
        </div>
      )}
    </div>
  )
}

function ToolResultMsg({ toolName, output, isError }) {
  const [expanded, setExpanded] = useState(false)
  const icon = isError ? '✗' : '✓'
  const iconCls = isError ? 'text-danger' : 'text-ok'
  const hasOutput = output && output.trim()

  return (
    <div
      className={`self-start max-w-[85%] rounded-xl border overflow-hidden transition-opacity ${hasOutput ? 'cursor-pointer active:opacity-80' : ''} ${isError ? 'border-danger/40 bg-danger/5' : 'border-border bg-bg2'}`}
      onClick={() => hasOutput && setExpanded(e => !e)}
    >
      <div className="flex items-center gap-2 px-3 py-2 text-xs">
        <span className={`font-bold text-sm ${iconCls}`}>{icon}</span>
        <span className="font-semibold text-fg2">{toolName}</span>
        {hasOutput && <span className="text-fg3 text-[11px] ml-auto">{expanded ? '收起' : '详情'}</span>}
      </div>
      {expanded && hasOutput && (
        <div className="px-3 pb-2 text-[11px] text-fg2 border-t border-border pt-2 whitespace-pre-wrap break-all max-h-[300px] overflow-y-auto">
          {output}
        </div>
      )}
    </div>
  )
}

function isNearBottom(el) {
  if (!el) return true
  return el.scrollHeight - el.scrollTop - el.clientHeight < 80
}

export default function App() {
  const [messages, setMessages] = useState([])
  const [state, setState] = useState('idle')
  const [connected, setConnected] = useState(false)
  const [modelName, setModelName] = useState('--')
  const [toolConfirm, setToolConfirm] = useState(null)
  const [toolConfirmIdx, setToolConfirmIdx] = useState(0)
  const [askQuestions, setAskQuestions] = useState(null)
  const [toast, setToast] = useState(null)
  const [inputText, setInputText] = useState('')
  const streamContentRef = useRef('')
  const messagesRef = useRef(null)
  const textareaRef = useRef(null)
  const autoScrollRef = useRef(true)

  const scrollToBottom = useCallback(() => {
    if (!autoScrollRef.current) return
    requestAnimationFrame(() => {
      if (messagesRef.current) {
        messagesRef.current.scrollTop = messagesRef.current.scrollHeight
      }
    })
  }, [])

  const handleScroll = useCallback(() => {
    autoScrollRef.current = isNearBottom(messagesRef.current)
  }, [])

  const onMessage = useCallback((msg) => {
    switch (msg.type) {
      case 'stream_chunk':
        // Skip empty chunks to avoid empty bubbles
        if (!msg.content) break
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
            // Skip empty messages
            if (!msg.content) return prev
            return [...prev, { role: 'assistant', content: msg.content }]
          })
        } else {
          if (msg.content) {
            setMessages(prev => [...prev, { role: msg.role, content: msg.content }])
          }
        }
        streamContentRef.current = ''
        scrollToBottom()
        break

      case 'tool_confirm_request':
        setState('tool_confirm')
        setToolConfirm(msg.tools)
        setToolConfirmIdx(0)
        break

      case 'ask_request':
        setState('ask')
        setAskQuestions(msg.questions)
        break

      case 'tool_call':
        setMessages(prev => {
          // Replace last tool_call (show only latest running tool)
          const last = prev[prev.length - 1]
          if (last?.role === 'tool_call') {
            return [...prev.slice(0, -1), { role: 'tool_call', name: msg.name, arguments: msg.arguments }]
          }
          return [...prev, { role: 'tool_call', name: msg.name, arguments: msg.arguments }]
        })
        scrollToBottom()
        break

      case 'tool_result': {
        setMessages(prev => {
          // Remove running tool_call indicator, add result
          const filtered = prev.filter(m => m.role !== 'tool_call')
          return [...filtered, {
            role: 'tool_result',
            toolName: msg.name || 'tool',
            output: msg.output,
            isError: msg.is_error,
          }]
        })
        scrollToBottom()
        break
      }

      case 'status':
        setState(msg.state)
        if (msg.state === 'idle') {
          setMessages(prev => {
            const last = prev[prev.length - 1]
            if (last?.streaming) {
              return [...prev.slice(0, -1), { ...last, streaming: false }].filter(m => m.role !== 'tool_call')
            }
            return prev.filter(m => m.role !== 'tool_call')
          })
          streamContentRef.current = ''
        }
        break

      case 'session_sync':
        streamContentRef.current = ''
        // Filter empty messages from sync
        setMessages(msg.messages.filter(m => m.content).map(m => ({ role: m.role, content: m.content })))
        setModelName(msg.model || '--')
        setState(msg.status)
        autoScrollRef.current = true
        scrollToBottom()
        break

      case 'error':
        setToast(msg.message || '发生错误')
        setTimeout(() => setToast(null), 4000)
        break
    }
  }, [scrollToBottom])

  const onStatusChange = useCallback((isConnected) => {
    setConnected(isConnected)
  }, [])

  const send = useWebSocket(wsUrl, onMessage, onStatusChange)

  const sendMessage = useCallback(() => {
    const text = inputText.trim()
    if (!text || !connected) return
    send({ type: 'send_message', content: text })
    setMessages(prev => [...prev, { role: 'user', content: text }])
    setInputText('')
    if (state !== 'loading') setState('loading')
    autoScrollRef.current = true
    scrollToBottom()
  }, [inputText, state, connected, send, scrollToBottom])

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

  const submitAsk = useCallback((answers) => {
    send({ type: 'ask_response', answers })
    setAskQuestions(null)
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
  const msgCount = messages.length
  const statusText = !connected
    ? '连接断开，重连中...'
    : isLoading
      ? 'AI 思考中...'
      : state === 'tool_confirm'
        ? '等待工具确认...'
        : state === 'ask'
          ? '等待回答问题...'
          : '已连接'

  return (
    <div className="flex flex-col h-[100dvh] max-w-[680px] mx-auto">
      {/* Header */}
      <div className="flex items-center gap-2 px-4 pt-[max(10px,env(safe-area-inset-top))] pb-2.5 bg-bg2/95 backdrop-blur-sm border-b border-border shrink-0">
        <div className={`w-2.5 h-2.5 rounded-full shrink-0 transition-colors duration-300 ${connected ? 'bg-ok shadow-[0_0_6px_var(--color-ok)]' : 'bg-fg3'}`} />
        <span className="text-[17px]">🦞</span>
        <span className="font-bold text-[16px] tracking-wide">Sprite</span>
        <span className="text-border-light mx-0.5 text-sm">|</span>
        <span className="text-label-ai text-[13px] font-medium">{modelName}</span>
        <span className="ml-auto text-fg2 text-xs font-medium">📬 {msgCount}</span>
      </div>

      {/* Messages */}
      <div
        className="flex-1 overflow-y-auto px-4 py-4 flex flex-col gap-2.5 [-webkit-overflow-scrolling:touch]"
        ref={messagesRef}
        onScroll={handleScroll}
      >
        {messages.length === 0 && (
          <div className="text-center text-fg3 mt-[40%] text-sm">发送消息开始对话</div>
        )}
        {messages.map((m, i) =>
          m.role === 'tool_call' ? (
            <ToolCallMsg key={`tc-${i}`} name={m.name} arguments={m.arguments} />
          ) : m.role === 'tool_result' ? (
            <ToolResultMsg key={`tr-${i}`} toolName={m.toolName} output={m.output} isError={m.isError} />
          ) : (
            <Message key={i} role={m.role} content={m.content} streaming={m.streaming} />
          )
        )}
      </div>

      {/* Toast */}
      {toast && (
        <div className="px-5 py-2 text-center text-[13px] text-danger bg-danger/10 border-t border-danger/30 shrink-0">{toast}</div>
      )}

      {/* Status Bar */}
      <div className={`px-5 py-1.5 text-center text-[12px] bg-bg2/95 backdrop-blur-sm border-t border-border shrink-0 flex items-center justify-center gap-2 ${!connected ? 'text-danger' : isLoading ? 'text-warn' : 'text-fg3'}`}>
        {isLoading && connected && <span className="w-2 h-2 rounded-full bg-warn animate-[pulse_1.2s_ease-in-out_infinite]" />}
        {statusText}
      </div>

      {/* Input Area */}
      <div className="flex gap-2 items-end px-4 pt-3.5 pb-[max(16px,env(safe-area-inset-bottom))] bg-bg2/95 backdrop-blur-sm border-t border-border shrink-0">
        <textarea
          ref={textareaRef}
          rows={1}
          placeholder={isLoading ? '追加消息...' : '输入消息...'}
          autoComplete="off"
          value={inputText}
          onChange={e => setInputText(e.target.value)}
          onKeyDown={handleKeyDown}
          className={`flex-1 bg-bg3 border-2 rounded-2xl px-5 py-3.5 text-fg text-[16px] resize-none outline-none max-h-[140px] font-[inherit] leading-relaxed transition-colors duration-200 placeholder:text-fg3 ${isLoading ? 'border-[#786432]' : 'border-[#3c6450] focus:border-accent'}`}
        />
        <button
          className="w-[48px] h-[48px] rounded-full border-none text-2xl cursor-pointer flex items-center justify-center shrink-0 transition-all duration-150 bg-label-user text-white disabled:opacity-30 disabled:cursor-default enabled:active:scale-[0.92]"
          onClick={sendMessage}
          disabled={!inputText.trim()}
          title="发送"
        >↑</button>
        {isLoading && (
          <button
            className="w-[48px] h-[48px] rounded-full border-none text-lg cursor-pointer flex items-center justify-center shrink-0 transition-all duration-150 bg-danger text-white active:scale-[0.92]"
            onClick={cancelStream}
            title="取消"
          >■</button>
        )}
      </div>

      <ToolModal tools={toolConfirm} currentIndex={toolConfirmIdx} onConfirm={confirmTool} />
      <AskModal questions={askQuestions} onSubmit={submitAsk} />
    </div>
  )
}
