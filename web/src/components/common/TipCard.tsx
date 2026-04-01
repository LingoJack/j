import type { TipItem } from '../../types'

export function TipCard({ title, desc, example }: TipItem) {
  return (
    <div className="p-5 bg-white rounded-lg border border-stone-200 hover:border-stone-300 transition-colors">
      <h4 className="font-medium text-stone-900 mb-2">{title}</h4>
      <p className="text-stone-600 text-sm mb-3 leading-relaxed">{desc}</p>
      <div className="bg-[#faf9f6] rounded px-3 py-2 border border-stone-200 overflow-x-auto">
        <code className="text-stone-700 text-xs font-mono whitespace-pre-wrap break-all sm:whitespace-pre">{example}</code>
      </div>
    </div>
  )
}
