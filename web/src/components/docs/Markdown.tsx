import { useMemo, useState, useCallback } from 'react'
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter'
import { oneLight } from 'react-syntax-highlighter/dist/esm/styles/prism'
import { CopyButton } from '../common/CopyButton'
import type { Platform } from '../common/TerminalWindow'

// Language mapping for syntax highlighting
const langMap: Record<string, string> = {
  'bash': 'bash',
  'shell': 'bash',
  'sh': 'bash',
  'zsh': 'bash',
  'powershell': 'powershell',
  'ps1': 'powershell',
  'typescript': 'typescript',
  'ts': 'typescript',
  'javascript': 'javascript',
  'js': 'javascript',
  'python': 'python',
  'py': 'python',
  'rust': 'rust',
  'rs': 'rust',
  'go': 'go',
  'golang': 'go',
  'java': 'java',
  'c': 'c',
  'cpp': 'cpp',
  'c++': 'cpp',
  'csharp': 'csharp',
  'c#': 'csharp',
  'ruby': 'ruby',
  'rb': 'ruby',
  'sql': 'sql',
  'json': 'json',
  'yaml': 'yaml',
  'yml': 'yaml',
  'toml': 'toml',
  'markdown': 'markdown',
  'md': 'markdown',
  'html': 'html',
  'css': 'css',
  'scss': 'scss',
}

// Generate slug from text (must match TOC.tsx)
function slugify(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^\w\u4e00-\u9fa5]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 50)
}

