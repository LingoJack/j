import { useEffect } from 'react'
import { AlertTriangle, CheckCircle, Close, Info } from './Icon'

interface Props {
  message: string
  /** 失败 / 成功 / 普通信息 */
  kind?: 'error' | 'info' | 'success'
  /** 自动关闭毫秒数；0 表示不自动关闭 */
  duration?: number
  onClose: () => void
}

/**
 * Typora 风 Toast：固定右上角，按 kind 选 icon + 颜色。
 * 用于替代 `alert(...)` 展示非阻塞错误/成功/信息。
 */
export function Toast({
  message,
  kind = 'error',
  duration = 3000,
  onClose,
}: Props) {
  useEffect(() => {
    if (duration <= 0) return
    const timer = window.setTimeout(onClose, duration)
    return () => window.clearTimeout(timer)
  }, [duration, onClose])

  return (
    <div
      className="group fixed top-4 right-4 z-60 max-w-[360px] flex items-start gap-2.5 px-3 py-2.5 bg-seeyue-elevated border border-seeyue-border-strong rounded-lg shadow-[0_8px_24px_rgba(0,0,0,0.35)] text-[13px] text-seeyue-fg animate-seeyue-slide-in data-[tone=error]:border-l-[3px] data-[tone=error]:border-l-seeyue-danger data-[tone=success]:border-l-[3px] data-[tone=success]:border-l-seeyue-success data-[tone=info]:border-l-[3px] data-[tone=info]:border-l-seeyue-accent"
      data-tone={kind}
      role="status"
      onClick={onClose}
    >
      <span className="shrink-0 mt-0.5 group-data-[tone=error]:text-seeyue-danger group-data-[tone=success]:text-seeyue-success group-data-[tone=info]:text-seeyue-accent">
        {kind === 'error' && <AlertTriangle size={16} />}
        {kind === 'success' && <CheckCircle size={16} />}
        {kind === 'info' && <Info size={16} />}
      </span>
      <span className="flex-1 whitespace-pre-wrap break-words">{message}</span>
      <button
        className="shrink-0 cursor-pointer bg-transparent border-0 text-seeyue-fg-dim p-0 inline-flex items-center justify-center transition-colors duration-150 hover:text-seeyue-fg-strong"
        onClick={(e) => {
          e.stopPropagation()
          onClose()
        }}
        title="关闭"
      >
        <Close size={14} />
      </button>
    </div>
  )
}
