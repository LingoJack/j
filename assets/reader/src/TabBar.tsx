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
      <div className="flex items-center h-[38px] bg-seeyue-sidebar-strong border-b border-seeyue-border overflow-x-auto overflow-y-hidden px-2 pl-3.5 gap-2 [&::-webkit-scrollbar]:h-1">
        <span className="flex-1 text-xs text-seeyue-fg-dim tracking-[0.02em]">没有打开的文件</span>
        <button
          type="button"
          className="inline-flex items-center justify-center w-[26px] h-[26px] rounded-md text-seeyue-fg-dim bg-transparent border-0 cursor-pointer transition-all duration-150 ml-auto self-center mr-1.5 shrink-0 hover:text-seeyue-danger hover:bg-[rgba(191,97,106,0.12)] disabled:opacity-30 disabled:cursor-not-allowed"
          onClick={onQuit}
          title="关闭 reader"
        >
          <Power size={14} />
        </button>
      </div>
    )
  }
  return (
    <div className="flex items-stretch h-[38px] bg-seeyue-sidebar-strong border-b border-seeyue-border overflow-x-auto overflow-y-hidden [&::-webkit-scrollbar]:h-1">
      {tabs.map((tab) => {
        const isActive = tab.path === activePath
        return (
          <div
            key={tab.path}
            className="inline-flex items-center gap-1.5 h-full px-3 pl-3.5 text-[13px] text-seeyue-fg-muted cursor-pointer relative border-r border-seeyue-border transition-colors duration-150 select-none hover:text-seeyue-fg-strong hover:bg-seeyue-elevated data-[active=true]:text-seeyue-fg-strong data-[active=true]:bg-seeyue-bg after:content-[''] after:absolute after:left-0 after:right-0 after:bottom-0 after:h-0.5 after:bg-transparent data-[active=true]:after:bg-seeyue-accent-strong"
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
            <span className="max-w-[200px] whitespace-nowrap overflow-hidden text-ellipsis">{tab.filename}</span>
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation()
                onClose(tab.path)
              }}
              className={`inline-flex items-center justify-center w-[18px] h-[18px] rounded border-0 bg-transparent text-seeyue-fg-dim cursor-pointer transition-all duration-150 shrink-0 hover:text-seeyue-fg-strong hover:bg-[rgba(191,97,106,0.4)]${tab.dirty ? ' text-seeyue-warn hover:text-seeyue-fg-strong hover:bg-[rgba(191,97,106,0.4)]' : ''}`}
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
        className="inline-flex items-center justify-center w-[26px] h-[26px] rounded-md text-seeyue-fg-dim bg-transparent border-0 cursor-pointer transition-all duration-150 ml-auto self-center mr-1.5 shrink-0 hover:text-seeyue-danger hover:bg-[rgba(191,97,106,0.12)] disabled:opacity-30 disabled:cursor-not-allowed"
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
