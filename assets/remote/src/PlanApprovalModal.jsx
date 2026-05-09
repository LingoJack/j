import { useState } from 'react'

export default function PlanApprovalModal({ request, onConfirm }) {
  const [feedback, setFeedback] = useState('')

  if (!request) return null

  const handleApprove = () => onConfirm(true, undefined)
  const handleApproveAndClear = () => onConfirm(true, 'clear')
  const handleReject = () => onConfirm(false, undefined)

  return (
    <div className="fixed inset-0 z-[100] flex items-center justify-center p-4 modal-overlay bg-black/50">
      <div className="modal-content bg-bg2 rounded-lg border border-border w-full max-w-md shadow-xl" onClick={e => e.stopPropagation()}>
        {/* Header */}
        <div className="px-5 py-4 border-b border-border">
          <div className="flex items-center gap-2">
            <span className="text-accent text-lg">◈</span>
            <span className="font-bold text-fg text-[15px]">Plan 审批请求</span>
          </div>
        </div>

        {/* Body */}
        <div className="px-5 py-4 space-y-3">
          <div>
            <span className="text-[11px] text-fg3">Agent</span>
            <div className="text-fg text-[13px] font-medium mt-0.5">{request.agent_name}</div>
          </div>
          {request.plan_summary && (
            <div>
              <span className="text-[11px] text-fg3">计划摘要</span>
              <div className="text-fg2 text-[12px] mt-0.5 whitespace-pre-wrap break-all max-h-[200px] overflow-y-auto bg-bg3 rounded-lg px-3 py-2">{request.plan_summary}</div>
            </div>
          )}
        </div>

        {/* Actions */}
        <div className="px-5 py-3 border-t border-border flex gap-2">
          <button
            className="flex-1 py-2.5 rounded-lg bg-err/15 text-err text-[13px] font-medium hover:bg-err/25 transition-colors"
            onClick={handleReject}
          >
            拒绝
          </button>
          <button
            className="flex-1 py-2.5 rounded-lg bg-accent text-white text-[13px] font-medium hover:bg-accent-dim transition-colors"
            onClick={handleApprove}
          >
            批准
          </button>
          <button
            className="py-2.5 px-3 rounded-lg bg-warn/15 text-warn text-[12px] font-medium hover:bg-warn/25 transition-colors"
            onClick={handleApproveAndClear}
            title="批准并清空上下文"
          >
            批准+C
          </button>
        </div>
      </div>
    </div>
  )
}
