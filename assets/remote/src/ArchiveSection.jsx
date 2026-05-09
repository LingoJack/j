import { useState } from 'react'
import { formatRelativeTime } from './utils'

export default function ArchiveSection({ archives, send, onCollapse }) {
  const [archiveName, setArchiveName] = useState('')
  const [showNameInput, setShowNameInput] = useState(false)

  const handleArchiveDefault = () => {
    send({ type: 'archive_with_default' })
  }

  const handleArchiveCustom = () => {
    if (archiveName.trim()) {
      send({ type: 'archive_with_custom', name: archiveName.trim() })
      setArchiveName('')
      setShowNameInput(false)
    }
  }

  const handleClear = () => {
    send({ type: 'clear_session' })
  }

  return (
    <div className="flex flex-col h-full">
      {/* Section header */}
      <div className="sidebar-section-header">
        <span className="font-semibold text-[13px]">归档</span>
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

      <div className="flex-1 overflow-y-auto">
        {/* Archive current session */}
        <div className="px-3 py-3 border-b border-border">
          <div className="text-[11px] text-fg3 mb-2">归档当前会话</div>
          <div className="flex gap-1.5 mb-2">
            <button
              className="flex-1 px-2 py-2 rounded-lg bg-accent/15 text-accent text-[12px] font-medium hover:bg-accent/25 transition-colors"
              onClick={handleArchiveDefault}
            >
              默认归档
            </button>
            <button
              className="flex-1 px-2 py-2 rounded-lg bg-bg3 text-fg text-[12px] font-medium hover:bg-border transition-colors"
              onClick={() => setShowNameInput(v => !v)}
            >
              自定义名称
            </button>
          </div>
          {showNameInput && (
            <div className="flex gap-1.5">
              <input
                className="flex-1 bg-bg border border-border rounded-lg px-2.5 py-1.5 text-[12px] text-fg outline-none focus:border-accent"
                placeholder="输入归档名称..."
                value={archiveName}
                onChange={e => setArchiveName(e.target.value)}
                onKeyDown={e => e.key === 'Enter' && handleArchiveCustom()}
                autoFocus
              />
              <button
                className="px-3 py-1.5 rounded-lg bg-accent text-white text-[12px] font-medium"
                onClick={handleArchiveCustom}
              >
                确定
              </button>
            </div>
          )}
          <button
            className="w-full mt-2 px-2 py-1.5 rounded-lg bg-err/10 text-err text-[11px] hover:bg-err/20 transition-colors"
            onClick={handleClear}
          >
            不归档，清空会话
          </button>
        </div>

        {/* Archive list */}
        <div className="px-3 py-2">
          <div className="flex items-center justify-between mb-2">
            <span className="text-[11px] text-fg3">已归档的会话</span>
            <button
              className="text-accent text-[11px] hover:underline"
              onClick={() => send({ type: 'start_archive_list' })}
            >
              刷新
            </button>
          </div>
          {archives.length === 0 ? (
            <div className="text-center text-fg3 text-[12px] py-6">暂无归档</div>
          ) : (
            archives.map((a, i) => (
              <div
                key={a.name}
                className="px-3 py-2.5 mb-1.5 rounded-lg bg-bg3/50 border border-border/50"
              >
                <div className="flex items-center justify-between mb-1">
                  <span className="text-[12px] font-medium text-fg truncate mr-2">{a.name}</span>
                  <span className="text-[10px] text-fg3 shrink-0">{a.message_count} 条</span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-[10px] text-fg3">{formatRelativeTime(a.created_at)}</span>
                  <div className="flex gap-1">
                    <button
                      className="px-2 py-0.5 rounded text-[10px] bg-accent/15 text-accent hover:bg-accent/25 transition-colors"
                      onClick={() => send({ type: 'restore_archive', index: i })}
                    >
                      恢复
                    </button>
                    <button
                      className="px-2 py-0.5 rounded text-[10px] bg-err/10 text-err hover:bg-err/20 transition-colors"
                      onClick={() => send({ type: 'delete_archive', index: i })}
                    >
                      删除
                    </button>
                  </div>
                </div>
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  )
}
