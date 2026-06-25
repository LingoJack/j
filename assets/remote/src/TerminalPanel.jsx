import { useState, useCallback, useRef, useEffect } from 'react'

const THEMES = {
  dark: {
    bg: '#0d1117', bar: '#161b22', border: '#30363d',
    muted: '#8b949e', dim: '#484f58',
    prompt: '#3fb950', cmd: '#58a6ff', output: '#c9d1d9', err: '#ff7b72',
    running: '#d29922', input: '#e6edf3',
    dot1: '#ff5f57', dot2: '#febc2e', dot3: '#28c840',
  },
  light: {
    bg: '#f6f8fa', bar: '#eaeef2', border: '#d0d7de',
    muted: '#656d76', dim: '#8b949e',
    prompt: '#1a7f37', cmd: '#0969da', output: '#24292f', err: '#cf222e',
    running: '#9a6700', input: '#24292f',
    dot1: '#ff5f57', dot2: '#febc2e', dot3: '#28c840',
  },
}

export default function TerminalPanel({ terminalHistory, send, onBack, theme }) {
  const [command, setCommand] = useState('')
  const [history, setHistory] = useState([])
  const [cmdHistory, setCmdHistory] = useState([])
  const [cmdHistoryIdx, setCmdHistoryIdx] = useState(-1)
  const [executing, setExecuting] = useState(false)
  const inputRef = useRef(null)
  const scrollRef = useRef(null)
  const lastConsumedIdx = useRef(-1)
  const composingRef = useRef(false)
  const c = THEMES[theme === 'light' ? 'light' : 'dark']

  useEffect(() => {
    if (scrollRef.current) scrollRef.current.scrollTop = scrollRef.current.scrollHeight
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
    // IME 组合期间（中文输入法等）不拦截任何按键，避免干扰输入法提交
    // 移动端 isComposing/keyCode 检测不可靠，优先使用 composition 事件 ref
    if (composingRef.current || e.nativeEvent.isComposing || e.keyCode === 229) return
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
    } else if (e.key === 'c' && e.ctrlKey && executing) {
      e.preventDefault()
      send({ type: 'terminal_interrupt' })
      setExecuting(false)
      setHistory(prev => [...prev, { type: 'output', text: '^C', exitCode: 130 }])
    }
  }, [handleExec, cmdHistory, cmdHistoryIdx, executing, send])

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

  const handleAreaClick = useCallback(() => { inputRef.current?.focus() }, [])

  useEffect(() => {
    if (scrollRef.current) scrollRef.current.scrollTop = scrollRef.current.scrollHeight
  }, [command])

  return (
    <div className="flex flex-col h-full" style={{ background: c.bg }}>
      {/* 顶栏 */}
      <div className="flex items-center gap-3 px-4 py-2 shrink-0" style={{ background: c.bar, borderBottom: `1px solid ${c.border}` }}>
        <button
          className="text-[12px] px-2 py-1 rounded transition-all duration-100 select-none active:scale-[0.92]"
          style={{ color: c.muted }}
          onClick={onBack}
          onMouseEnter={e => e.target.style.color = c.output}
          onMouseLeave={e => e.target.style.color = c.muted}
        >← 聊天</button>
        <div className="flex items-center gap-1.5">
          <span className="w-2.5 h-2.5 rounded-full" style={{ background: c.dot1 }} />
          <span className="w-2.5 h-2.5 rounded-full" style={{ background: c.dot2 }} />
          <span className="w-2.5 h-2.5 rounded-full" style={{ background: c.dot3 }} />
        </div>
        <span className="text-[12px] font-mono" style={{ color: c.muted }}>j — terminal</span>
        <div className="ml-auto">
          {executing && <span className="text-[11px] font-mono" style={{ color: c.running }}>● running</span>}
        </div>
      </div>

      {/* 终端主体 */}
      <div
        ref={scrollRef}
        className="flex-1 overflow-y-auto px-4 py-2 font-mono text-[13px] leading-[1.5] cursor-text"
        style={{ background: c.bg }}
        onClick={handleAreaClick}
      >
        {history.length === 0 && !executing && (
          <div className="mb-2" style={{ color: c.dim }}>Type a command and press Enter. (Ctrl+C to interrupt)</div>
        )}
        {history.map((item, i) => (
          <div key={i}>
            {item.type === 'input' ? (
              <div style={{ color: c.cmd }}>
                <span className="select-none" style={{ color: c.prompt }}>$ </span>{item.text}
              </div>
            ) : (
              <div className="whitespace-pre" style={{ color: item.exitCode != null && item.exitCode !== 0 ? c.err : c.output }}>
                {item.text}
              </div>
            )}
          </div>
        ))}
        <div className="flex items-center text-[13px]">
          <span className="shrink-0 select-none" style={{ color: c.prompt }}>$&nbsp;</span>
          <input
            ref={inputRef}
            className="flex-1 bg-transparent border-none font-mono outline-none min-w-0"
            style={{ color: c.input }}
            placeholder={executing ? '...' : ''}
            value={command}
            onChange={e => setCommand(e.target.value)}
            onKeyDown={handleKeyDown}
            onCompositionStart={() => { composingRef.current = true }}
            onCompositionEnd={e => {
              composingRef.current = false
              setCommand(e.target.value)
            }}
            autoComplete="off"
            autoFocus
            spellCheck={false}
          />
        </div>
      </div>
    </div>
  )
}
