interface SidebarProps {
  tree: Record<string, { title: string; children: Record<string, string> }>
  activeSection: string
  onNavigate: (section: string) => void
  isOpen: boolean
  onClose: () => void
}

export function Sidebar({ tree, activeSection, onNavigate, isOpen, onClose }: SidebarProps) {
  return (
    <>
      {/* Mobile overlay */}
      {isOpen && (
        <div 
          className="fixed inset-0 bg-black/20 z-40 lg:hidden"
          onClick={onClose}
        />
      )}
      
      {/* Sidebar */}
      <aside className={`
        fixed top-[65px] left-0 bottom-0 w-72 bg-[#faf9f6] border-r border-stone-200 
        overflow-y-auto z-50 transition-transform duration-300
        lg:translate-x-0
        ${isOpen ? 'translate-x-0' : '-translate-x-full'}
      `}>
        <nav className="p-6">
          {Object.entries(tree).map(([key, category]) => (
            <div key={key} className="mb-6">
              <h3 className="text-xs font-semibold text-stone-400 uppercase tracking-wider mb-3">
                {category.title}
              </h3>
              <ul className="space-y-1">
                {Object.entries(category.children).map(([childKey, childTitle]) => (
                  <li key={childKey}>
                    <button
                      onClick={() => {
                        onNavigate(childKey)
                        onClose()
                      }}
                      className={`
                        w-full text-left px-3 py-2 rounded-lg text-sm transition-colors
                        ${activeSection === childKey 
                          ? 'bg-stone-200 text-stone-900 font-medium' 
                          : 'text-stone-600 hover:bg-stone-100'
                        }
                      `}
                    >
                      {childTitle}
                    </button>
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </nav>
      </aside>
    </>
  )
}
