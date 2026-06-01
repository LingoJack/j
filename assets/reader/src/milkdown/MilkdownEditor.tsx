/**
 * Reader 的 Milkdown 编辑器封装。
 *
 * - 单一 contenteditable（ProseMirror）编辑面，光标可在段落、列表、表格、
 *   代码块之间自由移动 —— Typora 体验的本质。
 * - 监听 markdown 变化 → `onChange(path, source)`；外层 Reader 据此打 dirty。
 * - 内部维护一份 latestSourceRef，配合 150ms debounce 触发 /api/parse 给
 *   右栏 TOC 喂数据 —— 这套不再依赖 React state，避免按键级 re-render。
 * - 切 tab 由 Reader.tsx 通过 `key={path}` 触发整体 unmount/remount，
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
// 关键性能修复：用自研带缓存的 prism 插件替代 @milkdown/plugin-prism。
// 上游插件每次按键都会把整个文档的所有代码块重新跑 refractor.highlight，
// 大文档下卡顿明显。我们的版本按 (lang, text) 缓存高亮结果。
import { seeyuePrismBundle as prism, prismConfig } from './prismCached'
import { perfProbe } from './perfProbe'
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
import type { ParsedDocument } from '../types'

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
  path: string
  baseDir: string | null
  initialSource: string
  onChange: (path: string, source: string) => void
  onParsed: (path: string, doc: ParsedDocument) => void
  onSave: () => void | Promise<void>
}

/**
 * debounce 时间：source 变化后多久 POST /api/parse（仅供 TOC 使用）。
 *
 * TOC 不需要按键级实时，500ms 几乎察觉不到延迟，但能把 parse 频率降下来 ——
 * 服务端 parse_markdown 是 CPU-bound，对大文件可能耗 100~800ms，频繁调会
 * 阻塞 worker。配合下方 in-flight + pending 节流，永远只有一个 parse 在飞。
 */
const PARSE_DEBOUNCE_MS = 500

function MilkdownInner({
  path,
  baseDir,
  initialSource,
  onChange,
  onParsed,
}: Omit<Props, 'onSave'>) {
  ensureLanguages()

  // —— 桥接最新闭包，避免 useEditor (deps=[]) 吃 stale ——
  const onChangeRef = useRef(onChange)
  onChangeRef.current = onChange
  const onParsedRef = useRef(onParsed)
  onParsedRef.current = onParsed
  const pathRef = useRef(path)
  pathRef.current = path

  // initialSource 只在 mount 时被读
  const initialSourceRef = useRef(initialSource)
  const baseDirRef = useRef(baseDir)
  baseDirRef.current = baseDir

  // —— 内部 debounce 维护 /api/parse —— 不再走 React state
  // 这样按键级 onChange 只走 ref/timer，不触发任何 re-render
  const latestSourceRef = useRef(initialSource)
  const parseTimerRef = useRef<number | null>(null)
  // in-flight 节流：parse 期间不允许另一个并发，结束后看是否有 pending
  const parseInFlightRef = useRef(false)
  const parsePendingRef = useRef(false)

  const triggerParse = () => {
    if (parseInFlightRef.current) {
      parsePendingRef.current = true
      return
    }
    parseInFlightRef.current = true
    parsePendingRef.current = false
    void runParse(latestSourceRef.current, pathRef.current, onParsedRef).finally(
      () => {
        parseInFlightRef.current = false
        if (parsePendingRef.current) {
          parsePendingRef.current = false
          // 立刻发起下一次（已经至少 PARSE_DEBOUNCE_MS 又积累了变更）
          triggerParse()
        }
      },
    )
  }

  useEditor((root) =>
    Editor.make()
      .config((ctx) => {
        ctx.set(rootCtx, root)
        ctx.set(defaultValueCtx, initialSourceRef.current)
        ctx.set(seeyueBaseDirCtx.key, baseDirRef.current)
        ctx.get(listenerCtx).markdownUpdated((_ctx, md, prev) => {
          if (md === prev) return
          latestSourceRef.current = md
          onChangeRef.current(pathRef.current, md)
          // 重新 debounce 一次 /api/parse
          if (parseTimerRef.current != null) {
            window.clearTimeout(parseTimerRef.current)
          }
          parseTimerRef.current = window.setTimeout(() => {
            triggerParse()
          }, PARSE_DEBOUNCE_MS)
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
      .use(perfProbe)
      .use(seeyueHeadingIdSync)
      .use(seeyueImageResolver(baseDirRef.current)),
  )

  // —— mount 后立刻 parse 一次 —— 切 tab 时也会跑（Milkdown 整体 remount）
  // —— unmount 时清掉 pending timer
  useEffect(() => {
    return () => {
      if (parseTimerRef.current != null) {
        window.clearTimeout(parseTimerRef.current)
      }
    }
  }, [])

  return <Milkdown />
}

async function runParse(
  source: string,
  path: string,
  onParsedRef: React.RefObject<
    (path: string, doc: ParsedDocument) => void
  >,
) {
  try {
    const res = await fetch('./api/parse', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ source }),
    })
    if (!res.ok) return
    const doc = (await res.json()) as ParsedDocument
    onParsedRef.current?.(path, doc)
  } catch {
    /* 静默 */
  }
}

export function MilkdownEditor(props: Props) {
  return (
    <div className="h-full overflow-y-auto bg-seeyue-bg">
      {/* 行宽：跟 Typora 一样按视口百分比走，不再钉死 max-w-3xl。
          原来 max-w-3xl ≈ 768px，2k 行的 markdown 几乎每个段落都要换 2 行，
          ProseMirror DOM 节点数量翻倍 → 输入和滚动都慢一截。
          现在 min(75vw, 1100px)，桌面常见 1440~2560px 屏上行宽更接近 Typora。 */}
      <div
        className="seeyue-prose mx-auto px-8 py-8"
        style={{ width: 'min(75vw, 1100px)', maxWidth: '100%' }}
      >
        <MilkdownProvider>
          <MilkdownInner
            path={props.path}
            baseDir={props.baseDir}
            initialSource={props.initialSource}
            onChange={props.onChange}
            onParsed={props.onParsed}
          />
        </MilkdownProvider>
      </div>
    </div>
  )
}
