import { useState, useRef, useEffect, useCallback } from 'react'
import { useWebSocket } from './useWebSocket'
import { truncate } from './utils'
import Markdown from './Markdown'
import MessageDetailModal from './MessageDetailModal'
import ToolModal from './ToolModal'
import AskModal from './AskModal'

const params = new URLSearchParams(location.search)
const token = params.get('token') || ''
const wsProto = location.protocol === 'https:' ? 'wss:' : 'ws:'
const wsUrl = `${wsProto}//${location.host}/ws?token=${token}`

function Message({ role, content, streaming, onDetail }) {
  const isUser = role === 'user'
  const base = 'w-auto max-w-[90%] sm:max-w-[85%] md:max-w-[80%] lg:max-w-[75%] px-4 py-3 rounded-2xl leading-relaxed break-words text-sm'
  const cls = isUser
    ? `${base} self-end bg-bubble-user text-white rounded-br-md whitespace-pre-wrap`
    : `${base} self-start bg-bubble-ai rounded-bl-md border border-border md-msg${streaming ? ' streaming' : ''}`
  return (
    <div className="flex flex-col gap-0.5 cursor-pointer active:opacity-80 transition-opacity" onClick={onDetail}>
      <span className={`text-[11px] font-medium ${isUser ? 'self-end text-label-user' : 'self-start text-label-ai'}`}>
        {isUser ? '你' : 'Sprite'}
      </span>
      <div className={cls}>
        {isUser ? content : <Markdown content={content || ''} />}
      </div>
      <span className={`text-[10px] text-fg3 ${isUser ? 'self-end' : 'self-start'} mt-0.5 opacity-0 hover:opacity-100 transition-opacity`}>
        点击查看详情
      </span>
    </div>
  )
}

