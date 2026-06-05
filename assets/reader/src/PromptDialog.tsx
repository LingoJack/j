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
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-[rgba(20,22,27,0.55)] backdrop-blur-[4px] animate-seeyue-fade-in"
      onClick={onCancel}
    >
      <div
        className="min-w-[380px] max-w-[480px] bg-seeyue-panel border border-seeyue-border-strong rounded-[10px] shadow-[0_12px_40px_rgba(0,0,0,0.4)] px-5 py-5 pb-4 animate-seeyue-scale-in"
        onClick={(e) => e.stopPropagation()}
      >
        <h3 className="m-0 mb-1.5 text-[15px] font-semibold text-seeyue-fg-strong flex items-center gap-2">
          <Pencil size={16} /> {title}
        </h3>
        {description && (
          <p className="m-0 mb-4 text-[13px] text-seeyue-fg-muted leading-[1.7]">{description}</p>
        )}
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
          className="w-full px-2.5 py-2 bg-seeyue-bg-deep border border-seeyue-border-strong rounded-md text-seeyue-fg text-[13px] font-mono outline-none transition-colors duration-150 focus:border-seeyue-accent"
        />
        {error && (
          <p className="!mt-2 px-2.5 py-1.5 !text-xs !text-seeyue-danger bg-[rgba(191,97,106,0.12)] rounded-md whitespace-pre-wrap">
            {error}
          </p>
        )}
        <div className="flex justify-end gap-2 mt-[18px]">
          <button
            className="px-3.5 py-1.5 text-[12.5px] font-cjk rounded-md border border-transparent cursor-pointer bg-transparent text-seeyue-fg-muted transition-all duration-150 hover:text-seeyue-fg-strong hover:bg-seeyue-elevated data-[tone=primary]:bg-seeyue-accent-strong data-[tone=primary]:text-seeyue-fg-strong data-[tone=primary]:hover:bg-seeyue-accent data-[tone=primary]:hover:text-seeyue-bg data-[tone=danger]:text-seeyue-danger data-[tone=danger]:hover:bg-[rgba(191,97,106,0.18)]"
            onClick={onCancel}
          >
            {cancelLabel}
          </button>
          <button
            className="px-3.5 py-1.5 text-[12.5px] font-cjk rounded-md border border-transparent cursor-pointer bg-transparent text-seeyue-fg-muted transition-all duration-150 hover:text-seeyue-fg-strong hover:bg-seeyue-elevated data-[tone=primary]:bg-seeyue-accent-strong data-[tone=primary]:text-seeyue-fg-strong data-[tone=primary]:hover:bg-seeyue-accent data-[tone=primary]:hover:text-seeyue-bg data-[tone=danger]:text-seeyue-danger data-[tone=danger]:hover:bg-[rgba(191,97,106,0.18)]"
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
