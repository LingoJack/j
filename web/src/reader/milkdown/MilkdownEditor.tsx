/**
 * Reader 的 Milkdown 编辑器封装。
 *
 * - 单一 contenteditable（ProseMirror）编辑面，光标可在段落、列表、表格、
 *   代码块之间自由移动 —— Typora 体验的本质。
 * - 监听 markdown 变化 → `onChange(source)`；外层 Reader 据此打 dirty。
 * - 仍保留 150ms debounce 调用 `/api/parse` → `onParsed(doc)`，给右栏 TOC 用。
 * - 切 tab 由 Reader.tsx 通过 `key={tab.path}` 触发整体 unmount/remount，
 *   useEditor 的 destroy hook 会在 unmount 时清掉 ProseMirror 实例。
 */

import { useEffect, useRef } from 'react'
import { Editor, defaultValueCtx, rootCtx } from '@milkdown/kit/core'
import { commonmark } from '@milkdown/kit/preset/commonmark'
import { gfm } from '@milkdown/kit/preset/gfm'
import { listener, listenerCtx } from '@milkdown/kit/plugin/listener'
import { history } from '@milkdown/kit/plugin/history'
import { cursor } from '@milkdown/kit/plugin/cursor'
import { clipboard } from '@milkdown/kit/plugin/clipboard'
import { indent } from '@milkdown/kit/plugin/indent'
import { Milkdown, MilkdownProvider, useEditor } from '@milkdown/react'
import { prism, prismConfig } from '@milkdown/plugin-prism'
import { refractor } from 'refractor'   // 经 vite alias 后实际是 refractor/core（空内核）
import bash from 'refractor/bash'
import shell from 'refractor/shell-session'
import javascript from 'refractor/javascript'
import typescript from 'refractor/typescript'
import jsx from 'refractor/jsx'
import tsx from 'refractor/tsx'
import python from 'refractor/python'
import rust from 'refractor/rust'
import go from 'refractor/go'
import cLang from 'refractor/c'
import cpp from 'refractor/cpp'
import csharp from 'refractor/csharp'
import java from 'refractor/java'
import ruby from 'refractor/ruby'
import sql from 'refractor/sql'
import json from 'refractor/json'
import yaml from 'refractor/yaml'
import toml from 'refractor/toml'
import markdownLang from 'refractor/markdown'
import html from 'refractor/markup'
import css from 'refractor/css'
import scss from 'refractor/scss'
import diff from 'refractor/diff'

import {
  seeyueHeadingIdConfig,
  seeyueHeadingIdSync,
} from './headingId'
import { seeyueImageResolver } from './imageResolver'
import {
  htmlInlineView,
  seeyueBaseDirCtx,
} from './html'
import type { ParsedDocument, Tab } from '../types'

/** 一次性把要支持的语言注册到 refractor 核心实例上（模块级、所有 editor 共享） */
let _languagesRegistered = false
function ensureLanguages() {
  if (_languagesRegistered) return
  _languagesRegistered = true
  for (const lang of [
    bash,
    shell,
    javascript,
    typescript,
    jsx,
    tsx,
    python,
    rust,
    go,
    cLang,
    cpp,
    csharp,
    java,
    ruby,
    sql,
    json,
    yaml,
    toml,
    markdownLang,
    html,
    css,
    scss,
    diff,
  ]) {
    try {
      refractor.register(lang)
    } catch {
      /* 重复注册等情况忽略 */
    }
  }
}

interface Props {
  tab: Tab
  baseDir: string | null
  onChange: (source: string) => void
  onParsed: (doc: ParsedDocument) => void
  onSave: () => void | Promise<void>
}

/** debounce 时间：source 变化后多久 POST /api/parse（仅供 TOC 使用） */
const PARSE_DEBOUNCE_MS = 150

function MilkdownInner({ tab, baseDir, onChange, onParsed }: Omit<Props, 'onSave'>) {
  ensureLanguages()

  // 用 ref 桥接：使 markdownUpdated 闭包始终拿到最新 onChange，避免 useEditor
  // 因为 deps=[] 而吃到 stale closure
  const onChangeRef = useRef(onChange)
  onChangeRef.current = onChange

  // initialSource 只在 mount 时被读，之后的 tab.source 由 ProseMirror 自己保管
  // —— 切 tab 通过外层 key={tab.path} 整体重挂载来重新读 initial
  const initialSourceRef = useRef(tab.source)
  const baseDirRef = useRef(baseDir)
  baseDirRef.current = baseDir

  useEditor((root) =>
    Editor.make()
      .config((ctx) => {
        ctx.set(rootCtx, root)
        ctx.set(defaultValueCtx, initialSourceRef.current)
        ctx.set(seeyueBaseDirCtx.key, baseDirRef.current)
        ctx.get(listenerCtx).markdownUpdated((_ctx, md, prev) => {
          if (md !== prev) onChangeRef.current(md)
        })
        seeyueHeadingIdConfig(ctx)
        ctx.update(prismConfig.key, (prev) => ({
          ...prev,
          configureRefractor: () => {
            ensureLanguages()
            return refractor
          },
        }))
      })
      // baseDir ctx 必须在使用它的 plugin 之前 use
      .use(seeyueBaseDirCtx)
      .use(commonmark)
      .use(gfm)
      // htmlInlineView 装在 commonmark 之后 —— 替换默认 html schema 的
      // NodeView，让 inline html 通过 innerHTML 真渲染（块级 HTML 内容
      // 也走这条路径，NodeView 内会按 value 选择 div/span 包装）
      .use(htmlInlineView)
      .use(listener)
      .use(history)
      .use(cursor)
      .use(clipboard)
      .use(indent)
      .use(prism)
      .use(seeyueHeadingIdSync)
      .use(seeyueImageResolver(baseDirRef.current)),
  )

  // —— /api/parse debounce —— 仅给 TOC 用，编辑面本身不依赖
  useEffect(() => {
    let cancelled = false
    const t = window.setTimeout(async () => {
      try {
        const res = await fetch('./api/parse', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ source: tab.source }),
        })
        if (!res.ok) return
        const doc = (await res.json()) as ParsedDocument
        if (!cancelled) onParsed(doc)
      } catch {
        /* 静默 */
      }
    }, PARSE_DEBOUNCE_MS)
    return () => {
      cancelled = true
      window.clearTimeout(t)
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tab.source])

  return <Milkdown />
}

export function MilkdownEditor(props: Props) {
  return (
    <div className="h-full overflow-y-auto bg-seeyue-bg">
      <div className="seeyue-prose max-w-3xl mx-auto px-8 py-8">
        <MilkdownProvider>
          <MilkdownInner
            tab={props.tab}
            baseDir={props.baseDir}
            onChange={props.onChange}
            onParsed={props.onParsed}
          />
        </MilkdownProvider>
      </div>
    </div>
  )
}
