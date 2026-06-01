import { AlertTriangle } from './Icon'

interface Props {
  filename: string
  onSave: () => void | Promise<void>
  onDiscard: () => void
  onCancel: () => void
}

export function CloseConfirmDialog({
  filename,
  onSave,
  onDiscard,
  onCancel,
}: Props) {
  return (
    <div className="seeyue-modal-mask" onClick={onCancel}>
      <div className="seeyue-modal" onClick={(e) => e.stopPropagation()}>
        <h3>
          <span style={{ color: 'var(--color-seeyue-warn)' }}>
            <AlertTriangle size={16} />
          </span>
          有未保存的改动
        </h3>
        <p>
          <span style={{ color: 'var(--color-seeyue-fg)' }}>{filename}</span>{' '}
          已修改但未保存，是否保存？
        </p>
        <div className="seeyue-modal-actions">
          <button className="seeyue-btn" onClick={onCancel}>
            取消
          </button>
          <button
            className="seeyue-btn"
            data-tone="danger"
            onClick={onDiscard}
          >
            不保存
          </button>
          <button
            className="seeyue-btn"
            data-tone="primary"
            onClick={() => void onSave()}
          >
            保存
          </button>
        </div>
      </div>
    </div>
  )
}
