import { Section } from '../common/Section'
import { FeatureCard } from '../common/FeatureCard'
import type { I18nData } from '../../types'

export function FeaturesSection({ t }: { t: I18nData }) {
  return (
    <Section id="features" className="bg-white border-y border-stone-200">
      <div className="mb-12">
        <h2 className="text-3xl sm:text-4xl font-light text-stone-900 mb-4">
          {t.features.title}
        </h2>
        <p className="text-stone-500 max-w-lg">
          {t.features.subtitle}
        </p>
      </div>
      
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
        {t.features.list.map((feature, index) => (
          <FeatureCard key={index} {...feature} />
        ))}
      </div>
    </Section>
  )
}
