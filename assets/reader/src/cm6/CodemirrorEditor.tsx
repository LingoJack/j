/**
 * CM6 编辑器封装 —— 替代旧的 MilkdownEditor。
 *
 * 与外层 Reader 的契约不变：
 * - props.path / initialSource / onChange / onParsed / onSave / baseDir
 * - 只是 onParsed 这里**本地**调用（不再走 /api/parse），用 walk Lezer tree
 *   生成 `ParsedDocument` 给 TOC 用 —— 阶段 4 完成后这条会切换。
 *
 * 设计要点：
 * - 文档就是 markdown 源码本身。CM6 文档 ↔ Reader.sourcesRef 一一对应。
 * - 切 tab 时由 Reader 通过 `key={path}` 整体 unmount，CM6 实例销毁；状态
 *   不需要跨 tab 共享。
 * - 阶段 1 暂不接图片 widget / inline HTML widget / 代码块高亮 —— 后续阶段补。
 */
import { useEffect, useRef } from 'react'
import { EditorState } from '@codemirror/state'
import {
  EditorView,
  drawSelection,
  dropCursor,
  highlightActiveLine,
  keymap,
  rectangularSelection,
  crosshairCursor,
} from '@codemirror/view'
import {
  defaultKeymap,
  history,
  historyKeymap,
  indentWithTab,
} from '@codemirror/commands'
import { searchKeymap } from '@codemirror/search'
import {
  bracketMatching,
  indentOnInput,
  syntaxTree,
} from '@codemirror/language'
import { markdown, markdownLanguage } from '@codemirror/lang-markdown'
import { livePreview } from './livePreview'
import { codeHighlight } from './codeHighlight'
import { widgetsExtension } from './widgets'
import type {
  Block,
  BlockKind,
  Inline,
  ParsedDocument,
} from '../types'

interface Props {
  path: string
  baseDir: string | null
  initialSource: string
  onChange: (path: string, source: string) => void
  onParsed: (path: string, doc: ParsedDocument) => void
  onSave: () => void | Promise<void>
}

/**
 * 反复用到的 baseDir / onChange 闭包桥接 —— 避免 useEffect 每次依赖刷
 * 都重建整个 EditorView（成本巨大，会丢光标 / 历史栈 / scroll 位置）。
 */
function useLatest<T>(value: T) {
  const ref = useRef(value)
  ref.current = value
  return ref
}

export function CodemirrorEditor({
  path,
  baseDir,
  initialSource,
  onChange,
  onParsed,
  onSave,
}: Props) {
  const hostRef = useRef<HTMLDivElement | null>(null)
  const viewRef = useRef<EditorView | null>(null)

  const onChangeRef = useLatest(onChange)
  const onParsedRef = useLatest(onParsed)
  const onSaveRef = useLatest(onSave)
  const pathRef = useLatest(path)
  // baseDir 阶段 1 还没用到，先桥起来给阶段 4 用
  const baseDirRef = useLatest(baseDir)
  void baseDirRef

  useEffect(() => {
    if (!hostRef.current) return

    const state = EditorState.create({
      doc: initialSource,
      extensions: [
        history(),
        drawSelection(),
        dropCursor(),
        rectangularSelection(),
        crosshairCursor(),
        highlightActiveLine(),
        bracketMatching(),
        indentOnInput(),
        markdown({ base: markdownLanguage }),
        livePreview,
        codeHighlight,
        widgetsExtension(baseDir),
        EditorView.lineWrapping,
        keymap.of([
          // ⌘S → 保存（外层 Reader 接管）
          {
            key: 'Mod-s',
            run: () => {
              void onSaveRef.current()
              return true
            },
          },
          ...defaultKeymap,
          ...historyKeymap,
          ...searchKeymap,
          indentWithTab,
        ]),
        // 文档变化 → onChange + 本地 parse → onParsed（给 TOC）
        EditorView.updateListener.of((u) => {
          if (!u.docChanged) return
          const md = u.state.doc.toString()
          onChangeRef.current(pathRef.current, md)
          // TOC 重算 —— walk Lezer tree
          const doc = parseToIr(u.view)
          onParsedRef.current(pathRef.current, doc)
        }),
      ],
    })

    const view = new EditorView({
      state,
      parent: hostRef.current,
    })
    viewRef.current = view

    // 首次 mount 也跑一次 parse —— TOC 立刻有内容
    onParsedRef.current(pathRef.current, parseToIr(view))

    return () => {
      view.destroy()
      viewRef.current = null
    }
    // 故意只依赖 path：path 变才整体 remount
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [path])

  return <div ref={hostRef} className="cm6-host h-full overflow-hidden" />
}

// ---------------------------------------------------------------------------
// 本地 markdown → ParsedDocument（仅给 TOC 用，不追求完整保真）
// ---------------------------------------------------------------------------

/**
 * walk Lezer tree 生成给 TOC 用的最小 ParsedDocument。
 *
 * 阶段 1 实现：仅 heading 块完整生成，其它块用 paragraph 占位。
 * 阶段 4 把 image / list / code_block 等也补全后，TOC 就完全脱离 /api/parse。
 */
function parseToIr(view: EditorView): ParsedDocument {
  const blocks: Block[] = []
  const tree = syntaxTree(view.state)
  const doc = view.state.doc

  tree.iterate({
    enter: (node) => {
      const name = node.name
      if (/^ATXHeading[1-6]$/.test(name)) {
        const level = Number(name.slice(-1))
        const startLine = doc.lineAt(node.from).number
        const endLine = doc.lineAt(node.to).number
        // 文本：去掉前导 # 和首个空格
        const raw = doc.sliceString(node.from, node.to)
        const text = raw.replace(/^#+\s*/, '').trimEnd()
        const inline: Inline[] = [{ type: 'text', value: text }]
        const kind: BlockKind = {
          type: 'heading',
          value: { level, content: inline },
        }
        blocks.push({
          source: { start_line: startLine, end_line: endLine },
          kind,
        })
        return false
      }
      return undefined
    },
  })

  return { blocks }
}
