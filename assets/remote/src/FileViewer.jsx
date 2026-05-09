import { useState, useCallback, useEffect } from 'react'

export default function FileViewer({ fileContent, fileWriteResult, send, onBack }) {
  const [editing, setEditing] = useState(false)
  const [editContent, setEditContent] = useState('')
  const [saving, setSaving] = useState(false)
  const [dirty, setDirty] = useState(false)

  useEffect(() => {
    if (fileWriteResult) setSaving(false)
  }, [fileWriteResult])

  const handleFileWrite = useCallback(() => {
    if (!fileContent || saving) return
    setSaving(true)
    setDirty(false)
    send({ type: 'file_write', path: fileContent.path, content: editContent })
  }, [send, fileContent, editContent, saving])

  const startEditing = useCallback(() => {
    if (fileContent) {
      setEditContent(fileContent.content)
      setEditing(true)
      setDirty(false)
    }
  }, [fileContent])

  const handleBack = useCallback(() => {
    if (dirty && !confirm('有未保存的更改，确定返回？')) return
    onBack()
  }, [dirty, onBack])

  const handleCancel = useCallback(() => {
    setEditing(false)
    setDirty(false)
  }, [])

  return (
    <div className="flex flex-col h-full">
      {/* 顶栏 */}
      <div className="flex items-center gap-2 px-3 bg-bg2 border-b border-border shrink-0 h-[34px]">
        <button
          className="text-fg3 hover:text-fg active:text-accent active:scale-[0.92] text-[11px] px-1.5 py-0.5 rounded hover:bg-bg3 active:bg-bg3 transition-all duration-100 select-none"
          onClick={handleBack}
        >← 返回</button>
        <div className="w-px h-4 bg-border" />
        <svg className="w-3.5 h-3.5 text-[#6a9a6a] shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}>
          <path strokeLinecap="round" strokeLinejoin="round" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
        </svg>
        <span className="text-fg text-[12px] truncate flex-1">{fileContent?.path}</span>
        {editing && <span className={`text-[10px] shrink-0 ${dirty ? 'text-warn' : 'text-fg3'}`}>{dirty ? '● 未保存' : '● 编辑中'}</span>}
        <div className="flex items-center gap-1 shrink-0">
          {!editing ? (
            <button
              className="text-accent text-[11px] px-2 py-0.5 rounded hover:bg-accent/10 active:bg-accent/20 active:scale-[0.95] transition-all duration-100 select-none"
              onClick={startEditing}
            >编辑</button>
          ) : (
            <>
              <button
                className={`text-ok text-[11px] px-2 py-0.5 rounded transition-all duration-100 select-none ${saving ? 'opacity-50 pointer-events-none' : 'hover:bg-ok/10 active:bg-ok/20 active:scale-[0.95]'}`}
                onClick={handleFileWrite}
                disabled={saving}
              >{saving ? '保存中...' : '保存'}</button>
              <button
                className="text-fg3 text-[11px] px-2 py-0.5 rounded hover:bg-bg3 active:bg-border active:scale-[0.95] transition-all duration-100 select-none"
                onClick={handleCancel}
              >取消</button>
            </>
          )}
        </div>
      </div>

      {/* 文件内容 */}
      <div className="flex-1 overflow-auto">
        {fileContent?.error && (
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
            onChange={e => { setEditContent(e.target.value); setDirty(true) }}
            spellCheck={false}
          />
        ) : (
          <pre className="p-4 text-[13px] font-mono text-fg2 whitespace-pre break-all leading-relaxed">{fileContent?.content || '(空文件)'}</pre>
        )}
      </div>
    </div>
  )
}
