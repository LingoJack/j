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
        fixed top-[65px] left-0 bottom-0 w-64 bg-[#faf9f6] border-r border-stone-200/70
        overflow-y-auto z-50 transition-transform duration-300
        lg:translate-x-0 scrollbar-thin
        ${isOpen ? 'translate-x-0' : '-translate-x-full'}
      `}>
        <nav className="p-4">
          {Object.entries(tree).map(([key, category], index, arr) => (
            <div key={key} className={`mb-6 ${index < arr.length - 1 ? 'pb-6 border-b border-stone-200/50' : ''}`}>
              {/* Category title with decorative line */}
              <h3 className="flex items-center gap-2 text-xs font-semibold text-stone-400 uppercase tracking-wider mb-3 px-3">
                <span className="w-1 h-4 bg-stone-300 rounded-full flex-shrink-0" />
                {category.title}
              </h3>
              <ul className="space-y-0.5">
                {Object.entries(category.children).map(([childKey, childTitle]) => (
                  <li key={childKey}>
                    <button
                      onClick={() => {
                        onNavigate(childKey)
                        onClose()
                      }}
                      className={`
                        relative w-full text-left px-3 py-2 rounded-lg text-sm transition-all duration-200
                        ${activeSection === childKey 
                          ? 'text-stone-900 font-medium bg-stone-100 before:absolute before:left-0 before:top-1/2 before:-translate-y-1/2 before:w-0.5 before:h-5 before:bg-stone-900 before:rounded-full' 
                          : 'text-stone-500 hover:text-stone-700 hover:bg-stone-50'
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
