interface SectionProps {
  id?: string
  children: React.ReactNode
  className?: string
}

export function Section({ id, children, className = '' }: SectionProps) {
  return (
    <section id={id} className={`py-16 md:py-24 px-6 ${className}`}>
      <div className="max-w-4xl mx-auto">
        {children}
      </div>
    </section>
  )
}
