import { AlertTriangle } from './Icon'

interface Props {
  filename: string
  onSave: () => void | Promise<void>
  onDiscard: () => void
  onCancel: () => void
}

export function CloseConfirmDialog({ filename, onSave, onDiscard, onCancel }: Props) {
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
            <AlertTriangle size={16} />
          </span>
          有未保存的改动
        </h3>
        <p className="m-0 mb-4 text-[13px] text-seeyue-fg-muted leading-[1.7]">
          <span style={{ color: 'var(--color-seeyue-fg)' }}>{filename}</span>{' '}
          已修改但未保存，是否保存？
        </p>
        <div className="flex justify-end gap-2 mt-[18px]">
          <button
            className="px-3.5 py-1.5 text-[12.5px] font-cjk rounded-md border border-transparent cursor-pointer bg-transparent text-seeyue-fg-muted transition-all duration-150 hover:text-seeyue-fg-strong hover:bg-seeyue-elevated data-[tone=primary]:bg-seeyue-accent-strong data-[tone=primary]:text-seeyue-fg-strong data-[tone=primary]:hover:bg-seeyue-accent data-[tone=primary]:hover:text-seeyue-bg data-[tone=danger]:text-seeyue-danger data-[tone=danger]:hover:bg-[rgba(191,97,106,0.18)]"
            onClick={onCancel}
          >
            取消
          </button>
          <button
            className="px-3.5 py-1.5 text-[12.5px] font-cjk rounded-md border border-transparent cursor-pointer bg-transparent text-seeyue-fg-muted transition-all duration-150 hover:text-seeyue-fg-strong hover:bg-seeyue-elevated data-[tone=primary]:bg-seeyue-accent-strong data-[tone=primary]:text-seeyue-fg-strong data-[tone=primary]:hover:bg-seeyue-accent data-[tone=primary]:hover:text-seeyue-bg data-[tone=danger]:text-seeyue-danger data-[tone=danger]:hover:bg-[rgba(191,97,106,0.18)]"
            data-tone="danger"
            onClick={onDiscard}
          >
            不保存
          </button>
          <button
            className="px-3.5 py-1.5 text-[12.5px] font-cjk rounded-md border border-transparent cursor-pointer bg-transparent text-seeyue-fg-muted transition-all duration-150 hover:text-seeyue-fg-strong hover:bg-seeyue-elevated data-[tone=primary]:bg-seeyue-accent-strong data-[tone=primary]:text-seeyue-fg-strong data-[tone=primary]:hover:bg-seeyue-accent data-[tone=primary]:hover:text-seeyue-bg data-[tone=danger]:text-seeyue-danger data-[tone=danger]:hover:bg-[rgba(191,97,106,0.18)]"
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
