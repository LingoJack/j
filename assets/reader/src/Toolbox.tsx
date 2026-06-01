/**
 * 工具箱面板：当前侧栏的"工具"视图。
 *
 * 与 FileTree 平级，从 ActivityBar 的"工具箱"按钮切入。
 * 点击一项就把对应工具作为一个特殊 tab 打开（kind = 'tool'）。
 *
 * 现阶段只有一个工具：文本 diff。后面要再加，往 TOOLS 数组加项 +
 * Reader 里给 toolId 加分发即可。
 */
import { GitCompare, Wrench } from './Icon'
import type { ToolId } from './types'

interface Props {
  /** 当前激活的工具 tab id（如果有），用来高亮 */
  activeToolId: ToolId | null
  onOpen: (toolId: ToolId) => void
}

interface ToolDef {
  id: ToolId
  name: string
  description: string
  Icon: typeof GitCompare
}

const TOOLS: ToolDef[] = [
  {
    id: 'diff',
    name: '文本 Diff',
    description: '并排比较两段文本，行级差异高亮',
    Icon: GitCompare,
  },
]

export function Toolbox({ activeToolId, onOpen }: Props) {
  return (
    <div className="h-full flex flex-col text-[13px] text-seeyue-fg seeyue-sidebar-shell">
      {/* —— 顶部标题栏 —— */}
      <div className="flex items-center gap-2 px-3 pt-3 pb-2 border-b border-seeyue-border">
        <button className="seeyue-tab" data-active="true">
          <Wrench size={14} />
          <span>工具箱</span>
        </button>
        <div className="flex-1" />
      </div>

      {/* —— 工具列表 —— */}
      <div className="flex-1 overflow-y-auto px-2 py-2">
        {TOOLS.map((tool) => (
          <button
            key={tool.id}
            type="button"
            className="seeyue-toolbox-row"
            data-active={tool.id === activeToolId ? 'true' : undefined}
            onClick={() => onOpen(tool.id)}
            title={tool.description}
          >
            <span className="seeyue-toolbox-icon">
              <tool.Icon size={16} />
            </span>
            <span className="seeyue-toolbox-text">
              <span className="name">{tool.name}</span>
              <span className="desc">{tool.description}</span>
            </span>
          </button>
        ))}
      </div>
    </div>
  )
}
