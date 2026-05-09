import { useState } from 'react'
import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter'
import { oneDark, oneLight } from 'react-syntax-highlighter/dist/esm/styles/prism'

const darkCodeStyle = {
  ...oneDark,
  'pre[class*="language-"]': {
    ...oneDark['pre[class*="language-"]'],
    margin: '6px 0',
    borderRadius: '6px',
    fontSize: '13px',
    background: 'rgba(30,30,42,0.9)',
  },
  'code[class*="language-"]': {
    ...oneDark['code[class*="language-"]'],
    fontSize: '13px',
    background: 'none',
  },
}

const lightCodeStyle = {
  ...oneLight,
  'pre[class*="language-"]': {
    ...oneLight['pre[class*="language-"]'],
    margin: '6px 0',
    borderRadius: '6px',
    fontSize: '13px',
    background: 'rgba(240,240,240,0.9)',
  },
  'code[class*="language-"]': {
    ...oneLight['code[class*="language-"]'],
    fontSize: '13px',
    background: 'none',
  },
}

function CopyButton({ text }) {
  const [copied, setCopied] = useState(false)
  const handleCopy = () => {
    navigator.clipboard?.writeText(text)
    setCopied(true)
    setTimeout(() => setCopied(false), 1500)
  }
  return (
    <button
      className="absolute top-1.5 right-1.5 px-1.5 py-0.5 rounded text-[10px] bg-bg3/60 text-fg3 hover:text-fg hover:bg-bg3 active:scale-[0.92] transition-all duration-100 select-none opacity-0 group-hover:opacity-100"
      onClick={handleCopy}
    >{copied ? '✓' : '复制'}</button>
  )
}

function CodeBlock({ className, children }) {
  const match = /language-(\w+)/.exec(className || '')
  const code = String(children).replace(/\n$/, '')
  const theme = document.documentElement.getAttribute('data-theme') || 'dark'
  const codeStyle = theme === 'light' ? lightCodeStyle : darkCodeStyle

  if (match) {
    return (
      <div className="relative group">
        <SyntaxHighlighter style={codeStyle} language={match[1]} PreTag="div">
          {code}
        </SyntaxHighlighter>
        <CopyButton text={code} />
      </div>
    )
  }

  return (
    <code className={className}>{children}</code>
  )
}

function Table({ children }) {
  return (
    <div className="table-wrap">
      <table>{children}</table>
    </div>
  )
}

export default function Markdown({ content }) {
  return (
    <ReactMarkdown
      remarkPlugins={[remarkGfm]}
      components={{
        code: CodeBlock,
        table: Table,
      }}
    >
      {content}
    </ReactMarkdown>
  )
}
