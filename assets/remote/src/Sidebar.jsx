import SidebarNav from './SidebarNav'
import SessionSection from './SessionSection'
import GitSection from './GitSection'
import RemoteSection from './RemoteSection'
import FilePreviewSection from './FilePreviewSection'

export default function Sidebar({
  activeSection,
  sidebarCollapsed,
  sessions,
  currentSessionId,
  theme,
  onSelectSection,
  onSwitchSession,
  onNewSession,
  onToggleCollapse,
  onToggleTheme,
}) {
  const renderSection = () => {
    switch (activeSection) {
      case 'sessions':
        return (
          <SessionSection
            sessions={sessions}
            currentSessionId={currentSessionId}
            onSwitch={onSwitchSession}
            onNew={onNewSession}
            onCollapse={onToggleCollapse}
          />
        )
      case 'git':
        return <GitSection onCollapse={onToggleCollapse} />
      case 'remote':
        return <RemoteSection onCollapse={onToggleCollapse} />
      case 'files':
        return <FilePreviewSection onCollapse={onToggleCollapse} />
      default:
        return null
    }
  }

  return (
    <div className="flex shrink-0 h-full">
      <SidebarNav
        activeSection={activeSection}
        onSelect={onSelectSection}
        theme={theme}
        toggleTheme={onToggleTheme}
      />
      <div className={`sidebar-section ${sidebarCollapsed ? 'collapsed' : ''}`}>
        {renderSection()}
      </div>
    </div>
  )
}
