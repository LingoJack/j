/**
 * VS Code 风的最左侧“活动栏”。
 *
 * 顶部负责文件 / 工具箱视图切换，底部保留设置入口：当前用于主题切换，后续可继续
 * 挂载字体大小、默认预览模式、自动保存、TOC 行为等 Reader 偏好。
 */
import { useState } from 'react'
import { CheckCircle, Files, Settings, Toolbox as ToolboxIcon } from './Icon'

export type ActivityKey = 'files' | 'toolbox'
export type ReaderTheme = 'aliyun' | 'warm'

interface Props {
  active: ActivityKey
  theme: ReaderTheme
  onSelect: (key: ActivityKey) => void
  onThemeChange: (theme: ReaderTheme) => void
}

interface ItemDef {
  key: ActivityKey
  title: string
  Icon: typeof Files
}

const ITEMS: ItemDef[] = [
  { key: 'files', title: '文件 (⌘1)', Icon: Files },
  { key: 'toolbox', title: '工具箱 (⌘2)', Icon: ToolboxIcon },
]

const THEME_OPTIONS: Array<{ key: ReaderTheme; label: string; desc: string }> = [
  { key: 'aliyun', label: 'Aliyun Light', desc: '白底、轻边框，适合文档阅读' },
  { key: 'warm', label: 'Seeyue Warm', desc: '保留原来的暖色编辑氛围' },
]

export function ActivityBar({ active, theme, onSelect, onThemeChange }: Props) {
  const [settingsOpen, setSettingsOpen] = useState(false)

  return (
    <nav
      className="relative flex flex-col items-center gap-1 py-2 bg-seeyue-bg-deep border-r border-seeyue-border"
      aria-label="侧栏切换"
    >
      {ITEMS.map(({ key, title, Icon }) => (
        <button
          key={key}
          type="button"
          className="relative inline-flex items-center justify-center w-10 h-10 rounded-none bg-transparent border-0 text-seeyue-fg-dim cursor-pointer transition-colors duration-150 hover:text-seeyue-fg-strong hover:bg-seeyue-elevated focus-visible:outline-2 focus-visible:outline-seeyue-accent focus-visible:outline-offset-[-2px] data-[active=true]:text-seeyue-accent before:content-[''] before:absolute before:left-0 before:top-2 before:bottom-2 before:w-0.5 before:rounded-r before:bg-transparent before:transition-colors before:duration-150 data-[active=true]:before:bg-seeyue-accent"
          data-active={key === active ? 'true' : undefined}
          title={title}
          aria-label={title}
          onClick={() => onSelect(key)}
        >
          <Icon size={20} />
        </button>
      ))}

      <div className="mt-auto relative">
        <button
          type="button"
          className="relative inline-flex items-center justify-center w-10 h-10 rounded-none bg-transparent border-0 text-seeyue-fg-dim cursor-pointer transition-colors duration-150 hover:text-seeyue-fg-strong hover:bg-seeyue-elevated focus-visible:outline-2 focus-visible:outline-seeyue-accent focus-visible:outline-offset-[-2px] data-[open=true]:text-seeyue-accent data-[open=true]:bg-seeyue-elevated"
          data-open={settingsOpen ? 'true' : undefined}
          title="设置"
          aria-label="打开 Reader 设置"
          aria-expanded={settingsOpen}
          onClick={() => setSettingsOpen((open) => !open)}
        >
          <Settings size={20} />
        </button>

        {settingsOpen && (
          <div className="absolute left-[44px] bottom-0 z-30 w-[286px] rounded-lg border border-seeyue-border bg-seeyue-panel shadow-[0_12px_34px_rgba(15,23,42,0.14)] overflow-hidden animate-seeyue-scale-in">
            <div className="px-4 py-3 border-b border-seeyue-border bg-seeyue-bg">
              <div className="text-[13px] font-semibold text-seeyue-fg-strong">Reader 设置</div>
              <div className="text-[12px] leading-5 text-seeyue-fg-muted">
                当前先支持外观主题，后续设置会继续放在这里。
              </div>
            </div>
            <div className="p-3">
              <div className="px-1 pb-2 text-[11px] font-semibold tracking-[0.08em] uppercase text-seeyue-fg-dim">
                外观
              </div>
              <div className="grid gap-1">
                {THEME_OPTIONS.map((item) => {
                  const selected = item.key === theme
                  return (
                    <button
                      key={item.key}
                      type="button"
                      className="w-full flex items-start gap-2.5 rounded-md border border-transparent px-2.5 py-2 text-left cursor-pointer transition-colors duration-150 hover:bg-seeyue-elevated hover:border-seeyue-border data-[selected=true]:bg-seeyue-accent-soft data-[selected=true]:border-seeyue-border"
                      data-selected={selected ? 'true' : undefined}
                      onClick={() => onThemeChange(item.key)}
                    >
                      <span className="mt-0.5 inline-flex h-4 w-4 items-center justify-center text-seeyue-accent">
                        {selected ? <CheckCircle size={14} /> : null}
                      </span>
                      <span className="min-w-0">
                        <span className="block text-[13px] font-medium text-seeyue-fg-strong">
                          {item.label}
                        </span>
                        <span className="block text-[12px] leading-5 text-seeyue-fg-muted">
                          {item.desc}
                        </span>
                      </span>
                    </button>
                  )
                })}
              </div>
            </div>
          </div>
        )}
      </div>
    </nav>
  )
}
