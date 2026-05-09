export default function AgentPermModal({ request, onConfirm }) {
  if (!request) return null

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 modal-overlay bg-black/50" onClick={() => onConfirm(false)}>
      <div className="modal-content bg-bg2 rounded-lg border border-border w-full max-w-md shadow-xl" onClick={e => e.stopPropagation()}>
        {/* Header */}
        <div className="px-5 py-4 border-b border-border">
          <div className="flex items-center gap-2">
            <span className="text-warn text-lg">◉</span>
            <span className="font-bold text-fg text-[15px]">Agent 权限请求</span>
          </div>
        </div>

        {/* Body */}
        <div className="px-5 py-4 space-y-3">
          <div>
            <span className="text-[11px] text-fg3">Agent</span>
            <div className="text-fg text-[13px] font-medium mt-0.5">{request.agent_name}</div>
          </div>
          <div>
            <span className="text-[11px] text-fg3">工具</span>
            <div className="text-fg text-[13px] font-medium mt-0.5">{request.tool_name}</div>
          </div>
          {request.arguments && (
            <div>
              <span className="text-[11px] text-fg3">详情</span>
              <div className="text-fg2 text-[12px] mt-0.5 whitespace-pre-wrap break-all max-h-[120px] overflow-y-auto bg-bg3 rounded-lg px-3 py-2">{request.arguments}</div>
            </div>
          )}
        </div>

        {/* Actions */}
        <div className="px-5 py-3 border-t border-border flex gap-2">
          <button
            className="flex-1 py-2.5 rounded-lg bg-bg3 text-fg text-[13px] font-medium hover:bg-border transition-colors"
            onClick={() => onConfirm(false)}
          >
            拒绝
          </button>
          <button
            className="flex-1 py-2.5 rounded-lg bg-accent text-white text-[13px] font-medium hover:bg-accent-dim transition-colors"
            onClick={() => onConfirm(true)}
          >
            允许
          </button>
        </div>
      </div>
    </div>
  )
}
