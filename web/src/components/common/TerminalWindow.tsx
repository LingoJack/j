import type { ReactNode } from 'react'

export type Platform = 'unix' | 'windows'

interface TabItem {
  id: Platform
  label: string
  shortLabel: string
  icon: string
}

const tabs: TabItem[] = [
  { id: 'unix', label: 'macOS / Linux', shortLabel: 'Mac', icon: '\u2318' },
  { id: 'windows', label: 'Windows', shortLabel: 'Win', icon: '\u229E' },
]

interface TerminalWindowProps {
  children: ReactNode
  platform: Platform
  onPlatformChange: (platform: Platform) => void
  title?: string
  className?: string
  /** Whether to show dark variant (for dark backgrounds) */
  dark?: boolean
  /** Whether to show tabs for platform switching */
  showTabs?: boolean
  /** Size variant */
  size?: 'sm' | 'lg'
}

function TabButton({
  tab,
  isActive,
  dark,
  onClick,
}: {
  tab: TabItem
  isActive: boolean
  dark: boolean
  onClick: () => void
}) {
  return (
    <button
      onClick={onClick}
      className={`
        flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-md transition-all
        ${
          isActive
            ? dark
              ? 'bg-stone-600 text-white shadow-sm'
              : 'bg-white text-stone-800 shadow-sm'
            : dark
              ? 'text-stone-400 hover:text-stone-200'
              : 'text-stone-500 hover:text-stone-700'
        }
      `}
    >
      <span>{tab.icon}</span>
      <span className="hidden sm:inline">{tab.label}</span>
      <span className="sm:hidden">{tab.shortLabel}</span>
    </button>
  )
}

export function TerminalWindow({
  children,
  platform,
  onPlatformChange,
  title = 'Terminal',
  className = '',
  dark = false,
  showTabs = true,
  size = 'lg',
}: TerminalWindowProps) {
  const currentTab = tabs.find((t) => t.id === platform) ?? tabs[0]

  return (
    <div
      className={`
        rounded-xl overflow-hidden shadow-2xl
        ${dark ? 'shadow-black/30' : 'shadow-stone-900/10'}
        ${className}
      `}
    >
      {/* Title bar */}
      <div
        className={`
          flex items-center px-4 py-2.5
          ${dark ? 'bg-[#2d2d2d]' : 'bg-[#e8e6e1]'}
        `}
      >
        {/* Traffic lights */}
        <div className="flex gap-2 mr-4">
          <span className="w-3 h-3 rounded-full bg-[#ff5f57]" />
          <span className="w-3 h-3 rounded-full bg-[#febc2e]" />
          <span className="w-3 h-3 rounded-full bg-[#28c840]" />
        </div>

        {/* Title */}
        <span
          className={`
            text-xs font-medium
            ${dark ? 'text-stone-400' : 'text-stone-500'}
          `}
        >
          {title}
        </span>
      </div>

      {/* Tab bar */}
      {showTabs && (
        <div
          className={`
            flex items-center gap-1 px-4 py-2 border-b
            ${dark ? 'bg-[#252526] border-stone-700' : 'bg-[#f5f4f0] border-stone-200'}
          `}
        >
          {tabs.map((tab) => (
            <TabButton
              key={tab.id}
              tab={tab}
              isActive={tab.id === platform}
              dark={dark}
              onClick={() => onPlatformChange(tab.id)}
            />
          ))}
        </div>
      )}

      {/* Content area */}
      <div
        className={`
          bg-[#1e1e1e] relative group
          ${size === 'sm' ? 'px-3 py-3' : 'px-4 py-4'}
        `}
      >
        <div className="flex items-start gap-2 font-mono">
          <span className="text-emerald-400 shrink-0 select-none text-sm leading-relaxed">
            {currentTab.id === 'windows' ? '>' : '$'}
          </span>
          <div
            className={`
              flex-1 overflow-x-auto leading-relaxed
              ${size === 'sm' ? 'text-xs text-stone-300' : 'text-sm text-stone-200'}
            `}
          >
            {children}
          </div>
        </div>
      </div>
    </div>
  )
}
