import { CopyButton } from './CopyButton'

interface CodeBlockProps {
  children: string
  showCopy?: boolean
  className?: string
}

export function CodeBlock({ children, showCopy = true, className = '' }: CodeBlockProps) {
  return (
    <div className={`relative group max-w-full ${className}`}>
      <pre className="bg-[#faf9f6] text-stone-800 rounded-lg p-4 text-sm overflow-x-auto font-mono border border-stone-200">
        <code className="block whitespace-pre">{children}</code>
      </pre>
      {showCopy && <CopyButton text={children} />}
    </div>
  )
}