// Render inline markdown elements (bold, code, links, strikethrough)
function renderInlineMarkdown(text: string, baseKey: string): React.ReactNode {
  const parts: React.ReactNode[] = []
  let remaining = text
  let keyIndex = 0
  
  while (remaining.length > 0) {
    // Find all matches and pick the one with smallest index
    const codeMatch = remaining.match(/`([^`]+)`/)
    const boldMatch = remaining.match(/\*\*([^*]+)\*\*/)
    const italicMatch = remaining.match(/\*([^*]+)\*/)
    const strikeMatch = remaining.match(/~~([^~]+)~~/)
    
    // Collect all valid matches with their indices
    const matches: Array<{ type: string; match: RegExpMatchArray; index: number }> = []
    if (codeMatch && codeMatch.index !== undefined) {
      matches.push({ type: 'code', match: codeMatch, index: codeMatch.index })
    }
    if (boldMatch && boldMatch.index !== undefined) {
      matches.push({ type: 'bold', match: boldMatch, index: boldMatch.index })
    }
    if (italicMatch && italicMatch.index !== undefined) {
      matches.push({ type: 'italic', match: italicMatch, index: italicMatch.index })
    }
    if (strikeMatch && strikeMatch.index !== undefined) {
      matches.push({ type: 'strike', match: strikeMatch, index: strikeMatch.index })
    }
    
    // No matches found
    if (matches.length === 0) {
      parts.push(<span key={`${baseKey}-txt-${keyIndex++}`}>{remaining}</span>)
      break
    }
    
    // Sort by index and pick the first one
    matches.sort((a, b) => a.index - b.index)
    const first = matches[0]
    
    const before = remaining.slice(0, first.index)
    if (before) {
      parts.push(<span key={`${baseKey}-txt-${keyIndex++}`}>{before}</span>)
    }
    
    if (first.type === 'code') {
      parts.push(
        <code key={`${baseKey}-code-${keyIndex++}`} className="bg-stone-100 text-stone-700 px-1.5 py-0.5 rounded text-xs font-mono">
          {first.match[1]}
        </code>
      )
    } else if (first.type === 'bold') {
      parts.push(
        <strong key={`${baseKey}-bold-${keyIndex++}`} className="font-medium text-stone-900">
          {first.match[1]}
        </strong>
      )
    } else if (first.type === 'italic') {
      parts.push(
        <em key={`${baseKey}-italic-${keyIndex++}`} className="italic">
          {first.match[1]}
        </em>
      )
    } else if (first.type === 'strike') {
      parts.push(
        <del key={`${baseKey}-strike-${keyIndex++}`} className="line-through text-stone-400">
          {first.match[1]}
        </del>
      )
    }
    
    remaining = remaining.slice(first.index + first.match[0].length)
  }
  
  return parts.length > 0 ? parts : text
}

// ---------------------------------------------------------------------------
// PlatformCodeBlock: renders paired macOS/Linux + Windows code blocks
// inside a terminal-style window with tab switching.
// ---------------------------------------------------------------------------

interface CodeBlockData {
  lang: string
  content: string
}

function PlatformCodeBlock({
  unixCode,
  windowsCode,
  blockKey,
}: {
  unixCode: CodeBlockData
  windowsCode: CodeBlockData
  blockKey: string
}) {
  const [platform, setPlatform] = useState<Platform>('unix')
  const activeCode = platform === 'unix' ? unixCode : windowsCode
  const lang = langMap[activeCode.lang.toLowerCase()] || activeCode.lang || 'text'

  const handlePlatformChange = useCallback((p: Platform) => setPlatform(p), [])

  return (
    <div key={blockKey} className="my-4">
      {/* Platform tabs */}
      <div className="flex gap-2 mb-3">
        <button
          onClick={() => handlePlatformChange('unix')}
          className={`px-2.5 py-1 text-xs font-medium rounded-lg transition-colors ${
            platform === 'unix'
              ? 'bg-stone-900 text-white'
              : 'bg-stone-100 text-stone-500 hover:bg-stone-200'
          }`}
        >
          macOS / Linux
        </button>
        <button
          onClick={() => handlePlatformChange('windows')}
          className={`px-2.5 py-1 text-xs font-medium rounded-lg transition-colors ${
            platform === 'windows'
              ? 'bg-stone-900 text-white'
              : 'bg-stone-100 text-stone-500 hover:bg-stone-200'
          }`}
        >
          Windows
        </button>
      </div>

      {/* Code content */}
      <div className="relative group">
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
              fontFamily: 'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace',
            }
          }}
        >
          {activeCode.content}
        </SyntaxHighlighter>
        <CopyButton text={activeCode.content} />
      </div>
    </div>
  )
}

// ---------------------------------------------------------------------------
// Pre-processing: identify platform heading + code block pairs
// ---------------------------------------------------------------------------

type Segment =
  | { type: 'raw'; lines: string[] }
  | { type: 'platform-pair'; unixCode: CodeBlockData; windowsCode: CodeBlockData }

const PLATFORM_UNIX_RE = /^###\s+(macOS\s*\/\s*Linux|macOS|Linux)\s*$/
const PLATFORM_WIN_RE = /^###\s+Windows\s*$/

function extractSegments(lines: string[]): Segment[] {
  const segments: Segment[] = []
  let i = 0

    // Accumulate raw lines so multi-line structures (tables, code blocks) stay intact
    let rawBuf: string[] = []
    const flushRaw = () => {
      if (rawBuf.length > 0) {
        segments.push({ type: 'raw', lines: rawBuf })
        rawBuf = []
      }
    }

    while (i < lines.length) {
      // Look for: ### macOS / Linux -> ```lang -> code -> ``` -> ### Windows -> ```lang -> code -> ```
      if (PLATFORM_UNIX_RE.test(lines[i])) {
        const unixBlock = tryExtractCodeBlock(lines, i + 1)
        if (unixBlock) {
          // Skip blank lines between unix code block and Windows heading
          let winHeadingIdx = unixBlock.endIdx + 1
          while (winHeadingIdx < lines.length && lines[winHeadingIdx].trim() === '') winHeadingIdx++
          if (winHeadingIdx < lines.length && PLATFORM_WIN_RE.test(lines[winHeadingIdx])) {
            const winBlock = tryExtractCodeBlock(lines, winHeadingIdx + 1)
            if (winBlock) {
              flushRaw()
              segments.push({
                type: 'platform-pair',
                unixCode: { lang: unixBlock.lang, content: unixBlock.content },
                windowsCode: { lang: winBlock.lang, content: winBlock.content },
              })
              i = winBlock.endIdx + 1
              continue
            }
          }
        }
      }
      rawBuf.push(lines[i])
      i++
    }

    flushRaw()
    return segments
}

function tryExtractCodeBlock(lines: string[], startIdx: number): { lang: string; content: string; endIdx: number } | null {
  // Skip blank lines between heading and code block
  let idx = startIdx
  while (idx < lines.length && lines[idx].trim() === '') idx++
  if (idx >= lines.length || !lines[idx].startsWith('```')) return null

  const lang = lines[idx].slice(3).trim() || 'text'
  let content = ''
  idx++

  while (idx < lines.length && !lines[idx].startsWith('```')) {
    content += (content ? '\n' : '') + lines[idx]
    idx++
  }

  if (idx >= lines.length) return null // unclosed code block
  return { lang, content, endIdx: idx }
}

