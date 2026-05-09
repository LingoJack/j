import { useState, useCallback } from 'react'

export default function AskModal({ questions, onSubmit }) {
  const [currentIdx, setCurrentIdx] = useState(0)
  const [answers, setAnswers] = useState({})
  const [selected, setSelected] = useState([])
  const [freeText, setFreeText] = useState('')
  const [mode, setMode] = useState('select') // 'select' | 'input'

  if (!questions || questions.length === 0) return null

  const q = questions[currentIdx]
  const total = questions.length

  const toggleOption = (i) => {
    if (q.multi_select) {
      setSelected(prev => {
        const next = [...prev]
        next[i] = !next[i]
        return next
      })
    } else {
      // 单选：直接选中并提交
      const newAnswers = { ...answers, [q.question]: q.options[i].label }
      if (currentIdx + 1 < total) {
        setAnswers(newAnswers)
        setCurrentIdx(currentIdx + 1)
        setSelected([])
        setFreeText('')
        setMode('select')
      } else {
        onSubmit(newAnswers)
      }
    }
  }

  const submitMulti = () => {
    const labels = q.options
      .filter((_, i) => selected[i])
      .map(o => o.label)
      .join(', ')
    if (!labels) return
    const newAnswers = { ...answers, [q.question]: labels }
    advance(newAnswers)
  }

  const submitFreeText = () => {
    const text = freeText.trim()
    if (!text) return
    const newAnswers = { ...answers, [q.question]: text }
    advance(newAnswers)
  }

  const advance = (newAnswers) => {
    if (currentIdx + 1 < total) {
      setAnswers(newAnswers)
      setCurrentIdx(currentIdx + 1)
      setSelected([])
      setFreeText('')
      setMode('select')
    } else {
      onSubmit(newAnswers)
    }
  }

  return (
    <div className="fixed inset-0 bg-black/65 backdrop-blur-sm flex items-center justify-center z-100 p-[calc(env(safe-area-inset-top)+8px)_calc(env(safe-area-inset-right)+8px)_calc(env(safe-area-inset-bottom)+8px)_calc(env(safe-area-inset-left)+8px)]">
      <div className="bg-bg2 border border-border rounded-lg p-5 w-[92%] max-w-[420px] shadow-xl">
        {/* Header */}
        <div className="flex items-center gap-2 text-base font-bold mb-4">
          <span className="text-accent text-lg">◉</span>
          <span className="text-accent">
            {q.header || '请回答'}
          </span>
          {total > 1 && (
            <span className="ml-auto text-[11px] font-medium bg-accent/20 text-accent px-2 py-0.5 rounded-lg">
              {currentIdx + 1} / {total}
            </span>
          )}
        </div>

        {/* Question */}
        <div className="text-[14px] text-fg leading-relaxed mb-4">{q.question}</div>

        {mode === 'select' ? (
          <>
            {/* Options */}
            <div className="flex flex-col gap-2 mb-3">
              {q.options.map((opt, i) => (
                <button
                  key={i}
                  className={`flex items-start gap-2.5 px-4 py-3 border rounded-lg bg-transparent text-sm cursor-pointer transition-all duration-150 text-left active:scale-[0.98] ${
                    selected[i]
                      ? 'border-accent text-accent bg-accent/10'
                      : 'border-border text-fg hover:bg-accent/5'
                  }`}
                  onClick={() => toggleOption(i)}
                >
                  {q.multi_select && (
                    <span className="text-base w-5 text-center shrink-0 mt-px">
                      {selected[i] ? '☑' : '☐'}
                    </span>
                  )}
                  <div>
                    <div className="font-medium">{opt.label}</div>
                    {opt.description && (
                      <div className="text-fg2 text-xs mt-0.5">{opt.description}</div>
                    )}
                  </div>
                </button>
              ))}
            </div>

            {/* Actions */}
            <div className="flex gap-2 justify-between items-center">
              <button
                className="px-4 py-2 border-none rounded-lg text-[13px] font-medium cursor-pointer bg-bg3 text-fg2 hover:text-fg"
                onClick={() => { setMode('input'); setFreeText('') }}
              >
                ✎ 自由输入
              </button>
              {q.multi_select && (
                <button
                  className="px-5 py-2 border-none rounded-lg text-[13px] font-semibold cursor-pointer bg-accent text-bg disabled:opacity-30"
                  onClick={submitMulti}
                  disabled={!selected.some(Boolean)}
                >
                  确认
                </button>
              )}
            </div>
          </>
        ) : (
          /* Free text input mode */
          <div className="flex flex-col gap-2.5">
            <input
              type="text"
              className="w-full px-4 py-3 border border-accent/30 rounded-lg bg-bg text-fg text-sm outline-none font-[inherit] focus:border-accent"
              placeholder="输入你的回答..."
              value={freeText}
              onChange={e => setFreeText(e.target.value)}
              onKeyDown={e => {
                if (e.key === 'Enter') submitFreeText()
                if (e.key === 'Escape') setMode('select')
              }}
              autoFocus
            />
            <div className="flex gap-2 justify-end">
              <button
                className="px-4.5 py-2 border-none rounded-lg text-[13px] font-semibold cursor-pointer bg-bg3 text-fg2"
                onClick={() => setMode('select')}
              >
                返回
              </button>
              <button
                className="px-4.5 py-2 border-none rounded-lg text-[13px] font-semibold cursor-pointer bg-accent text-bg disabled:opacity-30"
                onClick={submitFreeText}
                disabled={!freeText.trim()}
              >
                提交
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}

