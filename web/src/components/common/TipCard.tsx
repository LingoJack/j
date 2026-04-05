import type { TipItem } from '../../types'

export function TipCard({ title, desc, example }: TipItem) {
  return (
    <div className="p-4 rounded-lg">
      <h4 className="font-medium text-stone-900 mb-1">{title}</h4>
      <p className="text-stone-500 text-sm mb-2 leading-relaxed">{desc}</p>
      <code className="text-stone-500 text-xs font-mono">{example}</code>
    </div>
  )
}
