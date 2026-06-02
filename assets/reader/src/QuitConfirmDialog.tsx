import { useEffect, useRef } from 'react'
import { Power } from './Icon'

interface Props {
  /** dirty tab 数量（>0 时额外提示「未保存改动会丢失」） */
  dirtyCount: number
  onConfirm: () => void
  onCancel: () => void
}

/**
 * 关闭整个 reader 之前的二次确认。
 *
 * 之所以独立于 CloseConfirmDialog：
 * - 关单个 tab 是「保存 / 不保存 / 取消」三选项；
 * - 关 reader 是「关 / 不关」二选项，且默认聚焦在「取消」上 ——
 *   防止用户在 dirty 弹窗里连按导致直接干掉整个窗口。
 */
export function QuitConfirmDialog({ dirtyCount, onConfirm, onCancel }: Props) {
  const cancelBtnRef = useRef<HTMLButtonElement>(null)

  // 默认聚焦「取消」：连按 Enter / Space 都只会取消，不会关 reader
  useEffect(() => {
    cancelBtnRef.current?.focus()
  }, [])

  // Esc 也走取消（mask 点击同义）
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key === 'Escape') {
        e.stopPropagation()
        onCancel()
      }
    }
    window.addEventListener('keydown', onKey, true)
    return () => window.removeEventListener('keydown', onKey, true)
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
          <span style={{ color: 'var(--color-seeyue-warn)' }}>
            <Power size={16} />
          </span>
          关闭 reader？
        </h3>
        <p className="m-0 mb-4 text-[13px] text-seeyue-fg-muted leading-[1.7]">
          确认后会通知服务端 shutdown 并尝试关闭浏览器窗口。
          {dirtyCount > 0 && (
            <>
              <br />
              <span style={{ color: 'var(--color-seeyue-warn)' }}>
                还有 {dirtyCount} 个文件未保存，关闭后改动会丢失。
              </span>
            </>
          )}
        </p>
        <div className="flex justify-end gap-2 mt-[18px]">
          <button
            ref={cancelBtnRef}
            className="px-3.5 py-1.5 text-[12.5px] font-cjk rounded-md border border-transparent cursor-pointer bg-transparent text-seeyue-fg-muted transition-all duration-150 hover:text-seeyue-fg-strong hover:bg-seeyue-elevated data-[tone=primary]:bg-seeyue-accent-strong data-[tone=primary]:text-seeyue-fg-strong data-[tone=primary]:hover:bg-seeyue-accent data-[tone=primary]:hover:text-seeyue-bg data-[tone=danger]:text-seeyue-danger data-[tone=danger]:hover:bg-[rgba(191,97,106,0.18)]"
            data-tone="primary"
            onClick={onCancel}
          >
            取消
          </button>
          <button
            className="px-3.5 py-1.5 text-[12.5px] font-cjk rounded-md border border-transparent cursor-pointer bg-transparent text-seeyue-fg-muted transition-all duration-150 hover:text-seeyue-fg-strong hover:bg-seeyue-elevated data-[tone=primary]:bg-seeyue-accent-strong data-[tone=primary]:text-seeyue-fg-strong data-[tone=primary]:hover:bg-seeyue-accent data-[tone=primary]:hover:text-seeyue-bg data-[tone=danger]:text-seeyue-danger data-[tone=danger]:hover:bg-[rgba(191,97,106,0.18)]"
            data-tone="danger"
            onClick={onConfirm}
          >
            关闭 reader
          </button>
        </div>
      </div>
    </div>
  )
}
