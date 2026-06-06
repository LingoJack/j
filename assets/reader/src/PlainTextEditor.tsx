import { useEffect, useRef } from 'react'
import { detectCodeLanguage, isCodeFile } from './codeLanguage'
import { renderHighlightedCode } from './editor/code-highlight'

interface Props {
  /** 文件路径，外层 Reader 用 key={path} 触发整体 remount */
  path: string
  /** 仅 mount 时被读 —— 之后 textarea 自管理 */
  initialSource: string
  /** 高频回调，外层只写 ref，不会 re-render */
  onChange: (path: string, source: string) => void
}

/**
 * 纯文本 / 代码文件编辑器。
 *
 * 文本文件保持轻量 textarea；代码文件使用 VS Code 风的 textarea + pre overlay，
 * 后景负责 refractor 高亮，前景负责真实输入、选择和保存。
 */
export function PlainTextEditor({ path, initialSource, onChange }: Props) {
  const textAreaRef = useRef<HTMLTextAreaElement | null>(null)
  const preRef = useRef<HTMLPreElement | null>(null)
  const highlightRef = useRef<HTMLElement | null>(null)
  const codeMode = isCodeFile(path)
  const language = detectCodeLanguage(path)

  useEffect(() => {
    textAreaRef.current?.focus()
  }, [])

  useEffect(() => {
    if (!codeMode) return
    renderCodeLayer(highlightRef.current, initialSource, language)
  }, [codeMode, initialSource, language])

  if (!codeMode) {
    return (
      <textarea
        ref={textAreaRef}
        key={path}
        className="seeyue-textarea w-full h-full px-6 py-5 overflow-y-auto"
        spellCheck={false}
        defaultValue={initialSource}
        onInput={(e) => {
          const v = (e.target as HTMLTextAreaElement).value
          onChange(path, v)
        }}
      />
    )
  }

  return (
    <div className="seeyue-code-editor relative w-full h-full overflow-hidden bg-seeyue-bg">
      <pre
        ref={preRef}
        aria-hidden="true"
        className="seeyue-code-editor-highlight absolute inset-0 m-0 overflow-auto px-8 py-6 font-[family-name:var(--font-mono)] text-[13px] leading-[1.72] whitespace-pre tab-size-2 pointer-events-none"
      >
        <code ref={highlightRef} />
      </pre>
      <textarea
        ref={textAreaRef}
        key={path}
        className="seeyue-code-editor-input absolute inset-0 w-full h-full resize-none overflow-auto border-0 bg-transparent px-8 py-6 font-[family-name:var(--font-mono)] text-[13px] leading-[1.72] whitespace-pre tab-size-2 outline-none text-transparent caret-seeyue-accent selection:bg-[rgba(22,119,255,0.18)]"
        spellCheck={false}
        defaultValue={initialSource}
        onInput={(e) => {
          const v = (e.target as HTMLTextAreaElement).value
          renderCodeLayer(highlightRef.current, v, language)
          onChange(path, v)
        }}
        onScroll={(e) => {
          preRef.current?.scrollTo({
            left: e.currentTarget.scrollLeft,
            top: e.currentTarget.scrollTop,
          })
        }}
      />
    </div>
  )
}

function renderCodeLayer(target: HTMLElement | null, source: string, language: string) {
  if (!target) return
  target.replaceChildren(renderHighlightedCode(source, language))
}
