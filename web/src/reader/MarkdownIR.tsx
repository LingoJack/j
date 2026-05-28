import { createContext, useContext } from 'react'
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter'
import { oneDark } from 'react-syntax-highlighter/dist/esm/styles/prism'
import { CopyButton } from '../components/common/CopyButton'
import type { Block, Inline, ListData, ListItem, TableData, ParsedDocument } from './types'

// ---------------------------------------------------------------------------
// BaseDir Context — 用于解析 markdown 里的相对图片路径
// ---------------------------------------------------------------------------

/** 当前文档所在目录的绝对路径；目录入口 / 找不到时为 null */
export const MarkdownBaseDirContext = createContext<string | null>(null)

/** 把图片 url 转成可由浏览器加载的实际 src */
function resolveAssetUrl(url: string, baseDir: string | null): string {
  // 远程 / data URL 直通
  if (/^(https?:|data:)/i.test(url)) return url
  // 绝对路径
  if (url.startsWith('/')) {
    return `./api/asset?path=${encodeURIComponent(url)}`
  }
  // 相对路径：拼到 baseDir 后面
  if (!baseDir) return url
  // 简单拼接，然后规范化 ./ 与 ../
  const joined = (baseDir.endsWith('/') ? baseDir : baseDir + '/') + url
  const normalized = normalizePath(joined)
  return `./api/asset?path=${encodeURIComponent(normalized)}`
}

/** 规范化绝对路径，消除 `./` `../` 段 */
function normalizePath(p: string): string {
  const parts = p.split('/')
  const out: string[] = []
  for (const seg of parts) {
    if (seg === '' || seg === '.') continue
    if (seg === '..') {
      if (out.length > 0) out.pop()
      continue
    }
    out.push(seg)
  }
  return '/' + out.join('/')
}

// ---------------------------------------------------------------------------
// Heading ID 生成
// ---------------------------------------------------------------------------

/** 从 Inline[] 提取纯文本 */
export function extractText(inlines: Inline[]): string {
  return inlines
    .map((inline) => {
      switch (inline.type) {
        case 'text':
          return inline.value
        case 'strong':
        case 'emphasis':
        case 'strikethrough':
          return extractText(inline.value)
        case 'code':
          return inline.value
        case 'link':
          return extractText(inline.value.text)
        default:
          return ''
      }
    })
    .join('')
}

let _headingIdCounter: Map<string, number> = new Map()

/** 生成 heading id（支持重复标题去重） */
function headingId(text: string): string {
  const slug = text
    .toLowerCase()
    .replace(/[^\w\u4e00-\u9fff]+/g, '-')
    .replace(/^-|-$/g, '')
  const count = _headingIdCounter.get(slug) ?? 0
  _headingIdCounter.set(slug, count + 1)
  return count === 0 ? slug : `${slug}-${count}`
}

/** 重置 heading id 计数器（每次渲染前调用） */
export function resetHeadingIdCounter() {
  _headingIdCounter = new Map()
}

// 与 `web/src/components/docs/Markdown.tsx` 同一份语言映射，保持代码块高亮一致。
const langMap: Record<string, string> = {
  bash: 'bash', shell: 'bash', sh: 'bash', zsh: 'bash',
  powershell: 'powershell', ps1: 'powershell',
  typescript: 'typescript', ts: 'typescript',
  javascript: 'javascript', js: 'javascript',
  python: 'python', py: 'python',
  rust: 'rust', rs: 'rust',
  go: 'go', golang: 'go',
  java: 'java',
  c: 'c', cpp: 'cpp', 'c++': 'cpp',
  csharp: 'csharp', 'c#': 'csharp',
  ruby: 'ruby', rb: 'ruby',
  sql: 'sql', json: 'json',
  yaml: 'yaml', yml: 'yaml', toml: 'toml',
  markdown: 'markdown', md: 'markdown',
  html: 'html', css: 'css', scss: 'scss',
}

// ---------------------------------------------------------------------------
// Inline 渲染
// ---------------------------------------------------------------------------

