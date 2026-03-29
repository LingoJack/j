import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter'
import { oneDark } from 'react-syntax-highlighter/dist/esm/styles/prism'

const codeStyle = {
  ...oneDark,
  'pre[class*="language-"]': {
    ...oneDark['pre[class*="language-"]'],
    margin: '6px 0',
    borderRadius: '8px',
    fontSize: '13px',
    background: 'rgba(0,0,0,0.35)',
  },
  'code[class*="language-"]': {
    ...oneDark['code[class*="language-"]'],
    fontSize: '13px',
    background: 'none',
  },
}

function CodeBlock({ className, children, ...props }) {
  const match = /language-(\w+)/.exec(className || '')
  const code = String(children).replace(/\n$/, '')

  if (match) {
    return (
      <SyntaxHighlighter style={codeStyle} language={match[1]} PreTag="div">
        {code}
      </SyntaxHighlighter>
    )
  }

  return (
    <code className={className} {...props}>
      {children}
    </code>
  )
}

export default function Markdown({ content }) {
  return (
    <ReactMarkdown
      remarkPlugins={[remarkGfm]}
      components={{ code: CodeBlock }}
    >
      {content}
    </ReactMarkdown>
  )
}
