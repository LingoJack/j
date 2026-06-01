/**
 * 工具箱面板：当前侧栏的"工具"视图。
 *
 * 与 FileTree 平级，从 ActivityBar 的"工具箱"按钮切入。
 * 点击一项就把对应工具作为一个特殊 tab 打开（kind = 'tool'）。
 *
 * 后续要再加工具，往 TOOLS 数组加项 + Reader 里给 toolId 加分发即可。
 */
import { Braces, GitCompare, Toolbox as ToolboxIcon } from './Icon'
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
  /** icon 颜色 css var，让每个工具有自己的小色标 */
  tone: 'accent' | 'success' | 'warn' | 'purple'
  Icon: typeof GitCompare
}

const TOOLS: ToolDef[] = [
  {
    id: 'diff',
    name: '文本 Diff',
    description: '并排比较两段文本，行级差异高亮',
    tone: 'accent',
    Icon: GitCompare,
  },
  {
    id: 'json',
    name: 'JSON 查看器',
    description: '格式化展示 JSON，支持节点折叠 / 修改',
    tone: 'success',
    Icon: Braces,
  },
]

export function Toolbox({ activeToolId, onOpen }: Props) {
  return (
    <div className="h-full flex flex-col text-[13px] text-seeyue-fg seeyue-sidebar-shell">
      {/* —— 顶部标题栏 —— */}
      <div className="flex items-center gap-2 px-3 pt-3 pb-2 border-b border-seeyue-border">
        <button className="seeyue-tab" data-active="true">
          <ToolboxIcon size={14} />
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
            <span className="seeyue-toolbox-icon" data-tone={tool.tone}>
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