function renderInline(inline: Inline, key: string): React.ReactNode {
  switch (inline.type) {
    case 'text':
      return <span key={key}>{inline.value}</span>
    case 'strong':
      return (
        <strong key={key} className="font-semibold text-seeyue-fg-strong">
          {renderInlineList(inline.value, key)}
        </strong>
      )
    case 'emphasis':
      return (
        <em key={key} className="italic text-seeyue-fg">
          {renderInlineList(inline.value, key)}
        </em>
      )
    case 'strikethrough':
      return (
        <del key={key} className="line-through text-seeyue-fg-dim">
          {renderInlineList(inline.value, key)}
        </del>
      )
    case 'code':
      // 行内 code 样式由 reader.css 的 .seeyue-prose :not(pre) > code 提供
      return (
        <code key={key}>{inline.value}</code>
      )
    case 'link':
      return (
        <a
          key={key}
          href={inline.value.url}
          target="_blank"
          rel="noreferrer noopener"
          className="seeyue-link"
        >
          {renderInlineList(inline.value.text, key)}
        </a>
      )
    case 'image':
      return <ImageInline key={key} url={inline.value.url} alt={inline.value.alt} />
    case 'soft_break':
      return <span key={key}> </span>
    case 'hard_break':
      return <br key={key} />
  }
}

/** 行内图片组件：从 Context 取 baseDir 解析相对路径 */
function ImageInline({ url, alt }: { url: string; alt: string }) {
  const baseDir = useContext(MarkdownBaseDirContext)
  const src = resolveAssetUrl(url, baseDir)
  return (
    <img
      src={src}
      alt={alt}
      className="max-w-full inline-block rounded my-2 border border-seeyue-border"
      loading="lazy"
    />
  )
}

function renderInlineList(items: Inline[], baseKey: string): React.ReactNode[] {
  return items.map((item, i) => renderInline(item, `${baseKey}-${i}`))
}

// ---------------------------------------------------------------------------
// Block 渲染
// ---------------------------------------------------------------------------

const ALIGN_CLASS: Record<string, string> = {
  none: 'text-left',
  left: 'text-left',
  center: 'text-center',
  right: 'text-right',
}

