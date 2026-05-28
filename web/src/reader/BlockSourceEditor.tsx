import { useEffect, useRef } from 'react'

interface Props {
  /** 当前 block 对应的源码切片（含原换行） */
  sourceSlice: string
  /** 用户编辑后回调：text 是新切片完整内容（不含跨 block） */
  onChangeSlice: (text: string) => void
  /** 离开（点击外部 / Esc / blur）时回调 */
  onLeave: () => void
  onSave: () => void | Promise<void>
}

/**
 * 单个 block 的源码态 textarea。
 *
 * - 自动聚焦
 * - 行数随内容自适应
 * - blur / Esc 触发 onLeave；Cmd+S 触发 onSave
 */
export function BlockSourceEditor({
  sourceSlice,
  onChangeSlice,
  onLeave,
  onSave,
}: Props) {
  const taRef = useRef<HTMLTextAreaElement | null>(null)

  // 进入编辑态时自动聚焦
  useEffect(() => {
    if (taRef.current) {
      taRef.current.focus()
      // 光标置于末尾
      const len = taRef.current.value.length
      taRef.current.setSelectionRange(len, len)
    }
  }, [])

  // rows 自适应（按换行数 + 1）
  const rows = Math.max(1, sourceSlice.split('\n').length)

  return (
    <textarea
      ref={taRef}
      className="seeyue-textarea w-full block my-2 px-2 py-1 rounded border border-seeyue-border bg-seeyue-panel"
      rows={rows}
      spellCheck={false}
      value={sourceSlice}
      onChange={(e) => onChangeSlice(e.target.value)}
      onBlur={onLeave}
      onKeyDown={(e) => {
        if (e.key === 'Escape') {
          e.preventDefault()
          onLeave()
        } else if (
          (e.metaKey || e.ctrlKey) &&
          e.key.toLowerCase() === 's'
        ) {
          e.preventDefault()
          void onSave()
        } else if (e.key === 'Tab') {
          // 让 Tab 插入两个空格
          e.preventDefault()
          const el = e.currentTarget
          const start = el.selectionStart
          const end = el.selectionEnd
          const next =
            sourceSlice.substring(0, start) +
            '  ' +
            sourceSlice.substring(end)
          onChangeSlice(next)
          queueMicrotask(() => {
            if (taRef.current) {
              taRef.current.selectionStart = taRef.current.selectionEnd =
                start + 2
            }
          })
        }
      }}
    />
  )
}
