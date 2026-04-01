import type { CommandExample as CommandExampleType } from '../../types'

export function CommandExample({ cmd, description }: CommandExampleType) {
  return (
    <div className="py-3 border-b border-stone-200 last:border-0">
      <code className="text-stone-700 font-mono text-sm block mb-1 break-all">
        {cmd}
      </code>
      <span className="text-stone-500 text-sm">{description}</span>
    </div>
  )
}
