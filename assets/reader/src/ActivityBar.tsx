/**
 * VSCode 风的最左侧"活动栏"。
 *
 * 现阶段两个槽：
 * - `files`    → 切到文件树侧栏
 * - `toolbox`  → 切到工具箱侧栏（文本 diff 等）
 *
 * 用 data-active 控制选中态视觉（左侧高亮条 + 图标点亮）。键盘聚焦时
 * 也走 :focus-visible 描边，保证 Tab 可达。
 */
import { Files, Toolbox as ToolboxIcon } from './Icon'

export type ActivityKey = 'files' | 'toolbox'

interface Props {
  active: ActivityKey
  onSelect: (key: ActivityKey) => void
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

export function ActivityBar({ active, onSelect }: Props) {
  return (
    <nav className="flex flex-col items-center gap-1 py-2 bg-seeyue-bg-deep border-r border-seeyue-border" aria-label="侧栏切换">
      {ITEMS.map(({ key, title, Icon }) => (
        <button
          key={key}
          type="button"
          className="relative inline-flex items-center justify-center w-9 h-9 rounded-md bg-transparent border-0 text-seeyue-fg-dim cursor-pointer transition-colors duration-150 hover:text-seeyue-fg-strong hover:bg-seeyue-elevated focus-visible:outline-2 focus-visible:outline-seeyue-accent focus-visible:outline-offset-2 data-[active=true]:text-seeyue-fg-strong before:content-[''] before:absolute before:-left-1 before:top-1.5 before:bottom-1.5 before:w-0.5 before:rounded-sm before:bg-transparent before:transition-colors before:duration-150 data-[active=true]:before:bg-seeyue-accent-strong"
          data-active={key === active ? 'true' : undefined}
          title={title}
          onClick={() => onSelect(key)}
        >
          <Icon size={20} />
        </button>
      ))}
    </nav>
  )
}