// ---------------------------------------------------------------------------
// Main Markdown component
// ---------------------------------------------------------------------------

interface MarkdownProps {
  content: string
}

export function Markdown({ content }: MarkdownProps) {
  const elements = useMemo(() => {
    const lines = content.split('\n')
    const segments = extractSegments(lines)
    const result: React.JSX.Element[] = []
    let blockCounter = 0
    const usedIds = new Set<string>()

    const flushTable = (tableRows: string[][]) => {
      if (tableRows.length > 0) {
        const maxCols = Math.max(...tableRows.map(row => row.length))
        const tableKey = `table-${blockCounter++}`
        result.push(
          <div key={tableKey} className="overflow-x-auto my-4">
            <table className="min-w-full border-collapse">
              <thead>
                <tr>
                  {tableRows[0]?.map((cell, i) => (
                    <th key={`th-${i}`} className="border border-stone-200 px-4 py-2 text-left bg-stone-50 text-sm font-medium">
                      {renderInlineMarkdown(cell, `${tableKey}-h${i}`)}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {tableRows.slice(1).map((row, i) => (
                  <tr key={`tr-${i}`}>
                    {Array.from({ length: maxCols }).map((_, j) => (
                      <td key={`td-${j}`} className="border border-stone-200 px-4 py-2 text-sm">
                        {renderInlineMarkdown(row[j] || '', `${tableKey}-r${i}c${j}`)}
                      </td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )
      }
    }

    // Render a batch of raw markdown lines (same logic as before)
    function renderRawLines(rawLines: string[]) {
      let inCodeBlock = false
      let codeContent = ''
      let codeLang = ''
      let inTable = false
      let tableRows: string[][] = []

      const closeTable = () => {
        if (inTable) {
          inTable = false
          flushTable(tableRows)
          tableRows = []
        }
      }

      rawLines.forEach((line) => {
        const lineKey = `line-${blockCounter++}`

        // Code blocks
        if (line.startsWith('```')) {
          if (!inCodeBlock) {
            closeTable()
            inCodeBlock = true
            codeLang = line.slice(3).trim() || 'text'
            codeContent = ''
          } else {
            inCodeBlock = false
            const lang = langMap[codeLang.toLowerCase()] || codeLang || 'text'

            result.push(
              <div key={`code-${blockCounter++}`} className="relative group my-4">
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
                      fontFamily: 'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace',
                    }
                  }}
                >
                  {codeContent}
                </SyntaxHighlighter>
                <CopyButton text={codeContent} />
              </div>
            )
          }
          return
        }

        if (inCodeBlock) {
          codeContent += (codeContent ? '\n' : '') + line
          return
        }

        // Tables
        if (line.startsWith('|')) {
          if (!inTable) {
            inTable = true
            tableRows = []
          }
          const cells = line.split('|').slice(1, -1).map(c => c.trim())
          if (!line.includes('---')) {
            tableRows.push(cells)
          }
          return
        } else if (inTable) {
          closeTable()
        }

        // Blockquotes
        if (line.startsWith('> ')) {
          result.push(
            <blockquote key={lineKey} className="border-l-4 border-stone-300 pl-4 py-1 my-3 text-stone-600 text-sm italic">
              {renderInlineMarkdown(line.slice(2), `${lineKey}-q`)}
            </blockquote>
          )
          return
        }

        // Headings
        if (line.startsWith('## ')) {
          const text = line.slice(3).trim()
          let id = slugify(text.replace(/\*\*([^*]+)\*\*/g, '$1').replace(/`([^`]+)`/g, '$1'))
          let counter = 1
          while (usedIds.has(id)) {
            id = `${slugify(text.replace(/\*\*([^*]+)\*\*/g, '$1').replace(/`([^`]+)`/g, '$1'))}-${counter}`
            counter++
          }
          usedIds.add(id)
          result.push(<h2 key={lineKey} id={id} className="text-2xl font-light text-stone-900 mt-12 mb-5">{renderInlineMarkdown(text, `${lineKey}-h2`)}</h2>)
          return
        }
        if (line.startsWith('### ')) {
          const text = line.slice(4).trim()
          let id = slugify(text.replace(/\*\*([^*]+)\*\*/g, '$1').replace(/`([^`]+)`/g, '$1'))
          let counter = 1
          while (usedIds.has(id)) {
            id = `${slugify(text.replace(/\*\*([^*]+)\*\*/g, '$1').replace(/`([^`]+)`/g, '$1'))}-${counter}`
            counter++
          }
          usedIds.add(id)
          result.push(<h3 key={lineKey} id={id} className="text-lg font-medium text-stone-900 mt-8 mb-4">{renderInlineMarkdown(text, `${lineKey}-h3`)}</h3>)
          return
        }
        if (line.startsWith('#### ')) {
          const text = line.slice(5).trim()
          let id = slugify(text.replace(/\*\*([^*]+)\*\*/g, '$1').replace(/`([^`]+)`/g, '$1'))
          let counter = 1
          while (usedIds.has(id)) {
            id = `${slugify(text.replace(/\*\*([^*]+)\*\*/g, '$1').replace(/`([^`]+)`/g, '$1'))}-${counter}`
            counter++
          }
          usedIds.add(id)
          result.push(<h4 key={lineKey} id={id} className="text-base font-semibold text-stone-800 mt-6 mb-3">{renderInlineMarkdown(text, `${lineKey}-h4`)}</h4>)
          return
        }

        // Lists
        if (line.startsWith('- ') || line.startsWith('* ')) {
          result.push(
            <li key={lineKey} className="text-stone-600 text-sm ml-4 mb-1 list-disc">
              {renderInlineMarkdown(line.slice(2), `${lineKey}-li`)}
            </li>
          )
          return
        }

        // Numbered lists
        const numMatch = line.match(/^(\d+)\.\s/)
        if (numMatch) {
          result.push(
            <li key={lineKey} className="text-stone-600 text-sm ml-4 mb-1 list-decimal">
              {renderInlineMarkdown(line.slice(numMatch[0].length), `${lineKey}-nli`)}
            </li>
          )
          return
        }

        // Paragraphs
        if (line.trim()) {
          result.push(
            <p key={lineKey} className="text-stone-600 text-sm leading-relaxed mb-3">
              {renderInlineMarkdown(line, `${lineKey}-p`)}
            </p>
          )
        }
      })

      // Flush any remaining open table
      if (inTable) {
        closeTable()
      }
    }

    // Process all segments
    for (const seg of segments) {
      if (seg.type === 'platform-pair') {
        const pairKey = `platform-${blockCounter++}`
        result.push(
          <PlatformCodeBlock
            key={pairKey}
            blockKey={pairKey}
            unixCode={seg.unixCode}
            windowsCode={seg.windowsCode}
          />
        )
      } else {
        renderRawLines(seg.lines)
      }
    }

    return result
  }, [content])
  
  return <>{elements}</>
}
