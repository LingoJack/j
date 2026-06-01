import { useEffect, useRef, useState } from 'react'
import { Pencil } from './Icon'

interface Props {
  title: string
  description?: string
  initialValue?: string
  placeholder?: string
  confirmLabel?: string
  cancelLabel?: string
  /** 可选的错误提示，显示在输入框下方红字 */
  error?: string
  onConfirm: (value: string) => void
  onCancel: () => void
}

/**
 * Typora 风的输入对话框，替代 window.prompt(...)。
 *
 * - Esc 取消，Enter 确认
 * - 自动聚焦 + 选中所有文字（方便直接覆写）
 */
export function PromptDialog({
  title,
  description,
  initialValue = '',
  placeholder,
  confirmLabel = '确定',
  cancelLabel = '取消',
  error,
  onConfirm,
  onCancel,
}: Props) {
  const [value, setValue] = useState(initialValue)
  const ref = useRef<HTMLInputElement | null>(null)

  useEffect(() => {
    const t = window.setTimeout(() => {
      ref.current?.focus()
      ref.current?.select()
    }, 30)
    return () => window.clearTimeout(t)
  }, [])

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key === 'Escape') {
        e.preventDefault()
        onCancel()
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [onCancel])

  return (
    <div className="seeyue-modal-mask" onClick={onCancel}>
      <div className="seeyue-modal" onClick={(e) => e.stopPropagation()}>
        <h3>
          <Pencil size={16} /> {title}
        </h3>
        {description && <p>{description}</p>}
        <input
          ref={ref}
          type="text"
          value={value}
          placeholder={placeholder}
          spellCheck={false}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') {
              e.preventDefault()
              onConfirm(value.trim())
            }
          }}
        />
        {error && <p className="seeyue-modal-error">{error}</p>}
        <div className="seeyue-modal-actions">
          <button className="seeyue-btn" onClick={onCancel}>
            {cancelLabel}
          </button>
          <button
            className="seeyue-btn"
            data-tone="primary"
            onClick={() => onConfirm(value.trim())}
          >
            {confirmLabel}
          </button>
        </div>
      </div>
    </div>
  )
}
