/**
 * 工具箱面板：当前侧栏的"工具"视图。
 *
 * 与 FileTree 平级，从 ActivityBar 的"工具箱"按钮切入。
 * 点击一项就把对应工具作为一个特殊 tab 打开（kind = 'tool'）。
 *
 * 后续要再加工具，往 TOOLS 数组加项 + Reader 里给 toolId 加分发即可。
 */
import { Braces, GitCompare } from './Icon'
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
    <div className="h-full flex flex-col text-[13px] text-seeyue-fg bg-seeyue-sidebar shadow-[0_0_12px_rgba(0,0,0,0.3)]">
      {/* —— 工具列表 —— */}
      <div className="flex-1 overflow-y-auto px-2 py-2.5">
        {TOOLS.map((tool) => (
          <button
            key={tool.id}
            type="button"
            className="group flex items-center gap-2.5 w-full px-2.5 py-2 bg-transparent border-0 rounded-md text-left cursor-pointer text-seeyue-fg transition-colors duration-150 hover:bg-seeyue-elevated data-[active=true]:bg-seeyue-accent-soft data-[active=true]:text-seeyue-fg-strong"
            data-active={tool.id === activeToolId ? 'true' : undefined}
            onClick={() => onOpen(tool.id)}
            title={tool.description}
          >
            <span
              className="shrink-0 inline-flex items-center justify-center w-7 h-7 rounded-[7px] text-seeyue-fg-muted bg-seeyue-bg-deep shadow-[inset_0_0_0_1px_var(--color-seeyue-border)] transition-all duration-150 data-[tone=accent]:text-seeyue-accent data-[tone=accent]:bg-seeyue-accent-mute data-[tone=accent]:shadow-[inset_0_0_0_1px_rgba(136,192,208,0.3)] data-[tone=success]:text-seeyue-success data-[tone=success]:bg-[rgba(163,190,140,0.12)] data-[tone=success]:shadow-[inset_0_0_0_1px_rgba(163,190,140,0.3)] data-[tone=warn]:text-seeyue-warn data-[tone=warn]:bg-[rgba(208,135,112,0.14)] data-[tone=warn]:shadow-[inset_0_0_0_1px_rgba(208,135,112,0.3)] data-[tone=purple]:text-seeyue-purple data-[tone=purple]:bg-[rgba(180,142,173,0.14)] data-[tone=purple]:shadow-[inset_0_0_0_1px_rgba(180,142,173,0.3)] group-hover:brightness-110 group-data-[active=true]:brightness-[1.2]"
              data-tone={tool.tone}
            >
              <tool.Icon size={16} />
            </span>
            <span className="flex flex-col gap-0.5 min-w-0">
              <span className="text-[13px] font-medium text-seeyue-fg-strong">{tool.name}</span>
              <span className="text-[11.5px] text-seeyue-fg-dim leading-normal whitespace-normal">
                {tool.description}
              </span>
            </span>
          </button>
        ))}
      </div>
    </div>
  )
}
