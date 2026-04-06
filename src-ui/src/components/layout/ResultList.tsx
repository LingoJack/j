import { useSearchStore } from '../../stores/searchStore';
import { openAlias, hideWindow } from '../../services/tauri';

const KIND_ICONS: Record<string, string> = {
  app: '📁',
  url: '🔗',
  outer_url: '🌐',
  script: '⚙️',
  editor: '📝',
  browser: '🌍',
  vpn: '🔒',
};

export function ResultList() {
  const { results, selectedIndex, setSelectedIndex } = useSearchStore();

  const handleOpen = async (alias: string) => {
    try {
      await openAlias(alias);
    } catch (e) {
      console.error('打开失败:', e);
    }
    await hideWindow();
  };

  return (
    <div className="border-t border-white/[0.06]">
      <div className="py-1 max-h-[280px] overflow-y-auto result-scroll">
        {results.map((item, index) => (
          <div
            key={`${item.kind}-${item.alias}`}
            className={`flex items-center px-4 py-[7px] cursor-default transition-colors duration-75 ${
              index === selectedIndex
                ? 'bg-primary/25'
                : 'hover:bg-white/[0.04]'
            }`}
            onClick={() => handleOpen(item.alias)}
            onMouseEnter={() => setSelectedIndex(index)}
          >
            <span className="mr-3 text-sm opacity-70">
              {KIND_ICONS[item.kind] || '📁'}
            </span>
            <div className="flex-1 min-w-0">
              <span className="text-[13px] font-medium text-text truncate block">
                {item.alias}
              </span>
            </div>
            <span className="text-[11px] text-text-secondary/50 ml-3 shrink-0">
              {item.path.length > 40 ? item.path.slice(0, 37) + '...' : item.path}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}
