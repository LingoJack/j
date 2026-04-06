import { useMemo } from 'react'
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter'
import { oneLight } from 'react-syntax-highlighter/dist/esm/styles/prism'
import { CopyButton } from '../common/CopyButton'

// Language mapping for syntax highlighting
const langMap: Record<string, string> = {
  'bash': 'bash',
  'shell': 'bash',
  'sh': 'bash',
  'zsh': 'bash',
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

// Render inline markdown elements (bold, code, links)
function renderInlineMarkdown(text: string, baseKey: string): React.ReactNode {
  const parts: React.ReactNode[] = []
  let remaining = text
  let keyIndex = 0
  
  while (remaining.length > 0) {
    // Find all matches and pick the one with smallest index
    const codeMatch = remaining.match(/`([^`]+)`/)
    const boldMatch = remaining.match(/\*\*([^*]+)\*\*/)
    const italicMatch = remaining.match(/\*([^*]+)\*/)
    
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
    }
    
    remaining = remaining.slice(first.index + first.match[0].length)
  }
  
  return parts.length > 0 ? parts : text
}

interface MarkdownProps {
  content: string
}

export function Markdown({ content }: MarkdownProps) {
  // Use content hash as key prefix to force re-render on content change
  const elements = useMemo(() => {
    const lines = content.split('\n')
    const result: React.JSX.Element[] = []
    let inCodeBlock = false
    let codeContent = ''
    let codeLang = ''
    let inTable = false
    let tableRows: string[][] = []
    let blockCounter = 0
    const usedIds = new Set<string>()
    
    const flushTable = () => {
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
        tableRows = []
      }
    }
    
    lines.forEach((line) => {
      const lineKey = `line-${blockCounter++}`
      
      // Code blocks
      if (line.startsWith('```')) {
        if (!inCodeBlock) {
          flushTable()
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
        inTable = false
        flushTable()
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
    
    // Handle any remaining open blocks after loop ends
    if (inTable) {
      flushTable()
    }
    
    return result
  }, [content])
  
  return <>{elements}</>
}
