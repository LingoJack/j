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
      className="seeyue-toast"
      data-tone={kind}
      role="status"
      onClick={onClose}
    >
      <span className="seeyue-toast-icon">
        {kind === 'error' && <AlertTriangle size={16} />}
        {kind === 'success' && <CheckCircle size={16} />}
        {kind === 'info' && <Info size={16} />}
      </span>
      <span className="seeyue-toast-msg">{message}</span>
      <button
        className="seeyue-toast-close"
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
