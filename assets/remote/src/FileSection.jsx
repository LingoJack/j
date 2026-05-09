import { useState, useCallback } from 'react'

export default function FileSection({ fileEntries, fileContent, fileWriteResult, send, onCollapse }) {
  const [currentPath, setCurrentPath] = useState('.')
  const [editing, setEditing] = useState(false)
  const [editContent, setEditContent] = useState('')

  const handleFileList = useCallback((path) => {
    setCurrentPath(path)
    send({ type: 'file_list', path })
  }, [send])

  const handleFileRead = useCallback((path) => {
    setEditing(false)
    send({ type: 'file_read', path })
  }, [send])

  const handleFileWrite = useCallback(() => {
    if (!fileContent) return
    send({ type: 'file_write', path: fileContent.path, content: editContent })
  }, [send, fileContent, editContent])

  const handleEntryClick = useCallback((entry) => {
    const newPath = currentPath === '.' ? entry.name : `${currentPath}/${entry.name}`
    if (entry.is_dir) {
      handleFileList(newPath)
    } else {
      handleFileRead(newPath)
    }
  }, [currentPath, handleFileList, handleFileRead])

  const goUp = useCallback(() => {
    if (currentPath === '.' || !currentPath) return
    const parts = currentPath.split('/')
    parts.pop()
    handleFileList(parts.length > 0 ? parts.join('/') : '.')
  }, [currentPath, handleFileList])

  const startEditing = useCallback(() => {
    if (fileContent) {
      setEditContent(fileContent.content)
      setEditing(true)
    }
  }, [fileContent])

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="sidebar-section-header">
        <span className="font-semibold text-[13px]">文件</span>
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

      {/* Path bar */}
      <div className="px-3 py-2 border-b border-border shrink-0">
        <div className="flex items-center gap-1">
          <button
            className="text-fg3 hover:text-fg text-[12px] px-1.5 py-1 rounded hover:bg-bg3 transition-colors"
            onClick={goUp}
            disabled={currentPath === '.'}
            title="上级目录"
          >↑</button>
          <button
            className="text-fg3 hover:text-fg text-[12px] px-1.5 py-1 rounded hover:bg-bg3 transition-colors"
            onClick={() => handleFileList('.')}
            title="当前目录"
          >🏠</button>
          <input
            className="flex-1 bg-bg border border-border rounded px-2 py-1 text-[11px] text-fg outline-none focus:border-accent"
            value={currentPath}
            onChange={e => setCurrentPath(e.target.value)}
            onKeyDown={e => e.key === 'Enter' && handleFileList(currentPath)}
          />
          <button
            className="text-accent text-[11px] px-2 py-1 rounded hover:bg-accent/10 transition-colors"
            onClick={() => handleFileList(currentPath)}
          >刷新</button>
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {fileContent ? (
          /* File viewer/editor */
          <div className="flex flex-col h-full">
            <div className="flex items-center gap-2 px-3 py-1.5 border-b border-border shrink-0">
              <button
                className="text-fg3 hover:text-fg text-[11px] px-1.5 py-0.5 rounded hover:bg-bg3 transition-colors"
                onClick={() => send({ type: 'file_list', path: currentPath })}
              >← 返回</button>
              <span className="text-[11px] text-fg3 truncate flex-1">{fileContent.path}</span>
              {!editing ? (
                <button
                  className="text-accent text-[11px] px-2 py-0.5 rounded hover:bg-accent/10 transition-colors"
                  onClick={startEditing}
                >编辑</button>
              ) : (
                <div className="flex gap-1">
                  <button
                    className="text-ok text-[11px] px-2 py-0.5 rounded hover:bg-ok/10 transition-colors"
                    onClick={handleFileWrite}
                  >保存</button>
                  <button
                    className="text-fg3 text-[11px] px-2 py-0.5 rounded hover:bg-bg3 transition-colors"
                    onClick={() => setEditing(false)}
                  >取消</button>
                </div>
              )}
            </div>
            {fileContent.error && (
              <div className="px-3 py-1 text-[11px] text-err bg-err/10">{fileContent.error}</div>
            )}
            {fileWriteResult && (
              <div className={`px-3 py-1 text-[11px] ${fileWriteResult.success ? 'text-ok bg-ok/10' : 'text-err bg-err/10'}`}>
                {fileWriteResult.success ? '保存成功' : (fileWriteResult.error || '保存失败')}
              </div>
            )}
            {editing ? (
              <textarea
                className="flex-1 bg-bg text-fg text-[12px] font-mono p-3 resize-none outline-none min-h-[200px]"
                value={editContent}
                onChange={e => setEditContent(e.target.value)}
                spellCheck={false}
              />
            ) : (
              <pre className="flex-1 overflow-auto p-3 text-[12px] font-mono text-fg2 whitespace-pre-wrap break-all">
                {fileContent.content || '(空文件)'}
              </pre>
            )}
          </div>
        ) : (
          /* Directory listing */
          <div>
            {fileEntries.length === 0 && (
              <div className="text-center text-fg3 text-[12px] py-4">
                <button
                  className="text-accent hover:underline"
                  onClick={() => handleFileList('.')}
                >点击加载当前目录</button>
              </div>
            )}
            {fileEntries.map((entry, i) => (
              <div
                key={i}
                className="flex items-center gap-2 px-3 py-1.5 hover:bg-bg3 cursor-pointer transition-colors border-b border-border/30"
                onClick={() => handleEntryClick(entry)}
              >
                <span className="text-[14px] shrink-0">{entry.is_dir ? '📁' : '📄'}</span>
                <span className={`text-[12px] truncate flex-1 ${entry.is_dir ? 'text-fg font-medium' : 'text-fg2'}`}>{entry.name}</span>
                {!entry.is_dir && (
                  <span className="text-[10px] text-fg3 shrink-0">
                    {entry.size > 1024 * 1024
                      ? `${(entry.size / 1024 / 1024).toFixed(1)}M`
                      : entry.size > 1024
                        ? `${(entry.size / 1024).toFixed(1)}K`
                        : `${entry.size}B`}
                  </span>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}
