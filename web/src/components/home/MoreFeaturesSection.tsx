import { Section } from '../common/Section'
import type { I18nData } from '../../types'

export function MoreFeaturesSection({ t }: { t: I18nData }) {
  return (
    <Section className="bg-white border-y border-stone-200">
      <div className="text-center mb-12">
        <h2 className="text-3xl sm:text-4xl font-light text-stone-900 mb-4">
          {t.more.title}
        </h2>
      </div>
      
      <div className="grid sm:grid-cols-2 lg:grid-cols-3 gap-8">
        {t.more.list.map((item, i) => (
          <div key={i} className="text-center">
            <h3 className="font-medium text-stone-900 mb-2">{item.title}</h3>
            <p className="text-stone-500 text-sm">{item.desc}</p>
          </div>
        ))}
      </div>
    </Section>
  )
}
