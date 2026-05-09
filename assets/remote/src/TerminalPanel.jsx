import { useState, useCallback, useRef, useEffect } from 'react'

export default function TerminalPanel({ terminalHistory, send, onBack }) {
  const [command, setCommand] = useState('')
  const [history, setHistory] = useState([])
  const [cmdHistory, setCmdHistory] = useState([])
  const [cmdHistoryIdx, setCmdHistoryIdx] = useState(-1)
  const [executing, setExecuting] = useState(false)
  const inputRef = useRef(null)
  const scrollRef = useRef(null)

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight
    }
  }, [history])

  const handleExec = useCallback(() => {
    const cmd = command.trim()
    if (!cmd || executing) return
    setExecuting(true)
    setHistory(prev => [...prev, { type: 'input', text: `$ ${cmd}` }])
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

  // 接收来自 App 的终端输出
  useEffect(() => {
    if (terminalHistory && terminalHistory.length > 0) {
      const last = terminalHistory[terminalHistory.length - 1]
      if (last.type === 'output' && !last._consumed) {
        setHistory(prev => [...prev, { type: 'output', text: last.text, exitCode: last.exitCode }])
        last._consumed = true
        setExecuting(false)
      }
    }
  }, [terminalHistory])

  // 点击终端区域聚焦输入
  const handleAreaClick = useCallback(() => {
    inputRef.current?.focus()
  }, [])

  return (
    <div className="flex flex-col h-full bg-[#1a1a2e]">
      {/* 顶栏 */}
      <div className="flex items-center gap-3 px-4 py-2 bg-[#16162a] border-b border-[#2a2a4a] shrink-0">
        <button
          className="text-[#6a6a9a] hover:text-[#a0a0d0] active:text-white active:scale-[0.92] text-[12px] px-2 py-1 rounded transition-all duration-100 select-none"
          onClick={onBack}
        >← 聊天</button>
        <div className="flex items-center gap-2">
          <span className="w-3 h-3 rounded-full bg-[#ff5f57]" />
          <span className="w-3 h-3 rounded-full bg-[#febc2e]" />
          <span className="w-3 h-3 rounded-full bg-[#28c840]" />
        </div>
        <span className="text-[#6a6a9a] text-[12px] font-mono">terminal</span>
        <div className="ml-auto flex items-center gap-2">
          {executing && (
            <span className="text-[#febc2e] text-[11px] font-mono animate-[pulse_1.2s_ease-in-out_infinite]">● running</span>
          )}
        </div>
      </div>

      {/* 终端输出区 */}
      <div
        ref={scrollRef}
        className="flex-1 overflow-y-auto px-4 py-3 font-mono text-[13px] leading-[1.6] cursor-text"
        onClick={handleAreaClick}
      >
        {history.length === 0 && (
          <div className="text-[#4a4a6a]">Welcome to j terminal. Type a command and press Enter.</div>
        )}
        {history.map((item, i) => (
          <div key={i} className={`whitespace-pre-wrap break-all ${item.type === 'input' ? 'text-[#28c840]' : item.exitCode != null && item.exitCode !== 0 ? 'text-[#ff5f57]' : 'text-[#c0c0e0]'}`}>
            {item.text}
          </div>
        ))}
      </div>

      {/* 输入行 */}
      <div className="px-4 py-2 border-t border-[#2a2a4a] shrink-0 flex items-center gap-2 font-mono">
        <span className="text-[#28c840] text-[13px] shrink-0 select-none">$</span>
        <input
          ref={inputRef}
          className="flex-1 bg-transparent border-none text-[#e0e0ff] text-[13px] font-mono outline-none placeholder:text-[#4a4a6a]"
          placeholder="输入命令..."
          value={command}
          onChange={e => setCommand(e.target.value)}
          onKeyDown={handleKeyDown}
          autoComplete="off"
          autoFocus
          spellCheck={false}
        />
        <button
          className={`text-[11px] px-2.5 py-1 rounded font-mono transition-all duration-100 select-none ${command.trim() && !executing ? 'text-[#28c840] hover:bg-[#28c840]/10 active:bg-[#28c840]/20 active:scale-[0.95]' : 'text-[#4a4a6a] cursor-default'}`}
          onClick={handleExec}
          disabled={!command.trim() || executing}
        >↵</button>
      </div>
    </div>
  )
}