function ToolCallMsg({ name, arguments: args, completed, collapsed: initCollapsed, onDetail }) {
  const [expanded, setExpanded] = useState(!initCollapsed)
  let parsed = null
  try { parsed = JSON.parse(args) } catch {}

  const statusIcon = completed
    ? <span className="text-ok font-bold text-sm">✓</span>
    : <span className="w-3 h-3 rounded-full border-2 border-warn animate-spin border-t-transparent inline-block shrink-0" />
  const statusText = completed ? '已完成' : '执行中'
  const statusCls = completed ? 'text-ok' : 'text-warn'

  const handleClick = (e) => {
    if (e.target.closest('.expand-btn')) {
      setExpanded(e => !e)
    } else if (onDetail) {
      onDetail()
    }
  }

  return (
    <div
      className={`self-start w-[90%] sm:w-[85%] md:w-[80%] lg:w-[75%] rounded-xl border overflow-hidden cursor-pointer active:opacity-80 transition-opacity shrink-0 min-h-[44px] ${completed ? 'border-ok/30 bg-ok/5' : 'border-border bg-bg2'}`}
      onClick={handleClick}
    >
      <div className="flex items-center gap-2 px-3 py-2 text-xs">
        {statusIcon}
        <span className={`font-semibold ${statusCls}`}>{name}</span>
        <span className={`text-[11px] ml-auto ${completed ? 'text-ok/70' : 'text-fg3'}`}>{statusText}</span>
        <button className="expand-btn text-fg3 hover:text-fg px-1" onClick={e => e.stopPropagation()}>
          {expanded ? '收起' : '展开'}
        </button>
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

function ToolResultMsg({ toolName, output, isError, collapsed: initCollapsed, onDetail }) {
  const [expanded, setExpanded] = useState(!initCollapsed)
  const icon = isError ? '✗' : '✓'
  const iconCls = isError ? 'text-err' : 'text-ok'
  const hasOutput = output && output.trim()

  // 生成预览文本（最多 50 字符）
  const preview = hasOutput
    ? (output.length > 50 ? output.slice(0, 50) + '...' : output)
    : ''

  const handleClick = (e) => {
    if (e.target.closest('.expand-btn')) {
      setExpanded(e => !e)
    } else if (onDetail) {
      onDetail()
    }
  }

  return (
    <div
      className={`self-start w-[90%] sm:w-[85%] md:w-[80%] lg:w-[75%] rounded-xl border overflow-hidden transition-opacity shrink-0 min-h-[44px] ${hasOutput ? 'cursor-pointer active:opacity-80' : ''} ${isError ? 'border-err/40 bg-err/5' : 'border-border bg-bg2'}`}
      onClick={handleClick}
    >
      <div className="flex items-center gap-2 px-3 py-2 text-xs">
        <span className={`font-bold text-sm ${iconCls}`}>{icon}</span>
        <span className="font-semibold text-fg2">{toolName}</span>
        {hasOutput && (
          <>
            <span className="text-fg3 text-[11px] ml-auto">{expanded ? '收起' : '展开'}</span>
            <button className="expand-btn text-accent text-[11px] hover:underline">详情</button>
          </>
        )}
      </div>
      {/* 收起状态显示预览 */}
      {!expanded && hasOutput && (
        <div className="px-3 pb-2 text-[11px] text-fg3 border-t border-border pt-2 truncate">
          {preview}
        </div>
      )}
      {/* 展开状态显示完整内容 */}
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

function formatRelativeTime(ts) {
  if (!ts) return ''
  const now = Math.floor(Date.now() / 1000)
  const diff = now - ts
  if (diff < 60) return '刚刚'
  if (diff < 3600) return `${Math.floor(diff / 60)} 分钟前`
  if (diff < 86400) return `${Math.floor(diff / 3600)} 小时前`
  if (diff < 604800) return `${Math.floor(diff / 86400)} 天前`
  const d = new Date(ts * 1000)
  return `${d.getMonth() + 1}/${d.getDate()}`
}

function SessionSidebar({ sessions, currentSessionId, onSwitch, onNew, onClose }) {
  return (
    <div className="sidebar-overlay" onClick={onClose}>
      <div className="sidebar-panel" onClick={e => e.stopPropagation()}>
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b border-border">
          <span className="font-bold text-[15px]">会话列表</span>
          <button
            className="text-fg3 hover:text-fg text-xl leading-none px-1"
            onClick={onClose}
          >×</button>
        </div>

        {/* Session list */}
        <div className="flex-1 overflow-y-auto">
          {sessions.map(s => {
            const isCurrent = s.id === currentSessionId
            return (
              <div
                key={s.id}
                className={`session-item px-4 py-3 border-b border-border/50 cursor-pointer transition-colors ${isCurrent ? 'bg-accent/10 border-l-2 border-l-accent' : 'hover:bg-bg3'}`}
                onClick={() => onSwitch(s.id)}
              >
                <div className="flex items-center justify-between mb-1">
                  <span className={`text-[13px] font-medium truncate flex-1 mr-2 ${isCurrent ? 'text-accent' : 'text-fg'}`}>
                    {s.first_message_preview || '新会话'}
                  </span>
                  {isCurrent && <span className="text-[10px] text-accent bg-accent/15 px-1.5 py-0.5 rounded-full shrink-0">当前</span>}
                </div>
                <div className="flex items-center gap-2 text-[11px] text-fg3">
                  <span>{s.message_count} 条消息</span>
                  <span>·</span>
                  <span>{formatRelativeTime(s.updated_at)}</span>
                </div>
              </div>
            )
          })}
          {sessions.length === 0 && (
            <div className="text-center text-fg3 text-sm py-8">暂无会话</div>
          )}
        </div>

        {/* New session button */}
        <div className="px-4 pt-3 pb-[max(12px,env(safe-area-inset-bottom))] border-t border-border">
          <button
            className="w-full py-2.5 rounded-xl bg-accent/15 text-accent text-[13px] font-medium hover:bg-accent/25 transition-colors"
            onClick={onNew}
          >
            + 新建会话
          </button>
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
  const [toolConfirmIdx, setToolConfirmIdx] = useState(0)
  const [askQuestions, setAskQuestions] = useState(null)
  const [toast, setToast] = useState(null)
  const [inputText, setInputText] = useState('')
  const [detailMessage, setDetailMessage] = useState(null)
  const [sessions, setSessions] = useState([])
  const [showSidebar, setShowSidebar] = useState(false)
  const [currentSessionId, setCurrentSessionId] = useState(null)
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
        // Add tool call to history (don't replace)
        setMessages(prev => [...prev, {
          role: 'tool_call',
          name: msg.name,
          arguments: msg.arguments,
          id: msg.id || Date.now()
        }])
        scrollToBottom()
        break

      case 'tool_result': {
        setMessages(prev => {
          // Mark corresponding tool_call as completed, add result
          return prev.map(m => {
            if (m.role === 'tool_call' && m.name === msg.name && !m.completed) {
              return { ...m, completed: true }
            }
            return m
          }).concat({
            role: 'tool_result',
            toolName: msg.name || 'tool',
            output: msg.output,
            isError: msg.is_error,
          })
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
              return [...prev.slice(0, -1), { ...last, streaming: false }].map(m =>
                m.role === 'tool_call' ? { ...m, completed: true } : m
              )
            }
            return prev.map(m => m.role === 'tool_call' ? { ...m, completed: true } : m)
          })
          streamContentRef.current = ''
        }
        break

      case 'session_sync':
        streamContentRef.current = ''
        // 转换后端消息为前端格式，正确处理 tool_calls 和 tool_result
        // 先建立 tool_call_id -> tool_name 映射
        const toolNameMap = {}
        for (const m of msg.messages) {
          if (m.tool_calls) {
            for (const tc of m.tool_calls) {
              toolNameMap[tc.id] = tc.name
            }
          }
        }

        const syncedMsgs = []
        for (const m of msg.messages) {
          if (m.tool_calls && m.tool_calls.length > 0) {
            // assistant 消息带有 tool_calls：生成 tool_call 消息，默认折叠
            for (const tc of m.tool_calls) {
              syncedMsgs.push({
                role: 'tool_call',
                name: tc.name,
                arguments: tc.arguments,
                id: tc.id,
                completed: true, // 断线重连后默认已完成
                collapsed: true, // 默认折叠
              })
            }
          } else if (m.role === 'tool' && m.tool_call_id) {
            // tool 消息：生成 tool_result 消息，默认折叠
            const toolName = toolNameMap[m.tool_call_id] || 'tool'
            syncedMsgs.push({
              role: 'tool_result',
              toolName: toolName,
              output: m.content,
              isError: false,
              collapsed: true, // 默认折叠
            })
          } else if (m.content) {
            // 普通消息
            syncedMsgs.push({ role: m.role, content: m.content })
          }
        }
        setMessages(syncedMsgs)
        setModelName(msg.model || '--')
        setState(msg.status)
        autoScrollRef.current = true
        scrollToBottom()
        break

      case 'session_list':
        setSessions(msg.sessions || [])
        break

      case 'session_switched':
        setShowSidebar(false)
        if (msg.session_id) {
          setCurrentSessionId(msg.session_id)
        }
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

  const fetchSessions = useCallback(() => {
    send({ type: 'list_sessions' })
  }, [send])

  const switchSession = useCallback((id) => {
    send({ type: 'switch_session', session_id: id })
  }, [send])

  const newSession = useCallback(() => {
    send({ type: 'new_session' })
  }, [send])

  const openSidebar = useCallback(() => {
    fetchSessions()
    setShowSidebar(true)
  }, [fetchSessions])

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
      ? 'Sprite 思考中...'
      : state === 'tool_confirm'
        ? '等待工具确认...'
        : state === 'ask'
          ? '等待回答问题...'
          : '已连接'

  return (
    <div className="flex flex-col h-[100dvh] w-full max-w-[100vw] lg:max-w-[900px] xl:max-w-[1100px] mx-auto">
      {/* Header */}
      <div className="bg-bg2/95 backdrop-blur-sm border-b border-border shrink-0">
        {/* 状态栏 - 放在最上面 */}
        <div className={`px-4 py-1 text-[11px] flex items-center justify-center gap-1.5 border-b border-border/50 ${!connected ? 'text-err' : isLoading ? 'text-warn' : 'text-fg3'}`}>
          {isLoading && connected && <span className="w-1.5 h-1.5 rounded-full bg-warn animate-[pulse_1.2s_ease-in-out_infinite]" />}
          {statusText}
        </div>
        {/* 导航栏 */}
        <div className="flex items-center gap-3 px-4 pt-2.5 pb-2.5">
          <button
            className="text-fg3 hover:text-fg text-[18px] leading-none px-0.5 transition-colors"
            onClick={openSidebar}
            title="会话列表"
          >☰</button>
          <div className="flex items-center gap-2">
            <span className="text-[20px] leading-none">🦞</span>
            <span className="font-bold text-[16px] tracking-wide">Sprite</span>
          </div>
          <span className="w-px h-4 bg-border" />
          <span className="text-label-ai text-[12px] font-medium">{modelName}</span>
          <div className="ml-auto flex items-center gap-2">
            <span className={`w-2 h-2 rounded-full shrink-0 transition-colors duration-300 ${connected ? 'bg-ok shadow-[0_0_6px_var(--color-ok)]' : 'bg-fg3'}`} />
            <span className="text-fg3 text-[11px]">{msgCount} 条消息</span>
          </div>
        </div>
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
            <ToolCallMsg
              key={`tc-${i}-${m.id || ''}`}
              name={m.name}
              arguments={m.arguments}
              completed={m.completed}
              collapsed={m.collapsed}
              onDetail={() => setDetailMessage(m)}
            />
          ) : m.role === 'tool_result' ? (
            <ToolResultMsg
              key={`tr-${i}`}
              toolName={m.toolName}
              output={m.output}
              isError={m.isError}
              collapsed={m.collapsed}
              onDetail={() => setDetailMessage(m)}
            />
          ) : (
            <Message
              key={i}
              role={m.role}
              content={m.content}
              streaming={m.streaming}
              onDetail={() => setDetailMessage(m)}
            />
          )
        )}
      </div>

      {/* Toast */}
      {toast && (
        <div className="px-5 py-2 text-center text-[13px] text-err bg-err/10 border-t border-err/30 shrink-0">{toast}</div>
      )}

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
            className="w-[48px] h-[48px] rounded-full border-none text-lg cursor-pointer flex items-center justify-center shrink-0 transition-all duration-150 bg-err text-white active:scale-[0.92]"
            onClick={cancelStream}
            title="取消"
          >■</button>
        )}
      </div>

      <ToolModal tools={toolConfirm} currentIndex={toolConfirmIdx} onConfirm={confirmTool} />
      <AskModal questions={askQuestions} onSubmit={submitAsk} />
      <MessageDetailModal message={detailMessage} onClose={() => setDetailMessage(null)} />

      {/* Session Sidebar */}
      {showSidebar && (
        <SessionSidebar
          sessions={sessions}
          currentSessionId={currentSessionId}
          onSwitch={switchSession}
          onNew={newSession}
          onClose={() => setShowSidebar(false)}
        />
      )}
    </div>
  )
}
