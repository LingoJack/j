import { useState } from 'react'
import { Link } from 'react-router-dom'
import { LanguageSwitcher } from '../common/LanguageSwitcher'
import type { Language, I18nData } from '../../types'

interface NavProps {
  lang: Language
  t: I18nData
  onLangChange: (lang: Language) => void
}

export function Nav({ lang, t, onLangChange }: NavProps) {
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false)
  
  return (
    <nav className="fixed top-0 left-0 right-0 z-50 bg-[#faf9f6]/90 backdrop-blur-sm border-b border-stone-200/50">
      <div className="max-w-4xl mx-auto px-6 py-4 flex items-center justify-between">
        <a href="#" className="flex items-center gap-2">
          <span className="text-2xl font-bold text-stone-900">j</span>
          <span className="text-stone-400 text-sm hidden sm:inline">Productivity CLI</span>
        </a>
        
        {/* Desktop Navigation */}
        <div className="hidden md:flex items-center gap-5">
          <LanguageSwitcher lang={lang} onChange={onLangChange} />
          <a href="#features" className="text-stone-500 hover:text-stone-900 transition-colors text-sm whitespace-nowrap">
            {t.nav.features}
          </a>
          <Link to="/docs" className="text-stone-500 hover:text-stone-900 transition-colors text-sm whitespace-nowrap">
            {lang === 'en' ? 'Docs' : '文档'}
          </Link>
          <a href="#quick-start" className="text-stone-500 hover:text-stone-900 transition-colors text-sm whitespace-nowrap">
            {t.nav.quickStart}
          </a>
          <a 
            href="https://github.com/LingoJack/j" 
            target="_blank" 
            rel="noopener noreferrer"
            className="flex items-center gap-2 text-stone-500 hover:text-stone-900 transition-colors"
          >
            <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 24 24">
              <path fillRule="evenodd" clipRule="evenodd" d="M12 2C6.477 2 2 6.477 2 12c0 4.42 2.87 8.17 6.84 9.5.5.08.66-.23.66-.5v-1.69c-2.77.6-3.36-1.34-3.36-1.34-.46-1.16-1.11-1.47-1.11-1.47-.91-.62.07-.6.07-.6 1 .07 1.53 1.03 1.53 1.03.87 1.52 2.34 1.07 2.91.83.09-.65.35-1.09.63-1.34-2.22-.25-4.55-1.11-4.55-4.92 0-1.11.38-2 1.03-2.71-.1-.25-.45-1.29.1-2.64 0 0 .84-.27 2.75 1.02.79-.22 1.65-.33 2.5-.33.85 0 1.71.11 2.5.33 1.91-1.29 2.75-1.02 2.75-1.02.55 1.35.2 2.39.1 2.64.65.71 1.03 1.6 1.03 2.71 0 3.82-2.34 4.66-4.57 4.91.36.31.69.92.69 1.85v2.74c0 .27.16.59.67.5C19.14 20.16 22 16.42 22 12A10 10 0 0012 2z"/>
            </svg>
            <span className="text-sm">{t.nav.github}</span>
          </a>
        </div>

        {/* Mobile Menu Button */}
        <div className="flex md:hidden items-center gap-3">
          <LanguageSwitcher lang={lang} onChange={onLangChange} />
          
          {/* Hamburger Button */}
          <button 
            onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
            className="text-stone-500 hover:text-stone-900 transition-colors p-1"
          >
            <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2}>
              {mobileMenuOpen ? (
                <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
              ) : (
                <path strokeLinecap="round" strokeLinejoin="round" d="M4 6h16M4 12h16M4 18h16" />
              )}
            </svg>
          </button>
        </div>
      </div>

      {/* Mobile Menu */}
      {mobileMenuOpen && (
        <div className="md:hidden bg-[#faf9f6] border-t border-stone-200 px-6 py-4 space-y-3">
          <a 
            href="#features" 
            onClick={() => setMobileMenuOpen(false)}
            className="block text-stone-500 hover:text-stone-900 transition-colors text-sm py-2"
          >
            {t.nav.features}
          </a>
          <Link 
            to="/docs" 
            onClick={() => setMobileMenuOpen(false)}
            className="block text-stone-500 hover:text-stone-900 transition-colors text-sm py-2"
          >
            {lang === 'en' ? 'Docs' : '文档'}
          </Link>
          <a 
            href="#quick-start" 
            onClick={() => setMobileMenuOpen(false)}
            className="block text-stone-500 hover:text-stone-900 transition-colors text-sm py-2"
          >
            {t.nav.quickStart}
          </a>
          <a 
            href="https://github.com/LingoJack/j" 
            target="_blank" 
            rel="noopener noreferrer"
            className="flex items-center gap-2 text-stone-500 hover:text-stone-900 transition-colors text-sm py-2"
          >
            <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 24 24">
              <path fillRule="evenodd" clipRule="evenodd" d="M12 2C6.477 2 2 6.477 2 12c0 4.42 2.87 8.17 6.84 9.5.5.08.66-.23.66-.5v-1.69c-2.77.6-3.36-1.34-3.36-1.34-.46-1.16-1.11-1.47-1.11-1.47-.91-.62.07-.6.07-.6 1 .07 1.53 1.03 1.53 1.03.87 1.52 2.34 1.07 2.91.83.09-.65.35-1.09.63-1.34-2.22-.25-4.55-1.11-4.55-4.92 0-1.11.38-2 1.03-2.71-.1-.25-.45-1.29.1-2.64 0 0 .84-.27 2.75 1.02.79-.22 1.65-.33 2.5-.33.85 0 1.71.11 2.5.33 1.91-1.29 2.75-1.02 2.75-1.02.55 1.35.2 2.39.1 2.64.65.71 1.03 1.6 1.03 2.71 0 3.82-2.34 4.66-4.57 4.91.36.31.69.92.69 1.85v2.74c0 .27.16.59.67.5C19.14 20.16 22 16.42 22 12A10 10 0 0012 2z"/>
            </svg>
            <span>{t.nav.github}</span>
          </a>
        </div>
      )}
    </nav>
  )
}
