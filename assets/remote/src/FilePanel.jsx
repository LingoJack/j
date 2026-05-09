import { useState, useCallback, useEffect } from 'react'

export default function FilePanel({ fileEntries, fileContent, fileWriteResult, send, onBack }) {
  const [currentPath, setCurrentPath] = useState('.')
  const [editing, setEditing] = useState(false)
  const [editContent, setEditContent] = useState('')
  const [saving, setSaving] = useState(false)
  const [expandedDirs, setExpandedDirs] = useState(new Set())
  const [dirCache, setDirCache] = useState({ '.': [] }) // path → entries cache

  // 保存完成后重置 saving 状态
  useEffect(() => {
    if (fileWriteResult) setSaving(false)
  }, [fileWriteResult])

  // 更新目录缓存
  useEffect(() => {
    if (fileEntries.length > 0) {
      setDirCache(prev => ({ ...prev, [currentPath]: fileEntries }))
    }
  }, [fileEntries, currentPath])

  const handleFileList = useCallback((path) => {
    setCurrentPath(path)
    send({ type: 'file_list', path })
  }, [send])

  const handleFileRead = useCallback((path) => {
    setEditing(false)
    send({ type: 'file_read', path })
  }, [send])

  const handleFileWrite = useCallback(() => {
    if (!fileContent || saving) return
    setSaving(true)
    send({ type: 'file_write', path: fileContent.path, content: editContent })
  }, [send, fileContent, editContent, saving])

  const startEditing = useCallback(() => {
    if (fileContent) {
      setEditContent(fileContent.content)
      setEditing(true)
    }
  }, [fileContent])

  const toggleDir = useCallback((dirPath) => {
    setExpandedDirs(prev => {
      const next = new Set(prev)
      if (next.has(dirPath)) {
        next.delete(dirPath)
      } else {
        next.add(dirPath)
        // 如果没有缓存，请求加载
        if (!dirCache[dirPath]) {
          send({ type: 'file_list', path: dirPath })
        }
      }
      return next
    })
  }, [dirCache, send])

  const goUp = useCallback(() => {
    if (currentPath === '.' || !currentPath) return
    const parts = currentPath.split('/')
    parts.pop()
    handleFileList(parts.length > 0 ? parts.join('/') : '.')
  }, [currentPath, handleFileList])

  // 构建目录树条目（递归展开）
  const renderTree = (entries, parentPath, depth = 0) => {
    if (!entries || entries.length === 0) return null
    // 排序：文件夹在前，文件在后
    const sorted = [...entries].sort((a, b) => {
      if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1
      return a.name.localeCompare(b.name)
    })
    return sorted.map((entry) => {
      const fullPath = parentPath === '.' ? entry.name : `${parentPath}/${entry.name}`
      const isExpanded = expandedDirs.has(fullPath)
      const childEntries = dirCache[fullPath]
      return (
        <div key={fullPath}>
          <div
            className={`flex items-center gap-1.5 px-2 py-[3px] cursor-pointer hover:bg-[var(--color-bg3)] active:bg-[var(--color-border)] transition-colors duration-75 select-none ${fileContent?.path === fullPath ? 'bg-accent/10 text-accent' : 'text-fg2'}`}
            style={{ paddingLeft: `${depth * 16 + 8}px` }}
            onClick={() => entry.is_dir ? toggleDir(fullPath) : handleFileRead(fullPath)}
          >
            {entry.is_dir ? (
              <svg className={`w-3.5 h-3.5 shrink-0 transition-transform duration-100 ${isExpanded ? 'rotate-90' : ''}`} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                <path strokeLinecap="round" strokeLinejoin="round" d="M9 5l7 7-7 7" />
              </svg>
            ) : (
              <span className="w-3.5 shrink-0" />
            )}
            {entry.is_dir ? (
              <svg className={`w-4 h-4 shrink-0 ${isExpanded ? 'text-[#e0a040]' : 'text-[#dcb67a]'}`} fill="currentColor" viewBox="0 0 20 20">
                <path d="M2 6a2 2 0 012-2h5l2 2h5a2 2 0 012 2v6a2 2 0 01-2 2H4a2 2 0 01-2-2V6z" />
              </svg>
            ) : (
              <svg className="w-4 h-4 shrink-0 text-[#6a9a6a]" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}>
                <path strokeLinecap="round" strokeLinejoin="round" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
              </svg>
            )}
            <span className="text-[12px] truncate">{entry.name}</span>
          </div>
          {entry.is_dir && isExpanded && childEntries && renderTree(childEntries, fullPath, depth + 1)}
          {entry.is_dir && isExpanded && !childEntries && (
            <div className="px-2 py-1 text-[11px] text-fg3" style={{ paddingLeft: `${(depth + 1) * 16 + 8}px` }}>加载中...</div>
          )}
        </div>
      )
    })
  }

  return (
    <div className="flex h-full">
      {/* 左侧：目录树 */}
      <div className="w-[240px] min-w-[180px] shrink-0 flex flex-col border-r border-border bg-bg2">
        {/* 目录树顶栏 */}
        <div className="flex items-center gap-2 px-3 py-2 border-b border-border shrink-0">
          <span className="text-[11px] font-semibold text-fg3 uppercase tracking-wider flex-1">Explorer</span>
          <button
            className="text-fg3 hover:text-fg active:text-accent active:scale-[0.9] text-[12px] p-1 rounded hover:bg-bg3 transition-all duration-100 select-none"
            onClick={() => handleFileList('.')}
            title="刷新"
          >
            <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
            </svg>
          </button>
          <button
            className="text-fg3 hover:text-fg active:text-accent active:scale-[0.9] text-[12px] p-1 rounded hover:bg-bg3 transition-all duration-100 select-none"
            onClick={goUp}
            title="上级目录"
          >
            <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M5 15l7-7 7 7" />
            </svg>
          </button>
          <button
            className="text-fg3 hover:text-fg active:text-accent active:scale-[0.9] text-[12px] p-1 rounded hover:bg-bg3 transition-all duration-100 select-none"
            onClick={onBack}
            title="回到聊天"
          >
            <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        {/* 目录树 */}
        <div className="flex-1 overflow-y-auto">
          {renderTree(dirCache['.'] || fileEntries, '.', 0)}
          {(!dirCache['.'] && fileEntries.length === 0) && (
            <div className="px-4 py-6 text-center text-fg3 text-[12px]">
              <button
                className="text-accent hover:underline active:scale-[0.95] transition-transform duration-100 select-none"
                onClick={() => handleFileList('.')}
              >加载目录</button>
            </div>
          )}
        </div>
      </div>

      {/* 右侧：文件内容 */}
      <div className="flex-1 flex flex-col min-w-0">
        {/* 文件标签栏 */}
        <div className="flex items-center bg-bg2 border-b border-border shrink-0 h-[34px]">
          {fileContent ? (
            <div className="flex items-center gap-2 px-3 h-full bg-bg text-fg text-[12px] border-r border-border">
              <svg className="w-3.5 h-3.5 text-[#6a9a6a] shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}>
                <path strokeLinecap="round" strokeLinejoin="round" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
              </svg>
              <span className="truncate">{fileContent.path}</span>
              {editing && <span className="text-[10px] text-accent">● 编辑中</span>}
              <button
                className="ml-2 text-fg3 hover:text-fg active:text-err active:scale-[0.85] transition-all duration-100 select-none"
                onClick={() => { setEditing(false); setEditContent('') }}
              >×</button>
            </div>
          ) : (
            <div className="px-3 text-fg3 text-[12px]">选择文件查看内容</div>
          )}
          <div className="ml-auto flex items-center gap-1 px-2">
            {fileContent && !editing && (
              <button
                className="text-accent text-[11px] px-2 py-0.5 rounded hover:bg-accent/10 active:bg-accent/20 active:scale-[0.95] transition-all duration-100 select-none"
                onClick={startEditing}
              >编辑</button>
            )}
            {editing && (
              <>
                <button
                  className={`text-ok text-[11px] px-2 py-0.5 rounded transition-all duration-100 select-none ${saving ? 'opacity-50 pointer-events-none' : 'hover:bg-ok/10 active:bg-ok/20 active:scale-[0.95]'}`}
                  onClick={handleFileWrite}
                  disabled={saving}
                >{saving ? '保存中...' : '保存'}</button>
                <button
                  className="text-fg3 text-[11px] px-2 py-0.5 rounded hover:bg-bg3 active:bg-border active:scale-[0.95] transition-all duration-100 select-none"
                  onClick={() => setEditing(false)}
                >取消</button>
              </>
            )}
          </div>
        </div>

        {/* 文件内容 */}
        <div className="flex-1 overflow-auto">
          {fileContent ? (
            <>
              {fileContent.error && (
                <div className="px-4 py-2 text-[12px] text-err bg-err/10">{fileContent.error}</div>
              )}
              {fileWriteResult && (
                <div className={`px-4 py-2 text-[12px] ${fileWriteResult.success ? 'text-ok bg-ok/10' : 'text-err bg-err/10'}`}>
                  {fileWriteResult.success ? '✓ 保存成功' : `✗ ${fileWriteResult.error || '保存失败'}`}
                </div>
              )}
              {editing ? (
                <textarea
                  className="w-full h-full bg-bg text-fg text-[13px] font-mono p-4 resize-none outline-none leading-relaxed"
                  value={editContent}
                  onChange={e => setEditContent(e.target.value)}
                  spellCheck={false}
                />
              ) : (
                <pre className="p-4 text-[13px] font-mono text-fg2 whitespace-pre-wrap break-all leading-relaxed">{fileContent.content || '(空文件)'}</pre>
              )}
            </>
          ) : (
            <div className="flex flex-col items-center justify-center h-full text-fg3">
              <svg className="w-16 h-16 mb-4 text-fg3/30" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1}>
                <path strokeLinecap="round" strokeLinejoin="round" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
              </svg>
              <span className="text-[13px]">从左侧选择文件查看</span>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
