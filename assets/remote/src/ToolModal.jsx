import { useState } from 'react'

const optCfg = [
  { label: '允许执行', icon: '▶', border: 'border-ok', text: 'text-ok', hover: 'hover:bg-ok/10' },
  { label: '始终允许', icon: '✓', border: 'border-warn', text: 'text-warn', hover: 'hover:bg-warn/10' },
  { label: '拒绝', icon: '✗', border: 'border-err', text: 'text-err', hover: 'hover:bg-err/10' },
  { label: '输入原因拒绝...', icon: '✎', border: 'border-fg2', text: 'text-fg2', hover: 'hover:bg-fg2/10' },
]

export default function ToolModal({ tools, currentIndex, onConfirm }) {
  const [mode, setMode] = useState('select')
  const [reason, setReason] = useState('')
  const [selected, setSelected] = useState(0)

  if (!tools || tools.length === 0) return null

  const tool = tools[currentIndex] || tools[0]
  const total = tools.length
  const remaining = total - currentIndex

  const handleConfirm = (idx) => {
    switch (idx) {
      case 0: onConfirm('allow'); break
      case 1: onConfirm('allow_always'); break
      case 2: onConfirm('reject'); break
      case 3: setMode('input'); setReason(''); break
    }
  }

  const submitReason = () => {
    onConfirm('reject_with_reason', reason.trim() || '用户拒绝')
    setMode('select')
    setReason('')
  }

  return (
    <div className="fixed inset-0 bg-black/65 backdrop-blur-sm flex items-center justify-center z-100 p-[calc(env(safe-area-inset-top)+8px)_calc(env(safe-area-inset-right)+8px)_calc(env(safe-area-inset-bottom)+8px)_calc(env(safe-area-inset-left)+8px)]">
      <div className="bg-[#231e0f] border border-[#806830] rounded-[20px] p-5.5 w-[92%] max-w-[420px] shadow-[0_20px_60px_rgba(0,0,0,0.5)]">
        {/* Header */}
        <div className="flex items-center gap-2 text-base font-bold mb-4">
          <span className="text-xl">🔧</span>
          <span className="text-tool-confirm">工具调用确认</span>
          {total > 1 && (
            <span className="ml-auto text-[11px] font-medium bg-warn/20 text-warn px-2 py-0.5 rounded-[10px]">{remaining} 个待确认</span>
          )}
        </div>

        {/* Tool Detail */}
        <div className="bg-[#1a180e] border border-[#504020] rounded-xl p-3.5 mb-4">
          <div className="flex items-center gap-2 mb-2">
            <span className="text-[11px] text-fg3 uppercase tracking-wide">工具</span>
            <span className="font-bold text-tool-confirm text-sm">{tool.name}</span>
          </div>
          <div className="text-[13px] text-fg2 leading-relaxed break-all max-h-40 overflow-y-auto">{tool.confirm_message}</div>
        </div>

        {/* Options / Reason Input */}
        {mode === 'select' ? (
          <div className="flex flex-col gap-2">
            {optCfg.map((opt, i) => (
              <button
                key={i}
                className={`flex items-center gap-2.5 px-4 py-3 border rounded-xl bg-transparent text-sm cursor-pointer transition-all duration-150 text-left active:scale-[0.98] ${opt.border} ${opt.text} ${opt.hover}`}
                onClick={() => handleConfirm(i)}
                onMouseEnter={() => setSelected(i)}
              >
                <span className="text-base w-5 text-center shrink-0">{opt.icon}</span>
                {opt.label}
              </button>
            ))}
          </div>
        ) : (
          <div className="flex flex-col gap-2.5">
            <input
              type="text"
              className="w-full px-4 py-3 border border-border rounded-xl bg-bg text-fg text-sm outline-none font-[inherit] focus:border-warn"
              placeholder="输入拒绝原因..."
              value={reason}
              onChange={e => setReason(e.target.value)}
              onKeyDown={e => {
                if (e.key === 'Enter') submitReason()
                if (e.key === 'Escape') { setMode('select'); setReason('') }
              }}
              autoFocus
            />
            <div className="flex gap-2 justify-end">
              <button className="px-4.5 py-2 border-none rounded-[10px] text-[13px] font-semibold cursor-pointer bg-bg3 text-fg2" onClick={() => { setMode('select'); setReason('') }}>取消</button>
              <button className="px-4.5 py-2 border-none rounded-[10px] text-[13px] font-semibold cursor-pointer bg-err text-white" onClick={submitReason}>提交</button>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
