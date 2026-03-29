import { useState } from 'react'

export default function ToolModal({ tools, currentIndex, onConfirm }) {
  const [mode, setMode] = useState('select') // 'select' | 'input'
  const [reason, setReason] = useState('')
  const [selected, setSelected] = useState(0)

  if (!tools || tools.length === 0) return null

  const tool = tools[currentIndex] || tools[0]
  const total = tools.length
  const remaining = total - currentIndex

  const options = [
    { label: '允许执行', icon: '▶', cls: 'opt-allow' },
    { label: '始终允许', icon: '✓', cls: 'opt-always' },
    { label: '拒绝', icon: '✗', cls: 'opt-reject' },
    { label: '输入原因拒绝...', icon: '✎', cls: 'opt-input' },
  ]

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
    <div className="modal-overlay">
      <div className="modal">
        <div className="modal-header">
          <span className="modal-icon">🔧</span>
          <span>工具调用确认</span>
          {total > 1 && (
            <span className="modal-badge">{remaining} 个待确认</span>
          )}
        </div>

        <div className="tool-detail">
          <div className="tool-name-row">
            <span className="tool-label">工具</span>
            <span className="tool-name-val">{tool.name}</span>
          </div>
          <div className="tool-desc">{tool.confirm_message}</div>
        </div>

        {mode === 'select' ? (
          <div className="tool-options">
            {options.map((opt, i) => (
              <button
                key={i}
                className={`tool-opt ${opt.cls}${selected === i ? ' active' : ''}`}
                onClick={() => handleConfirm(i)}
                onMouseEnter={() => setSelected(i)}
              >
                <span className="opt-icon">{opt.icon}</span>
                {opt.label}
              </button>
            ))}
          </div>
        ) : (
          <div className="tool-reason-input">
            <input
              type="text"
              className="reason-input"
              placeholder="输入拒绝原因..."
              value={reason}
              onChange={e => setReason(e.target.value)}
              onKeyDown={e => {
                if (e.key === 'Enter') submitReason()
                if (e.key === 'Escape') { setMode('select'); setReason('') }
              }}
              autoFocus
            />
            <div className="reason-actions">
              <button className="reason-btn cancel" onClick={() => { setMode('select'); setReason('') }}>取消</button>
              <button className="reason-btn submit" onClick={submitReason}>提交</button>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
