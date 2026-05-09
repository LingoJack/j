export default function HelpSection({ onCollapse }) {
  const shortcuts = [
    { key: 'Enter', desc: '发送消息' },
    { key: 'Shift+Enter', desc: '换行' },
    { key: '■ 按钮', desc: '取消当前回复' },
    { key: '☰ 按钮', desc: '切换会话列表' },
  ]

  return (
    <div className="flex flex-col h-full">
      {/* Section header */}
      <div className="sidebar-section-header">
        <span className="font-semibold text-[13px]">帮助</span>
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

      <div className="flex-1 overflow-y-auto px-3 py-3">
        {/* 快捷键 */}
        <div className="mb-4">
          <h3 className="text-[12px] font-semibold text-fg mb-2">快捷键</h3>
          <div className="space-y-1.5">
            {shortcuts.map(s => (
              <div key={s.key} className="flex items-center justify-between text-[11px]">
                <span className="text-fg3">{s.desc}</span>
                <kbd className="bg-bg3 text-fg px-1.5 py-0.5 rounded text-[10px] font-mono border border-border">{s.key}</kbd>
              </div>
            ))}
          </div>
        </div>

        {/* 关于 */}
        <div className="mb-4">
          <h3 className="text-[12px] font-semibold text-fg mb-2">关于</h3>
          <div className="text-[11px] text-fg3 space-y-1.5">
            <p>Sprite Remote — 远程控制终端</p>
            <p>通过 WebSocket + ECDH/AES-256-GCM 加密连接到本地 j-cli</p>
          </div>
        </div>

        {/* 功能说明 */}
        <div>
          <h3 className="text-[12px] font-semibold text-fg mb-2">功能</h3>
          <div className="space-y-2 text-[11px] text-fg3">
            <div className="flex gap-2">
              <span className="text-accent shrink-0">会话</span>
              <span>切换、新建、删除会话</span>
            </div>
            <div className="flex gap-2">
              <span className="text-accent shrink-0">配置</span>
              <span>切换模型、主题，编辑全局设置，开关工具/技能</span>
            </div>
            <div className="flex gap-2">
              <span className="text-accent shrink-0">归档</span>
              <span>归档/恢复/删除会话，清空当前会话</span>
            </div>
            <div className="flex gap-2">
              <span className="text-accent shrink-0">工具</span>
              <span>允许/拒绝工具执行，回答 Ask 问题</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
