import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter'
import { oneLight } from 'react-syntax-highlighter/dist/esm/styles/prism'
import type { Block, Inline, ListData, ListItem, TableData, ParsedDocument } from './types'

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
        <strong key={key} className="font-medium text-stone-900">
          {renderInlineList(inline.value, key)}
        </strong>
      )
    case 'emphasis':
      return (
        <em key={key} className="italic">
          {renderInlineList(inline.value, key)}
        </em>
      )
    case 'strikethrough':
      return (
        <del key={key} className="line-through text-stone-400">
          {renderInlineList(inline.value, key)}
        </del>
      )
    case 'code':
      return (
        <code
          key={key}
          className="bg-stone-100 text-stone-700 px-1.5 py-0.5 rounded text-xs font-mono"
        >
          {inline.value}
        </code>
      )
    case 'link':
      return (
        <a
          key={key}
          href={inline.value.url}
          target="_blank"
          rel="noreferrer noopener"
          className="text-stone-700 underline decoration-stone-300 hover:decoration-stone-700"
        >
          {renderInlineList(inline.value.text, key)}
        </a>
      )
    case 'soft_break':
      return <br key={key} />
    case 'hard_break':
      return <br key={key} />
  }
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
    <div key={key} className="overflow-x-auto my-4">
      <table className="min-w-full border-collapse">
        {headerRow && (
          <thead>
            <tr>
              {Array.from({ length: colCount }).map((_, c) => (
                <th
                  key={`h${c}`}
                  className={`border border-stone-200 px-4 py-2 bg-stone-50 text-sm font-medium ${alignClass(c)}`}
                >
                  {renderInlineList(headerRow[c] ?? [], `${key}-h${c}`)}
                </th>
              ))}
            </tr>
          </thead>
        )}
        <tbody>
          {bodyRows.map((row, r) => (
            <tr key={`r${r}`}>
              {Array.from({ length: colCount }).map((_, c) => (
                <td
                  key={`c${c}`}
                  className={`border border-stone-200 px-4 py-2 text-sm ${alignClass(c)}`}
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
        className="mr-2 align-middle"
      />
    )
  return (
    <li key={key} className="text-stone-600 text-sm mb-1">
      {taskCheckbox}
      {renderInlineList(item.content, `${key}-c`)}
      {item.children.length > 0 && (
        <div className="mt-1">{renderBlocks(item.children, `${key}-ch`)}</div>
      )}
    </li>
  )
}

function renderList(data: ListData, key: string): React.ReactNode {
  const baseClass = 'pl-6 my-3'
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
  switch (kind.type) {
    case 'paragraph':
      return (
        <p key={key} className="text-stone-600 text-sm leading-relaxed mb-3">
          {renderInlineList(kind.value, `${key}-p`)}
        </p>
      )
    case 'heading': {
      const { level, content } = kind.value
      const cls =
        level === 1
          ? 'text-3xl font-light text-stone-900 mt-10 mb-6'
          : level === 2
            ? 'text-2xl font-light text-stone-900 mt-12 mb-5'
            : level === 3
              ? 'text-lg font-medium text-stone-900 mt-8 mb-4'
              : level === 4
                ? 'text-base font-semibold text-stone-800 mt-6 mb-3'
                : 'text-sm font-semibold text-stone-700 mt-5 mb-2'
      const Tag = (`h${Math.min(Math.max(level, 1), 6)}` as unknown) as keyof React.JSX.IntrinsicElements
      const text = extractText(content)
      const id = headingId(text)
      return (
        <Tag key={key} id={id} className={cls}>
          {renderInlineList(content, `${key}-h`)}
        </Tag>
      )
    }
    case 'code_block': {
      const { lang: rawLang, code } = kind.value
      const lang = langMap[rawLang.toLowerCase()] || rawLang || 'text'
      return (
        <div key={key} className="my-4">
          <SyntaxHighlighter
            language={lang}
            style={oneLight}
            customStyle={{
              margin: 0,
              borderRadius: '0.5rem',
              fontSize: '0.875rem',
              backgroundColor: '#faf9f6',
              border: '1px solid #e7e5e4',
            }}
            codeTagProps={{
              style: {
                fontFamily:
                  'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace',
              },
            }}
          >
            {code}
          </SyntaxHighlighter>
        </div>
      )
    }
    case 'table':
      return renderTable(kind.value, key)
    case 'list':
      return renderList(kind.value, key)
    case 'block_quote':
      return (
        <blockquote
          key={key}
          className="border-l-4 border-stone-300 pl-4 py-1 my-3 text-stone-600 text-sm italic"
        >
          {renderBlocks(kind.value, `${key}-bq`)}
        </blockquote>
      )
    case 'rule':
      return <hr key={key} className="my-6 border-stone-200" />
  }
}

function renderBlocks(blocks: Block[], baseKey: string): React.ReactNode {
  return blocks.map((b, i) => renderBlock(b, `${baseKey}-${i}`))
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
