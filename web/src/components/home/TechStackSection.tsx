import { Section } from '../common/Section'
import type { I18nData } from '../../types'

export function TechStackSection({ t }: { t: I18nData }) {
  return (
    <Section>
      <div className="text-center">
        <h2 className="text-2xl font-light text-stone-900 mb-8">
          {t.tech.title}
        </h2>
        <div className="flex flex-wrap justify-center gap-3">
          {['clap', 'ratatui', 'async-openai', 'serde', 'tokio'].map((tech) => (
            <span 
              key={tech}
              className="px-4 py-1.5 text-sm text-stone-600 bg-white rounded-full border border-stone-200"
            >
              {tech}
            </span>
          ))}
        </div>
      </div>
    </Section>
  )
}
