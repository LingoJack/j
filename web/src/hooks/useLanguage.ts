import { useState, useCallback } from 'react'
import type { Language } from '../types'

export function useLanguage(defaultLang: Language = 'zh') {
  const [lang, setLang] = useState<Language>(defaultLang)
  
  const toggleLang = useCallback(() => {
    setLang(prev => prev === 'en' ? 'zh' : 'en')
  }, [])
  
  return { lang, setLang, toggleLang }
}
