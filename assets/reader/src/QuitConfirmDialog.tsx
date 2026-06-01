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
    <div className="seeyue-modal-mask" onClick={onCancel}>
      <div className="seeyue-modal" onClick={(e) => e.stopPropagation()}>
        <h3>
          <span style={{ color: 'var(--color-seeyue-warn)' }}>
            <Power size={16} />
          </span>
          关闭 reader？
        </h3>
        <p>
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
        <div className="seeyue-modal-actions">
          <button
            ref={cancelBtnRef}
            className="seeyue-btn"
            data-tone="primary"
            onClick={onCancel}
          >
            取消
          </button>
          <button
            className="seeyue-btn"
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
