import {
  Braces,
  Close,
  FileMd,
  FileGeneric,
  FileText,
  FileCode,
  FileImage,
  GitCompare,
  Power,
  Toolbox as ToolboxIcon,
  pickFileIconKind,
} from './Icon'
import type { Tab, ToolId } from './types'

interface Props {
  tabs: Tab[]
  activePath: string | null
  onActivate: (path: string) => void
  onClose: (path: string) => void
  /** 关闭整个 reader（会先弹确认弹窗） */
  onQuit: () => void
}

export function TabBar({ tabs, activePath, onActivate, onClose, onQuit }: Props) {
  if (tabs.length === 0) {
    // 空态依然给一条「关闭 reader」入口，免得只能在编辑器里才能找到
    return (
      <div className="seeyue-tabbar seeyue-tabbar-empty">
        <span className="seeyue-tabbar-empty-hint">没有打开的文件</span>
        <button
          type="button"
          className="seeyue-icon-btn seeyue-tabbar-quit"
          onClick={onQuit}
          title="关闭 reader"
        >
          <Power size={14} />
        </button>
      </div>
    )
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
              <TabIcon tab={tab} />
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
      <button
        type="button"
        className="seeyue-icon-btn seeyue-tabbar-quit"
        onClick={onQuit}
        title="关闭 reader"
      >
        <Power size={14} />
      </button>
    </div>
  )
}

function TabIcon({ tab }: { tab: Tab }) {
  if (tab.kind === 'tool') {
    return <ToolIcon toolId={tab.toolId ?? null} />
  }
  switch (pickFileIconKind(tab.filename)) {
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

function ToolIcon({ toolId }: { toolId: ToolId | null }) {
  switch (toolId) {
    case 'diff':
      return <GitCompare size={13} />
    case 'json':
      return <Braces size={13} />
    default:
      return <ToolboxIcon size={13} />
  }
}
