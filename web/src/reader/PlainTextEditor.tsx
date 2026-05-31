import { useEffect, useRef } from 'react'

interface Props {
  /** 文件路径，外层 Reader 用 key={path} 触发整体 remount */
  path: string
  /** 仅 mount 时被读 —— 之后 textarea 自管理 */
  initialSource: string
  /** 高频回调，外层只写 ref，不会 re-render */
  onChange: (path: string, source: string) => void
}

/**
 * 极简纯文本编辑器：一个 uncontrolled `<textarea>`，无任何顶栏。
 *
 * - 顶栏（保存按钮、未保存徽章、面包屑）都已经在 EditorBar，重复不美观，去掉。
 * - 改成 uncontrolled (defaultValue) 后，输入流不再每次按键回经 React 树，
 *   彻底避免 Reader/TabBar 等的连带 re-render —— 大文件下表现明显平滑。
 */
export function PlainTextEditor({ path, initialSource, onChange }: Props) {
  const ref = useRef<HTMLTextAreaElement | null>(null)

  // 让 onChange 闭包始终拿最新引用
  const onChangeRef = useRef(onChange)
  onChangeRef.current = onChange

  useEffect(() => {
    ref.current?.focus()
  }, [])

  return (
    <textarea
      ref={ref}
      key={path}
      className="seeyue-textarea w-full h-full px-6 py-5 overflow-y-auto"
      spellCheck={false}
      defaultValue={initialSource}
      onInput={(e) => {
        const v = (e.target as HTMLTextAreaElement).value
        onChangeRef.current(path, v)
      }}
    />
  )
}
