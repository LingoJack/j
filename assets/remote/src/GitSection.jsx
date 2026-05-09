export default function GitSection({ onCollapse }) {
  return (
    <div className="flex flex-col h-full">
      <div className="sidebar-section-header">
        <span className="font-semibold text-[13px]">Git</span>
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
          <path strokeLinecap="round" strokeLinejoin="round" d="M13 10V3L4 14h7v7l9-11h-7z" />
        </svg>
        <div className="text-sm font-medium mb-1">Git</div>
        <div className="text-xs">Git 功能开发中</div>
      </div>
    </div>
  )
}
