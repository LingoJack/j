import { useState } from 'react'

interface CopyButtonProps {
  text: string
  className?: string
}

export function CopyButton({ text, className = '' }: CopyButtonProps) {
  const [copied, setCopied] = useState(false)
  
  const handleCopy = async (e: React.MouseEvent) => {
    e.stopPropagation()
    await navigator.clipboard.writeText(text)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }
  
  return (
    <button
      onClick={handleCopy}
      className={`absolute right-3 top-3 px-3 py-1.5 text-xs font-medium 
                 text-stone-600 hover:text-stone-900 bg-white hover:bg-stone-50 
                 rounded border border-stone-300 hover:border-stone-400 transition-colors shadow-sm
                 opacity-0 group-hover:opacity-100 ${className}`}
    >
      {copied ? 'Copied!' : 'Copy'}
    </button>
  )
}
