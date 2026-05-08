import type { I18nData } from '../../types'

export function Footer({ t }: { t: I18nData }) {
  return (
    <footer className="border-t border-stone-200 py-8 px-6 bg-[#faf9f6]">
      <div className="max-w-4xl mx-auto flex flex-col sm:flex-row items-center justify-between gap-4">
        <div className="flex items-center gap-2">
          <span className="text-lg font-bold text-stone-900">j</span>
          <span className="text-stone-400 text-sm">
            {t.footer.license}
          </span>
        </div>
        <div className="flex items-center gap-6">
          <a 
            href="https://github.com/LingoJack/jcli" 
            target="_blank" 
            rel="noopener noreferrer"
            className="text-stone-400 hover:text-stone-900 transition-colors text-sm"
          >
            GitHub
          </a>
          <a 
            href="https://crates.io/crates/j-cli" 
            target="_blank" 
            rel="noopener noreferrer"
            className="text-stone-400 hover:text-stone-900 transition-colors text-sm"
          >
            crates.io
          </a>
        </div>
      </div>
    </footer>
  )
}
