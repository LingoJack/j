import type { Tab } from './types'

interface Props {
  tabs: Tab[]
  activePath: string | null
  onActivate: (path: string) => void
  onClose: (path: string) => void
}

export function TabBar({ tabs, activePath, onActivate, onClose }: Props) {
  if (tabs.length === 0) {
    return <div className="h-9 border-b border-seeyue-border" />
  }
  return (
    <div className="flex items-stretch h-9 border-b border-seeyue-border bg-seeyue-panel overflow-x-auto">
      {tabs.map((tab) => {
        const isActive = tab.path === activePath
        return (
          <div
            key={tab.path}
            className={`group flex items-center gap-1.5 px-3 border-r border-seeyue-border text-[13px] cursor-pointer transition-colors ${
              isActive
                ? 'bg-seeyue-bg text-seeyue-fg-strong'
                : 'text-seeyue-fg-muted hover:text-seeyue-fg hover:bg-seeyue-bg/40'
            }`}
            onClick={() => onActivate(tab.path)}
            title={tab.path}
          >
            {tab.dirty && (
              <span className="text-seeyue-warn text-xs leading-none">●</span>
            )}
            <span className="truncate max-w-[200px]">{tab.filename}</span>
            <button
              onClick={(e) => {
                e.stopPropagation()
                onClose(tab.path)
              }}
              className="ml-1 w-4 h-4 flex items-center justify-center text-seeyue-fg-dim hover:text-seeyue-fg-strong hover:bg-seeyue-border rounded"
              title="关闭"
            >
              ×
            </button>
          </div>
        )
      })}
    </div>
  )
}
