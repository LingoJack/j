export default function FilePreviewSection({ onCollapse }) {
  return (
    <div className="flex flex-col h-full">
      <div className="sidebar-section-header">
        <span className="font-semibold text-[13px]">文件内容预览</span>
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
          <path strokeLinecap="round" strokeLinejoin="round" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
        </svg>
        <div className="text-sm font-medium mb-1">文件内容预览</div>
        <div className="text-xs">文件预览功能开发中</div>
      </div>
    </div>
  )
}
