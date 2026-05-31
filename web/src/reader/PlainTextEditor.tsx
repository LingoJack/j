import { Save } from './Icon'
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
          {tab.dirty && (
            <span className="status-pill" data-tone="warn"
                  style={{ fontSize: 11, padding: '2px 8px', borderRadius: 999 }}>
              ● 未保存
            </span>
          )}
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
            className="seeyue-icon-btn"
            title="Cmd+S 保存"
          >
            <Save size={14} />
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
