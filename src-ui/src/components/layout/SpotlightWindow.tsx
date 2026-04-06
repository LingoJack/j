import { useCallback, useEffect } from 'react';
import { SearchBar } from './SearchBar';
import { ResultList } from './ResultList';
import { useSearchStore } from '../../stores/searchStore';
import { openAlias, hideWindow } from '../../services/tauri';

export function SpotlightWindow() {
  const { results, selectedIndex, moveSelection, query, execute, feedback, feedbackType, reset } =
    useSearchStore();

  const handleKeyDown = useCallback(
    async (e: KeyboardEvent) => {
      switch (e.key) {
        case 'ArrowDown':
          e.preventDefault();
          moveSelection(1);
          break;
        case 'ArrowUp':
          e.preventDefault();
          moveSelection(-1);
          break;
        case 'Enter':
          e.preventDefault();
          if (results.length > 0 && results[selectedIndex]) {
            // 有搜索结果时：打开选中项
            try {
              await openAlias(results[selectedIndex].alias);
            } catch (err) {
              console.error('打开失败:', err);
            }
            await hideWindow();
            reset();
          } else if (query.trim()) {
            // 无搜索结果时：作为命令执行
            const result = await execute(query.trim());
            if (result.success && result.command === 'open') {
              await hideWindow();
              reset();
            }
          }
          break;
        case 'Escape':
          e.preventDefault();
          await hideWindow();
          reset();
          break;
      }
    },
    [results, selectedIndex, moveSelection, query, execute, reset]
  );

  useEffect(() => {
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleKeyDown]);

  return (
    <div className="glass rounded-xl overflow-hidden no-select">
      <SearchBar />
      {feedback && (
        <div
          className={`px-4 py-2 text-[13px] border-t border-white/[0.06] ${
            feedbackType === 'error' ? 'text-red-400' : 'text-green-400'
          }`}
        >
          {feedback}
        </div>
      )}
      {results.length > 0 && <ResultList />}
    </div>
  );
}
