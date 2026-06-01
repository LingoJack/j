import { Close, FileMd, FileGeneric, FileText, FileCode, FileImage, pickFileIconKind } from './Icon'
import type { Tab } from './types'

interface Props {
  tabs: Tab[]
  activePath: string | null
  onActivate: (path: string) => void
  onClose: (path: string) => void
}

export function TabBar({ tabs, activePath, onActivate, onClose }: Props) {
  if (tabs.length === 0) {
    return <div className="h-9 border-b border-seeyue-border bg-seeyue-sidebar-strong" />
  }
  return (
    <div className="seeyue-tabbar">
      {tabs.map((tab) => {
        const isActive = tab.path === activePath
        return (
          <div
            key={tab.path}
            className="seeyue-tab-pill"
            data-active={isActive ? 'true' : undefined}
            onClick={() => onActivate(tab.path)}
            onAuxClick={(e) => {
              if (e.button === 1) onClose(tab.path)
            }}
            title={tab.path}
          >
            <span
              className="inline-flex items-center justify-center"
              style={{
                width: 14,
                height: 14,
                color: isActive
                  ? 'var(--color-seeyue-accent)'
                  : 'var(--color-seeyue-fg-muted)',
              }}
            >
              <TabIcon name={tab.filename} />
            </span>
            <span className="tab-name">{tab.filename}</span>
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation()
                onClose(tab.path)
              }}
              className={`tab-close ${tab.dirty ? 'tab-dirty-mark' : ''}`}
              title={tab.dirty ? '有未保存改动 · 关闭' : '关闭'}
            >
              {tab.dirty ? (
                <span style={{ fontSize: 12, lineHeight: 1 }}>●</span>
              ) : (
                <Close size={12} />
              )}
            </button>
          </div>
        )
      })}
    </div>
  )
}

function TabIcon({ name }: { name: string }) {
  switch (pickFileIconKind(name)) {
    case 'markdown':
      return <FileMd size={13} />
    case 'text':
      return <FileText size={13} />
    case 'code':
      return <FileCode size={13} />
    case 'image':
      return <FileImage size={13} />
    default:
      return <FileGeneric size={13} />
  }
}
