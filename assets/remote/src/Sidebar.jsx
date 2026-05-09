import SidebarNav from './SidebarNav'
import SessionSection from './SessionSection'
import ConfigSection from './ConfigSection'
import ArchiveSection from './ArchiveSection'
import FileSection from './FileSection'
import TerminalSection from './TerminalSection'
import BrowserSection from './BrowserSection'
import HelpSection from './HelpSection'

export default function Sidebar({
  activeSection,
  sidebarCollapsed,
  sessions,
  currentSessionId,
  theme,
  configData,
  modelList,
  themeList,
  archives,
  fileEntries,
  fileContent,
  fileWriteResult,
  terminalHistory,
  send,
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
      case 'config':
        return (
          <ConfigSection
            configData={configData}
            modelList={modelList}
            themeList={themeList}
            send={send}
            onCollapse={onToggleCollapse}
          />
        )
      case 'archive':
        return (
          <ArchiveSection
            archives={archives}
            send={send}
            onCollapse={onToggleCollapse}
          />
        )
      case 'files':
        return (
          <FileSection
            fileEntries={fileEntries}
            fileContent={fileContent}
            fileWriteResult={fileWriteResult}
            send={send}
            onCollapse={onToggleCollapse}
          />
        )
      case 'terminal':
        return (
          <TerminalSection
            terminalHistory={terminalHistory}
            send={send}
            onCollapse={onToggleCollapse}
          />
        )
      case 'browser':
        return (
          <BrowserSection
            send={send}
            onCollapse={onToggleCollapse}
          />
        )
      case 'help':
        return <HelpSection onCollapse={onToggleCollapse} />
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
