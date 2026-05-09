export default function RemoteSection({ onCollapse }) {
  return (
    <div className="flex flex-col h-full">
      <div className="sidebar-section-header">
        <span className="font-semibold text-[13px]">远程连接管理</span>
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
      <div className="sidebar-placeholder">
        <svg className="w-8 h-8 mb-3 opacity-40" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}>
          <path strokeLinecap="round" strokeLinejoin="round" d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1" />
        </svg>
        <div className="text-sm font-medium mb-1">远程连接管理</div>
        <div className="text-xs">远程连接功能开发中</div>
      </div>
    </div>
  )
}
