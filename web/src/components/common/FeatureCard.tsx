import type { FeatureItem } from '../../types'

export function FeatureCard({ icon, title, description }: FeatureItem) {
  return (
    <div className="p-6 bg-white rounded-lg border border-stone-200 hover:border-stone-300 transition-colors">
      <div className="text-2xl mb-3">{icon}</div>
      <h3 className="text-base font-medium text-stone-900 mb-2">{title}</h3>
      <p className="text-stone-600 text-sm leading-relaxed">{description}</p>
    </div>
  )
}
