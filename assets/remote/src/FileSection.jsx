import { useState, useCallback, useEffect } from 'react'

export default function FileSection({ fileEntries, fileContent, fileWriteResult, send, onOpenFile, onCollapse }) {
  const [currentPath, setCurrentPath] = useState('.')
  const [expandedDirs, setExpandedDirs] = useState(new Set(['.']))
  const [dirCache, setDirCache] = useState({})
  const [rootName, setRootName] = useState('项目')

  // 更新目录缓存
  useEffect(() => {
    if (fileEntries.length > 0 || currentPath) {
      setDirCache(prev => ({ ...prev, [currentPath]: fileEntries }))
    }
  }, [fileEntries, currentPath])

  // 首次加载
  useEffect(() => {
    if (fileEntries.length === 0) {
      send({ type: 'file_list', path: '.' })
    }
  }, [])

  // 从 fileEntries 推断根目录名
  useEffect(() => {
    if (fileEntries.length > 0 && rootName === '项目') {
      // 尝试从路径获取项目名
      // 默认用 cwd 的 basename
      setRootName('j')
    }
  }, [fileEntries])

  const handleFileList = useCallback((path) => {
    setCurrentPath(path)
    send({ type: 'file_list', path })
  }, [send])

  const handleFileRead = useCallback((path) => {
    send({ type: 'file_read', path })
    if (onOpenFile) onOpenFile(path)
  }, [send, onOpenFile])

  const toggleDir = useCallback((dirPath) => {
    setExpandedDirs(prev => {
      const next = new Set(prev)
      if (next.has(dirPath)) {
        next.delete(dirPath)
      } else {
        next.add(dirPath)
        if (!dirCache[dirPath]) {
          send({ type: 'file_list', path: dirPath })
        }
      }
      return next
    })
  }, [dirCache, send])

  // 构建目录树（递归）
  const renderTree = (entries, parentPath, depth = 0) => {
    if (!entries || entries.length === 0) return null
    const sorted = [...entries].sort((a, b) => {
      if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1
      return a.name.localeCompare(b.name)
    })
    return sorted.map((entry) => {
      const fullPath = parentPath === '.' ? entry.name : `${parentPath}/${entry.name}`
      const isExpanded = expandedDirs.has(fullPath)
      const childEntries = dirCache[fullPath]
      const isActive = fileContent?.path === fullPath
      return (
        <div key={fullPath}>
          <div
            className={`flex items-center gap-1 py-[2px] cursor-pointer hover:bg-bg3 active:bg-border transition-colors duration-75 select-none ${isActive ? 'bg-accent/10 text-accent' : 'text-fg2'}`}
            style={{ paddingLeft: `${depth * 14 + 8}px`, paddingRight: '8px' }}
            onClick={() => entry.is_dir ? toggleDir(fullPath) : handleFileRead(fullPath)}
          >
            {/* 展开箭头 */}
            {entry.is_dir ? (
              <svg className={`w-3 h-3 shrink-0 transition-transform duration-100 text-fg3 ${isExpanded ? 'rotate-90' : ''}`} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2.5}>
                <path strokeLinecap="round" strokeLinejoin="round" d="M9 5l7 7-7 7" />
              </svg>
            ) : (
              <span className="w-3 shrink-0" />
            )}
            {/* 图标 */}
            {entry.is_dir ? (
              isExpanded ? (
                <svg className="w-4 h-4 shrink-0 text-[#e0a040]" fill="currentColor" viewBox="0 0 20 20">
                  <path fillRule="evenodd" d="M2 6a2 2 0 012-2h4l2 2h4a2 2 0 012 2v1H8a3 3 0 00-3 3v1.5a1.5 1.5 0 01-3 0V6z" clipRule="evenodd" />
                  <path d="M6 10a2 2 0 00-2 2v2a2 2 0 002 2h8a2 2 0 002-2v-2a2 2 0 00-2-2H6z" />
                </svg>
              ) : (
                <svg className="w-4 h-4 shrink-0 text-[#dcb67a]" fill="currentColor" viewBox="0 0 20 20">
                  <path d="M2 6a2 2 0 012-2h5l2 2h5a2 2 0 012 2v6a2 2 0 01-2 2H4a2 2 0 01-2-2V6z" />
                </svg>
              )
            ) : (
              <svg className="w-4 h-4 shrink-0 text-[#6a9a6a]" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}>
                <path strokeLinecap="round" strokeLinejoin="round" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
              </svg>
            )}
            <span className="text-[12px] truncate">{entry.name}</span>
          </div>
          {entry.is_dir && isExpanded && childEntries && renderTree(childEntries, fullPath, depth + 1)}
          {entry.is_dir && isExpanded && !childEntries && (
            <div className="py-1 text-[11px] text-fg3 animate-[pulse_1.5s_ease-in-out_infinite]" style={{ paddingLeft: `${(depth + 1) * 14 + 20}px` }}>...</div>
          )}
        </div>
      )
    })
  }

  // 根目录条目
  const rootExpanded = expandedDirs.has('.')
  const rootEntries = dirCache['.'] || fileEntries

  return (
    <div className="flex flex-col h-full">
      {/* Section header */}
      <div className="sidebar-section-header">
        <span className="font-semibold text-[13px]">文件</span>
        <div className="flex items-center gap-0.5">
          <button
            className="text-fg3 hover:text-fg active:text-accent active:scale-[0.9] p-1 rounded hover:bg-bg3 transition-all duration-100 select-none"
            onClick={() => handleFileList('.')}
            title="刷新"
          >
            <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
            </svg>
          </button>
          <button
            className="text-fg3 hover:text-fg active:text-accent active:scale-[0.9] p-1 rounded hover:bg-bg3 transition-all duration-100 select-none"
            onClick={onCollapse}
            title="收起侧边栏"
          >
            <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M15 19l-7-7 7-7" />
            </svg>
          </button>
        </div>
      </div>

      {/* 目录树 */}
      <div className="flex-1 overflow-y-auto">
        {/* 根目录（项目名） */}
        <div
          className="flex items-center gap-1 py-[3px] cursor-pointer hover:bg-bg3 active:bg-border transition-colors duration-75 select-none text-fg font-semibold"
          style={{ paddingLeft: '4px', paddingRight: '8px' }}
          onClick={() => toggleDir('.')}
        >
          <svg className={`w-3 h-3 shrink-0 transition-transform duration-100 text-fg3 ${rootExpanded ? 'rotate-90' : ''}`} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2.5}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M9 5l7 7-7 7" />
          </svg>
          <svg className="w-4 h-4 shrink-0 text-[#e0a040]" fill="currentColor" viewBox="0 0 20 20">
            <path fillRule="evenodd" d="M2 6a2 2 0 012-2h4l2 2h4a2 2 0 012 2v1H8a3 3 0 00-3 3v1.5a1.5 1.5 0 01-3 0V6z" clipRule="evenodd" />
            <path d="M6 10a2 2 0 00-2 2v2a2 2 0 002 2h8a2 2 0 002-2v-2a2 2 0 00-2-2H6z" />
          </svg>
          <span className="text-[12px]">{rootName}</span>
        </div>

        {/* 子目录/文件 */}
        {rootExpanded && renderTree(rootEntries, '.', 1)}

        {/* 空状态 */}
        {rootEntries.length === 0 && rootExpanded && (
          <div className="px-4 py-6 text-center text-fg3 text-[12px]">
            <button
              className="text-accent hover:underline active:scale-[0.95] transition-transform duration-100 select-none"
              onClick={() => handleFileList('.')}
            >点击加载目录</button>
          </div>
        )}
      </div>
    </div>
  )
}
