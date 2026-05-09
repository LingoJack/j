import SidebarNav from './SidebarNav'
import SessionSection from './SessionSection'
import ConfigSection from './ConfigSection'
import ArchiveSection from './ArchiveSection'
import FileSection from './FileSection'
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
      case 'browser':
        // 这些使用全屏面板，侧边栏只显示占位提示
        return (
          <div className="flex flex-col items-center justify-center h-full text-fg3 px-4 text-center">
            <div className="text-[12px]">使用主内容区</div>
            <button
              className="mt-2 text-accent text-[11px] hover:underline active:scale-[0.92] transition-transform duration-100 select-none"
              onClick={onToggleCollapse}
            >收起侧边栏</button>
          </div>
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