function renderTable(data: TableData, key: string): React.ReactNode {
  const [headerRow, ...bodyRows] = data.rows
  const colCount = data.rows.reduce((max, row) => Math.max(max, row.length), 0)
  const alignClass = (col: number) => ALIGN_CLASS[data.alignments[col] ?? 'none'] ?? 'text-left'
  return (
    <div key={key} className="overflow-x-auto my-6 rounded-lg border border-seeyue-border">
      <table className="min-w-full border-collapse text-[15px]">
        {headerRow && (
          <thead>
            <tr className="bg-seeyue-panel">
              {Array.from({ length: colCount }).map((_, c) => (
                <th
                  key={`h${c}`}
                  className={`border-b border-seeyue-border px-4 py-2.5 font-medium text-seeyue-fg-strong ${alignClass(c)}`}
                >
                  {renderInlineList(headerRow[c] ?? [], `${key}-h${c}`)}
                </th>
              ))}
            </tr>
          </thead>
        )}
        <tbody>
          {bodyRows.map((row, r) => (
            <tr key={`r${r}`} className="border-b border-seeyue-border last:border-0">
              {Array.from({ length: colCount }).map((_, c) => (
                <td
                  key={`c${c}`}
                  className={`px-4 py-2.5 text-seeyue-fg ${alignClass(c)}`}
                >
                  {renderInlineList(row[c] ?? [], `${key}-r${r}c${c}`)}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

function renderListItem(item: ListItem, key: string): React.ReactNode {
  const taskCheckbox =
    item.checked === null ? null : (
      <input
        type="checkbox"
        checked={item.checked}
        readOnly
        className="mr-2 align-middle accent-seeyue-accent"
      />
    )
  return (
    <li key={key} className="text-seeyue-fg text-[15px] leading-7 mb-1.5 marker:text-seeyue-fg-dim">
      {taskCheckbox}
      {renderInlineList(item.content, `${key}-c`)}
      {item.children.length > 0 && (
        <div className="mt-1.5">{renderBlocks(item.children, `${key}-ch`)}</div>
      )}
    </li>
  )
}

function renderList(data: ListData, key: string): React.ReactNode {
  const baseClass = 'pl-6 my-4 space-y-1'
  if (data.ordered) {
    return (
      <ol
        key={key}
        className={`list-decimal ${baseClass}`}
        start={data.start_index ?? 1}
      >
        {data.items.map((item, i) => renderListItem(item, `${key}-i${i}`))}
      </ol>
    )
  }
  return (
    <ul key={key} className={`list-disc ${baseClass}`}>
      {data.items.map((item, i) => renderListItem(item, `${key}-i${i}`))}
    </ul>
  )
}

function renderBlock(block: Block, key: string): React.ReactNode {
  const kind = block.kind
  const srcAttrs = {
    'data-src-start': block.source.start_line,
    'data-src-end': block.source.end_line,
  } as Record<string, number>
  switch (kind.type) {
    case 'paragraph':
      return (
        <p key={key} {...srcAttrs} className="text-seeyue-fg text-[15px] leading-7 mb-5">
          {renderInlineList(kind.value, `${key}-p`)}
        </p>
      )
    case 'heading': {
      const { level, content } = kind.value
      const cls =
        level === 1
          ? 'text-3xl font-semibold text-seeyue-fg-strong tracking-tight mt-2 mb-7 pb-3 border-b border-seeyue-border'
          : level === 2
            ? 'text-2xl font-semibold text-seeyue-fg-strong tracking-tight mt-12 mb-4'
            : level === 3
              ? 'text-xl font-medium text-seeyue-fg-strong tracking-tight mt-9 mb-3'
              : level === 4
                ? 'text-base font-semibold text-seeyue-fg-strong mt-6 mb-2.5'
                : 'text-sm font-semibold uppercase tracking-wider text-seeyue-fg-muted mt-5 mb-2'
      const Tag = (`h${Math.min(Math.max(level, 1), 6)}` as unknown) as keyof React.JSX.IntrinsicElements
      const text = extractText(content)
      const id = headingId(text)
      return (
        <Tag key={key} id={id} {...srcAttrs} className={`scroll-mt-20 ${cls}`}>
          {renderInlineList(content, `${key}-h`)}
        </Tag>
      )
    }
    case 'code_block': {
      const { lang: rawLang, code } = kind.value
      const lang = langMap[rawLang.toLowerCase()] || rawLang || 'text'
      const langLabel = (rawLang || 'text').toUpperCase()
      return (
        <div key={key} {...srcAttrs} className="seeyue-codeblock relative group">
          <span className="seeyue-codeblock-lang">{langLabel}</span>
          <SyntaxHighlighter
            language={lang}
            style={oneDark}
            customStyle={{
              margin: 0,
              borderRadius: 0,
              fontSize: '13.5px',
              lineHeight: '1.65',
              padding: '1rem 1.1rem',
              background: 'transparent',
              border: 'none',
            }}
            codeTagProps={{
              style: {
                fontFamily:
                  '"JetBrains Mono", ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace',
              },
            }}
          >
            {code}
          </SyntaxHighlighter>
          <CopyButton text={code} />
        </div>
      )
    }
    case 'table':
      return (
        <div key={key} {...srcAttrs}>
          {renderTable(kind.value, key)}
        </div>
      )
    case 'list':
      return (
        <div key={key} {...srcAttrs}>
          {renderList(kind.value, key)}
        </div>
      )
    case 'block_quote':
      return (
        <blockquote
          key={key}
          {...srcAttrs}
          className="border-l-2 border-seeyue-border-strong pl-5 py-1 my-5 text-seeyue-fg-muted text-[15px] leading-7 [&>p]:mb-2 [&>p:last-child]:mb-0"
        >
          {renderBlocks(kind.value, `${key}-bq`)}
        </blockquote>
      )
    case 'rule':
      return <hr key={key} {...srcAttrs} />
  }
}

function renderBlocks(blocks: Block[], baseKey: string): React.ReactNode {
  return blocks.map((b, i) => renderBlock(b, `${baseKey}-${i}`))
}

/** 单个 block 渲染（供 MarkdownLiveEditor 局部调用，按需在外部驱动 reset id 计数） */
export function renderSingleBlock(block: Block, key: string): React.ReactNode {
  return renderBlock(block, key)
}

// ---------------------------------------------------------------------------
// 公开组件
// ---------------------------------------------------------------------------

interface Props {
  doc: ParsedDocument
}

export function MarkdownIR({ doc }: Props) {
  resetHeadingIdCounter()
  return <>{renderBlocks(doc.blocks, 'b')}</>
}
