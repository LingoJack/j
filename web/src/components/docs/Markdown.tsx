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

// Render inline markdown elements (bold, code, links)
function renderInlineMarkdown(text: string): React.ReactNode {
  const parts: React.ReactNode[] = []
  let remaining = text
  let key = 0
  
  while (remaining.length > 0) {
    // Inline code
    const codeMatch = remaining.match(/`([^`]+)`/)
    if (codeMatch && codeMatch.index !== undefined) {
      const before = remaining.slice(0, codeMatch.index)
      if (before) {
        parts.push(<span key={key++}>{renderInlineMarkdown(before)}</span>)
      }
      parts.push(
        <code key={key++} className="bg-stone-100 text-stone-700 px-1.5 py-0.5 rounded text-xs font-mono">
          {codeMatch[1]}
        </code>
      )
      remaining = remaining.slice(codeMatch.index + codeMatch[0].length)
      continue
    }
    
    // Bold
    const boldMatch = remaining.match(/\*\*([^*]+)\*\*/)
    if (boldMatch && boldMatch.index !== undefined) {
      const before = remaining.slice(0, boldMatch.index)
      if (before) {
        parts.push(<span key={key++}>{renderInlineMarkdown(before)}</span>)
      }
      parts.push(
        <strong key={key++} className="font-medium text-stone-900">
          {boldMatch[1]}
        </strong>
      )
      remaining = remaining.slice(boldMatch.index + boldMatch[0].length)
      continue
    }
    
    // Italic
    const italicMatch = remaining.match(/\*([^*]+)\*/)
    if (italicMatch && italicMatch.index !== undefined) {
      const before = remaining.slice(0, italicMatch.index)
      if (before) {
        parts.push(<span key={key++}>{renderInlineMarkdown(before)}</span>)
      }
      parts.push(
        <em key={key++} className="italic">
          {italicMatch[1]}
        </em>
      )
      remaining = remaining.slice(italicMatch.index + italicMatch[0].length)
      continue
    }
    
    // No more matches, push remaining text
    parts.push(<span key={key++}>{remaining}</span>)
    break
  }
  
  return parts.length > 0 ? parts : text
}

interface MarkdownProps {
  content: string
}

export function Markdown({ content }: MarkdownProps) {
  const lines = content.split('\n')
  const elements: React.JSX.Element[] = []
  let inCodeBlock = false
  let codeContent = ''
  let codeLang = ''
  let inTable = false
  let tableRows: string[][] = []
  
  lines.forEach((line, index) => {
    // Code blocks
    if (line.startsWith('```')) {
      if (!inCodeBlock) {
        inCodeBlock = true
        codeLang = line.slice(3).trim() || 'text'
        codeContent = ''
      } else {
        inCodeBlock = false
        const lang = langMap[codeLang.toLowerCase()] || codeLang || 'text'
        
        elements.push(
          <div key={index} className="relative group my-4">
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
      const maxCols = Math.max(...tableRows.map(row => row.length))
      elements.push(
        <div key={`table-${index}`} className="overflow-x-auto my-4">
          <table className="min-w-full border-collapse">
            <thead>
              <tr>
                {tableRows[0]?.map((cell, i) => (
                  <th key={i} className="border border-stone-200 px-4 py-2 text-left bg-stone-50 text-sm font-medium">
                    {renderInlineMarkdown(cell)}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {tableRows.slice(1).map((row, i) => (
                <tr key={i}>
                  {Array.from({ length: maxCols }).map((_, j) => (
                    <td key={j} className="border border-stone-200 px-4 py-2 text-sm">
                      {renderInlineMarkdown(row[j] || '')}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )
    }
    
    // Blockquotes
    if (line.startsWith('> ')) {
      elements.push(
        <blockquote key={index} className="border-l-4 border-stone-300 pl-4 py-1 my-3 text-stone-600 text-sm italic">
          {renderInlineMarkdown(line.slice(2))}
        </blockquote>
      )
      return
    }
    
    // Headings
    if (line.startsWith('## ')) {
      elements.push(<h2 key={index} className="text-2xl font-light text-stone-900 mt-8 mb-4">{renderInlineMarkdown(line.slice(3))}</h2>)
      return
    }
    if (line.startsWith('### ')) {
      elements.push(<h3 key={index} className="text-lg font-medium text-stone-900 mt-6 mb-3">{renderInlineMarkdown(line.slice(4))}</h3>)
      return
    }
    
    // Lists
    if (line.startsWith('- ') || line.startsWith('* ')) {
      elements.push(
        <li key={index} className="text-stone-600 text-sm ml-4 mb-1 list-disc">
          {renderInlineMarkdown(line.slice(2))}
        </li>
      )
      return
    }
    
    // Numbered lists
    const numMatch = line.match(/^(\d+)\.\s/)
    if (numMatch) {
      elements.push(
        <li key={index} className="text-stone-600 text-sm ml-4 mb-1 list-decimal">
          {renderInlineMarkdown(line.slice(numMatch[0].length))}
        </li>
      )
      return
    }
    
    // Paragraphs
    if (line.trim()) {
      elements.push(
        <p key={index} className="text-stone-600 text-sm leading-relaxed mb-3">
          {renderInlineMarkdown(line)}
        </p>
      )
    }
  })
  
  return <>{elements}</>
}
