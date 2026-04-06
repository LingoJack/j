import { useCallback, useEffect } from 'react';
import { SearchBar } from './SearchBar';
import { useSearchStore } from '../../stores/searchStore';
import { hideWindow } from '../../services/tauri';

export function SpotlightWindow() {
  const { query, execute, reset } = useSearchStore();

  const handleKeyDown = useCallback(
    async (e: KeyboardEvent) => {
      switch (e.key) {
        case 'Enter':
          e.preventDefault();
          if (query.trim()) {
            await execute(query.trim());
            await hideWindow();
            reset();
          }
          break;
        case 'Escape':
          e.preventDefault();
          await hideWindow();
          reset();
          break;
      }
    },
    [query, execute, reset]
  );

  useEffect(() => {
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleKeyDown]);

  return (
    <div className="h-screen w-screen flex items-start justify-center pt-[15vh] px-4">
      <SearchBar />
    </div>
  );
}
