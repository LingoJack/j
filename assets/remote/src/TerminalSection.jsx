import { useState, useCallback, useRef, useEffect } from 'react'

export default function TerminalSection({ terminalHistory, send, onCollapse }) {
  const [command, setCommand] = useState('')
  const [history, setHistory] = useState([])
  const [cmdHistory, setCmdHistory] = useState([])
  const [cmdHistoryIdx, setCmdHistoryIdx] = useState(-1)
  const inputRef = useRef(null)
  const scrollRef = useRef(null)

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight
    }
  }, [history])

  const handleExec = useCallback(() => {
    const cmd = command.trim()
    if (!cmd) return
    setHistory(prev => [...prev, { type: 'input', text: `$ ${cmd}` }])
    setCmdHistory(prev => [cmd, ...prev])
    setCmdHistoryIdx(-1)
    setCommand('')
    send({ type: 'terminal_exec', command: cmd })
  }, [command, send])

  const handleKeyDown = useCallback((e) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      handleExec()
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      if (cmdHistory.length > 0) {
        const newIdx = Math.min(cmdHistoryIdx + 1, cmdHistory.length - 1)
        setCmdHistoryIdx(newIdx)
        setCommand(cmdHistory[newIdx] || '')
      }
    } else if (e.key === 'ArrowDown') {
      e.preventDefault()
      if (cmdHistoryIdx > 0) {
        const newIdx = cmdHistoryIdx - 1
        setCmdHistoryIdx(newIdx)
        setCommand(cmdHistory[newIdx] || '')
      } else {
        setCmdHistoryIdx(-1)
        setCommand('')
      }
    }
  }, [handleExec, cmdHistory, cmdHistoryIdx])

  // 接收来自 App 的终端输出
  useEffect(() => {
    if (terminalHistory && terminalHistory.length > 0) {
      const last = terminalHistory[terminalHistory.length - 1]
      if (last.type === 'output' && !last._consumed) {
        setHistory(prev => [...prev, { type: 'output', text: last.text, exitCode: last.exitCode }])
        last._consumed = true
      }
    }
  }, [terminalHistory])

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="sidebar-section-header">
        <span className="font-semibold text-[13px]">终端</span>
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

      {/* Output area */}
      <div
        ref={scrollRef}
        className="flex-1 overflow-y-auto bg-bg p-3 font-mono text-[12px]"
        onClick={() => inputRef.current?.focus()}
      >
        {history.length === 0 && (
          <div className="text-fg3">输入命令开始执行...</div>
        )}
        {history.map((item, i) => (
          <div key={i} className={`whitespace-pre-wrap break-all ${item.type === 'input' ? 'text-accent' : item.exitCode != null && item.exitCode !== 0 ? 'text-err' : 'text-fg2'}`}>
            {item.text}
          </div>
        ))}
      </div>

      {/* Input area */}
      <div className="px-3 py-2 border-t border-border shrink-0">
        <div className="flex items-center gap-2">
          <span className="text-accent text-[13px] font-mono shrink-0">$</span>
          <input
            ref={inputRef}
            className="flex-1 bg-bg border border-border rounded px-2 py-1.5 text-[12px] text-fg font-mono outline-none focus:border-accent"
            placeholder="输入命令..."
            value={command}
            onChange={e => setCommand(e.target.value)}
            onKeyDown={handleKeyDown}
            autoComplete="off"
            autoFocus
          />
          <button
            className="text-accent text-[11px] px-2 py-1 rounded hover:bg-accent/10 transition-colors"
            onClick={handleExec}
            disabled={!command.trim()}
          >执行</button>
        </div>
      </div>
    </div>
  )
}
