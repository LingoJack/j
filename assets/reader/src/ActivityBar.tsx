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
    <nav className="seeyue-activity-bar" aria-label="侧栏切换">
      {ITEMS.map(({ key, title, Icon }) => (
        <button
          key={key}
          type="button"
          className="seeyue-activity-item"
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
