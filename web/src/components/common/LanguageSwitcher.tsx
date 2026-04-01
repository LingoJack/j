import { useState } from 'react'
import type { Language } from '../../types'

interface LanguageSwitcherProps {
  lang: Language
  onChange: (lang: Language) => void
  className?: string
}

export function LanguageSwitcher({ lang, onChange, className = '' }: LanguageSwitcherProps) {
  const [isOpen, setIsOpen] = useState(false)
  
  return (
    <div className={`relative ${className}`}>
      <button 
        onClick={() => setIsOpen(!isOpen)}
        onBlur={() => setTimeout(() => setIsOpen(false), 150)}
        className="text-stone-500 hover:text-stone-900 transition-colors text-sm flex items-center gap-0.5 whitespace-nowrap"
      >
        {lang === 'en' ? 'EN' : '中文'}
        <svg className={`w-3 h-3 ml-0.5 transition-transform ${isOpen ? 'rotate-180' : ''}`} fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2}>
          <path strokeLinecap="round" strokeLinejoin="round" d="M19 9l-7 7-7-7" />
        </svg>
      </button>
      {isOpen && (
        <div className="absolute top-full left-0 mt-1 bg-white rounded shadow-lg py-1 z-50 min-w-[60px]">
          <button
            onClick={() => { onChange('en'); setIsOpen(false); }}
            className={`block w-full text-left px-3 py-1.5 text-sm hover:bg-stone-50 whitespace-nowrap ${lang === 'en' ? 'text-stone-900 font-medium' : 'text-stone-500'}`}
          >
            EN
          </button>
          <button
            onClick={() => { onChange('zh'); setIsOpen(false); }}
            className={`block w-full text-left px-3 py-1.5 text-sm hover:bg-stone-50 whitespace-nowrap ${lang === 'zh' ? 'text-stone-900 font-medium' : 'text-stone-500'}`}
          >
            中文
          </button>
        </div>
      )}
    </div>
  )
}
