import { useState, useCallback, useRef, useEffect } from 'react'

export default function TerminalPanel({ terminalHistory, send, onBack }) {
  const [command, setCommand] = useState('')
  const [history, setHistory] = useState([])
  const [cmdHistory, setCmdHistory] = useState([])
  const [cmdHistoryIdx, setCmdHistoryIdx] = useState(-1)
  const [executing, setExecuting] = useState(false)
  const inputRef = useRef(null)
  const scrollRef = useRef(null)
  const lastConsumedIdx = useRef(-1)

  // 自动滚到底部
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight
    }
  }, [history])

  const handleExec = useCallback(() => {
    const cmd = command.trim()
    if (!cmd || executing) return
    setExecuting(true)
    setHistory(prev => [...prev, { type: 'input', text: cmd }])
    setCmdHistory(prev => [cmd, ...prev])
    setCmdHistoryIdx(-1)
    setCommand('')
    send({ type: 'terminal_exec', command: cmd })
  }, [command, send, executing])

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

  // 接收终端输出 — 逐条消费，不丢消息
  useEffect(() => {
    if (!terminalHistory || terminalHistory.length === 0) return
    const newOutputs = []
    for (let i = lastConsumedIdx.current + 1; i < terminalHistory.length; i++) {
      const item = terminalHistory[i]
      if (item.type === 'output') {
        newOutputs.push({ text: item.text, exitCode: item.exitCode })
      }
    }
    if (newOutputs.length > 0) {
      lastConsumedIdx.current = terminalHistory.length - 1
      setHistory(prev => [...prev, ...newOutputs])
      setExecuting(false)
    }
  }, [terminalHistory])

  // 点击聚焦输入
  const handleAreaClick = useCallback(() => {
    inputRef.current?.focus()
  }, [])

  // 输入变化时滚到底
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight
    }
  }, [command])

  return (
    <div className="flex flex-col h-full bg-[#0d1117]">
      {/* 顶栏 */}
      <div className="flex items-center gap-3 px-4 py-2 bg-[#161b22] border-b border-[#30363d] shrink-0">
        <button
          className="text-[#8b949e] hover:text-[#c9d1d9] active:text-white active:scale-[0.92] text-[12px] px-2 py-1 rounded transition-all duration-100 select-none"
          onClick={onBack}
        >← 聊天</button>
        <div className="flex items-center gap-1.5">
          <span className="w-2.5 h-2.5 rounded-full bg-[#ff5f57]" />
          <span className="w-2.5 h-2.5 rounded-full bg-[#febc2e]" />
          <span className="w-2.5 h-2.5 rounded-full bg-[#28c840]" />
        </div>
        <span className="text-[#8b949e] text-[12px] font-mono">j — terminal</span>
        <div className="ml-auto">
          {executing && (
            <span className="text-[#d29922] text-[11px] font-mono">● running</span>
          )}
        </div>
      </div>

      {/* 终端主体：输出 + 输入在一起 */}
      <div
        ref={scrollRef}
        className="flex-1 overflow-y-auto px-4 py-2 font-mono text-[13px] leading-[1.5] cursor-text"
        onClick={handleAreaClick}
      >
        {history.length === 0 && !executing && (
          <div className="text-[#484f58] mb-2">Type a command and press Enter.</div>
        )}
        {history.map((item, i) => (
          <div key={i}>
            {item.type === 'input' ? (
              <div className="text-[#58a6ff]">
                <span className="text-[#3fb950] select-none">$ </span>{item.text}
              </div>
            ) : (
              <div className={`whitespace-pre ${item.exitCode != null && item.exitCode !== 0 ? 'text-[#ff7b72]' : 'text-[#c9d1d9]'}`}>
                {item.text}
              </div>
            )}
          </div>
        ))}
        {/* 当前输入行 — 内嵌在输出区底部 */}
        <div className="flex items-center text-[13px]">
          <span className="text-[#3fb950] shrink-0 select-none">$&nbsp;</span>
          <input
            ref={inputRef}
            className="flex-1 bg-transparent border-none text-[#e6edf3] font-mono outline-none min-w-0"
            placeholder={executing ? '...' : ''}
            value={command}
            onChange={e => setCommand(e.target.value)}
            onKeyDown={handleKeyDown}
            autoComplete="off"
            autoFocus
            spellCheck={false}
          />
        </div>
      </div>
    </div>
  )
}
