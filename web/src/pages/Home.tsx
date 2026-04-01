import { useState } from 'react'
import { Nav } from '../components/home/Nav'
import { HeroSection } from '../components/home/HeroSection'
import { FeaturesSection } from '../components/home/FeaturesSection'
import { QuickStartSection } from '../components/home/QuickStartSection'
import { MoreFeaturesSection } from '../components/home/MoreFeaturesSection'
import { BestPracticesSection } from '../components/home/BestPracticesSection'
import { TechStackSection } from '../components/home/TechStackSection'
import { CTASection } from '../components/home/CTASection'
import { Footer } from '../components/home/Footer'
import { i18n } from '../data/i18n'
import type { Language } from '../types'

const installCmd = 'curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | bash'

export default function Home() {
  const [lang, setLang] = useState<Language>('zh')
  const t = i18n[lang]
  
  return (
    <div className="min-h-screen bg-[#faf9f6]">
      <Nav lang={lang} t={t} onLangChange={setLang} />
      <HeroSection t={t} installCmd={installCmd} />
      <FeaturesSection t={t} />
      <QuickStartSection t={t} installCmd={installCmd} />
      <MoreFeaturesSection t={t} />
      <BestPracticesSection lang={lang} t={t} />
      <TechStackSection t={t} />
      <CTASection t={t} installCmd={installCmd} />
      <Footer t={t} />
    </div>
  )
}
