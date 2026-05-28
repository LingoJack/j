import type { Tab } from './types'

interface Props {
  tab: Tab
  onChange: (source: string) => void
  onSave: () => void | Promise<void>
}

export function PlainTextEditor({ tab, onChange, onSave }: Props) {
  return (
    <div className="h-full flex flex-col bg-seeyue-bg">
      <div className="flex items-center justify-between h-8 px-3 border-b border-seeyue-border text-[11px] text-seeyue-fg-dim uppercase tracking-wider">
        <span>纯文本</span>
        <span className="flex items-center gap-2">
          {tab.dirty && <span className="text-seeyue-warn text-xs">● 未保存</span>}
          {tab.saving === 'saving' && (
            <span className="text-seeyue-accent">保存中…</span>
          )}
          {tab.saving === 'error' && (
            <span className="text-seeyue-danger" title={tab.error}>
              保存失败
            </span>
          )}
          <button
            onClick={() => void onSave()}
            className="px-1.5 py-0.5 rounded text-[10px] hover:text-seeyue-accent transition-colors"
            title="Cmd+S 保存"
          >
            ⌘S
          </button>
        </span>
      </div>
      <textarea
        className="seeyue-textarea flex-1 px-5 py-4 overflow-y-auto"
        spellCheck={false}
        autoFocus
        value={tab.source}
        onChange={(e) => onChange(e.target.value)}
      />
    </div>
  )
}
