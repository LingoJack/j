import { useEffect } from 'react'

interface Props {
  message: string
  /** 失败 / 成功 / 普通信息 */
  kind?: 'error' | 'info' | 'success'
  /** 自动关闭毫秒数；0 表示不自动关闭 */
  duration?: number
  onClose: () => void
}

/**
 * 简易 Toast：固定右上角，3 秒后自动消失。
 * 用于替代 `alert(...)` 展示非阻塞错误信息。
 */
export function Toast({ message, kind = 'error', duration = 3000, onClose }: Props) {
  useEffect(() => {
    if (duration <= 0) return
    const timer = window.setTimeout(onClose, duration)
    return () => window.clearTimeout(timer)
  }, [duration, onClose])

  const palette =
    kind === 'error'
      ? 'border-seeyue-danger/60 bg-seeyue-panel text-seeyue-danger'
      : kind === 'success'
        ? 'border-seeyue-accent/40 bg-seeyue-panel text-seeyue-accent'
        : 'border-seeyue-border bg-seeyue-panel text-seeyue-fg'

  return (
    <div
      className={`fixed top-4 right-4 z-50 max-w-md rounded-md border px-4 py-2.5 text-sm shadow-lg backdrop-blur-sm ${palette}`}
      role="status"
      onClick={onClose}
    >
      <div className="flex items-start gap-2">
        <span className="break-words whitespace-pre-wrap flex-1">{message}</span>
        <button
          onClick={(e) => {
            e.stopPropagation()
            onClose()
          }}
          className="text-seeyue-fg-dim hover:text-seeyue-fg-strong text-xs leading-none"
          title="关闭"
        >
          ✕
        </button>
      </div>
    </div>
  )
}
