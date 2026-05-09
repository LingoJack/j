import { formatRelativeTime } from './utils'

export default function SessionSection({ sessions, currentSessionId, onSwitch, onNew, onCollapse }) {
  return (
    <div className="flex flex-col h-full">
      {/* Section header */}
      <div className="sidebar-section-header">
        <span className="font-semibold text-[13px]">会话</span>
        <div className="flex items-center gap-1">
          <button
            className="text-accent hover:bg-accent/10 text-[13px] px-2 py-1 rounded-md transition-colors"
            onClick={onNew}
          >+ 新建</button>
          <button
            className="text-fg3 hover:text-fg p-1 rounded-md hover:bg-bg3 transition-colors"
            onClick={onCollapse}
            title="收起侧边栏"
          >
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M15 19l-7-7 7-7" />
            </svg>
          </button>
        </div>
      </div>

      {/* Session list */}
      <div className="flex-1 overflow-y-auto">
        {sessions.map(s => {
          const isCurrent = s.id === currentSessionId
          return (
            <div
              key={s.id}
              className={`px-3 py-2.5 border-b border-border/50 cursor-pointer transition-colors ${isCurrent ? 'bg-accent/10 border-l-2 border-l-accent' : 'hover:bg-bg3'}`}
              onClick={() => onSwitch(s.id)}
            >
              <div className="flex items-center justify-between mb-0.5">
                <span className={`text-[12px] font-medium truncate flex-1 mr-2 ${isCurrent ? 'text-accent' : 'text-fg'}`}>
                  {s.first_message_preview || '新会话'}
                </span>
                {isCurrent && <span className="text-[10px] text-accent bg-accent/15 px-1.5 py-0.5 rounded-full shrink-0">当前</span>}
              </div>
              <div className="flex items-center gap-2 text-[10px] text-fg3">
                <span>{s.message_count} 条消息</span>
                <span>·</span>
                <span>{formatRelativeTime(s.updated_at)}</span>
              </div>
            </div>
          )
        })}
        {sessions.length === 0 && (
          <div className="text-center text-fg3 text-sm py-8">暂无会话</div>
        )}
      </div>
    </div>
  )
}
