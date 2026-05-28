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
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm"
      onClick={onCancel}
    >
      <div
        className="bg-seeyue-panel border border-seeyue-border rounded-lg shadow-xl w-[420px] p-6"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="text-seeyue-fg-strong text-base font-medium mb-2">
          有未保存的改动
        </div>
        <div className="text-seeyue-fg-muted text-sm mb-6">
          <span className="text-seeyue-fg">{filename}</span> 已修改但未保存，是否保存？
        </div>
        <div className="flex justify-end gap-2">
          <button
            onClick={onCancel}
            className="px-3 py-1.5 text-sm text-seeyue-fg-muted hover:text-seeyue-fg-strong rounded transition-colors"
          >
            取消
          </button>
          <button
            onClick={onDiscard}
            className="px-3 py-1.5 text-sm text-seeyue-danger hover:bg-seeyue-border rounded transition-colors"
          >
            不保存
          </button>
          <button
            onClick={() => void onSave()}
            className="px-3 py-1.5 text-sm bg-seeyue-accent-soft text-seeyue-accent hover:bg-seeyue-accent hover:text-seeyue-bg rounded transition-colors"
          >
            保存
          </button>
        </div>
      </div>
    </div>
  )
}
