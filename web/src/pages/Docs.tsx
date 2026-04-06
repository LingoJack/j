import { useState, useEffect } from 'react'
import { Link, useSearchParams } from 'react-router-dom'
import { Sidebar } from '../components/docs/Sidebar'
import { Markdown } from '../components/docs/Markdown'
import { PageNav } from '../components/docs/PageNav'
import { TOC } from '../components/docs/TOC'
import { LanguageSwitcher } from '../components/common/LanguageSwitcher'
import { docTree, docNavI18n, getDocContent, getSectionTitle, defaultSection } from '../data/docs'
import type { Language } from '../types'

export default function Docs() {
  const [searchParams, setSearchParams] = useSearchParams()
  const [lang, setLang] = useState<Language>('zh')
  const [sidebarOpen, setSidebarOpen] = useState(false)
  
  // 从 URL 读取当前章节，默认为 installation
  const activeSection = searchParams.get('section') || defaultSection
  const t = docNavI18n[lang]
  const tree = docTree[lang]
  
  // 导航到指定章节
  const navigateToSection = (section: string) => {
    setSearchParams({ section })
  }
  
  // Scroll to section on navigate
  useEffect(() => {
    const element = document.getElementById(activeSection)
    if (element) {
      element.scrollIntoView({ behavior: 'smooth', block: 'start' })
    }
  }, [activeSection])
  
  // Render section content
  const renderSection = () => {
    const content = getDocContent(lang, activeSection)
    const title = getSectionTitle(lang, activeSection)
    
    if (!content) return null
    
    return (
      <div key={`${lang}-${activeSection}`} id={activeSection} className="py-8">
        <h1 className="text-3xl font-light text-stone-900 mb-6">{title}</h1>
        <Markdown content={content} />
      </div>
    )
  }
  
  return (
    <div className="min-h-screen bg-[#faf9f6] text-stone-800">
      {/* Navigation */}
      <nav className="fixed top-0 left-0 right-0 z-50 bg-[#faf9f6]/95 backdrop-blur-sm border-b border-stone-200/50">
        <div className="px-4 sm:px-6 py-4 flex items-center justify-between">
          <div className="flex items-center gap-3">
            {/* Mobile menu button */}
            <button 
              onClick={() => setSidebarOpen(!sidebarOpen)}
              className="lg:hidden p-2 -ml-2 text-stone-500 hover:text-stone-900 transition-colors"
            >
              <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2}>
                {sidebarOpen ? (
                  <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
                ) : (
                  <path strokeLinecap="round" strokeLinejoin="round" d="M4 6h16M4 12h16M4 18h16" />
                )}
              </svg>
            </button>
            
            <Link to="/" className="flex items-center gap-2">
              <span className="text-2xl font-bold text-stone-900">j</span>
              <span className="text-stone-400 text-sm hidden sm:inline">docs</span>
            </Link>
          </div>
          
          <div className="flex items-center gap-3 sm:gap-5">
            <Link 
              to="/" 
              className="text-stone-500 hover:text-stone-900 transition-colors text-sm hidden sm:inline"
            >
              {lang === 'zh' ? '首页' : 'Home'}
            </Link>
            <LanguageSwitcher lang={lang} onChange={setLang} />
            <a 
              href="https://github.com/LingoJack/j" 
              target="_blank" 
              rel="noopener noreferrer"
              className="flex items-center gap-2 text-stone-500 hover:text-stone-900 transition-colors"
            >
              <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 24 24">
                <path fillRule="evenodd" clipRule="evenodd" d="M12 2C6.477 2 2 6.477 2 12c0 4.42 2.87 8.17 6.84 9.5.5.08.66-.23.66-.5v-1.69c-2.77.6-3.36-1.34-3.36-1.34-.46-1.16-1.11-1.47-1.11-1.47-.91-.62.07-.6.07-.6 1 .07 1.53 1.03 1.53 1.03.87 1.52 2.34 1.07 2.91.83.09-.65.35-1.09.63-1.34-2.22-.25-4.55-1.11-4.55-4.92 0-1.11.38-2 1.03-2.71-.1-.25-.45-1.29.1-2.64 0 0 .84-.27 2.75 1.02.79-.22 1.65-.33 2.5-.33.85 0 1.71.11 2.5.33 1.91-1.29 2.75-1.02 2.75-1.02.55 1.35.2 2.39.1 2.64.65.71 1.03 1.6 1.03 2.71 0 3.82-2.34 4.66-4.57 4.91.36.31.69.92.69 1.85v2.74c0 .27.16.59.67.5C19.14 20.16 22 16.42 22 12A10 10 0 0012 2z"/>
              </svg>
              <span className="text-sm hidden sm:inline">{t.github}</span>
            </a>
          </div>
        </div>
      </nav>

      {/* Sidebar */}
      <Sidebar 
        tree={tree}
        activeSection={activeSection}
        onNavigate={navigateToSection}
        isOpen={sidebarOpen}
        onClose={() => setSidebarOpen(false)}
      />

      {/* Main Content */}
      <main className="lg:ml-64 xl:mr-52 pt-[65px]">
        <div className="max-w-3xl mx-auto px-6 pb-16">
          {renderSection()}
          <PageNav lang={lang} activeSection={activeSection} onNavigate={navigateToSection} />
        </div>
      </main>

      {/* TOC */}
      <TOC content={getDocContent(lang, activeSection) || ''} lang={lang} />

      {/* Footer */}
      <footer className="lg:ml-64 xl:mr-52 border-t border-stone-200 py-8 px-6 bg-[#faf9f6]">
        <div className="max-w-3xl mx-auto flex items-center justify-between text-sm">
          <Link to="/" className="text-stone-500 hover:text-stone-900 transition-colors">
            {t.back}
          </Link>
          <div className="flex items-center gap-6">
            <a 
              href="https://github.com/LingoJack/j" 
              target="_blank" 
              rel="noopener noreferrer"
              className="text-stone-500 hover:text-stone-900 transition-colors"
            >
              GitHub
            </a>
            <a 
              href="https://crates.io/crates/j-cli" 
              target="_blank" 
              rel="noopener noreferrer"
              className="text-stone-500 hover:text-stone-900 transition-colors"
            >
              crates.io
            </a>
          </div>
        </div>
      </footer>
    </div>
  )
}
