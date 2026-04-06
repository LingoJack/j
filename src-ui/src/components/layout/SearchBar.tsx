import { useRef, useEffect } from 'react';
import { useSearchStore } from '../../stores/searchStore';

export function SearchBar() {
  const inputRef = useRef<HTMLInputElement>(null);
  const { query, setQuery } = useSearchStore();

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  return (
    <div className="flex items-center px-4 py-3 drag-region">
      <span className="text-text-secondary/60 mr-3 text-lg no-drag">j</span>
      <input
        ref={inputRef}
        type="text"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder="输入命令..."
        className="flex-1 bg-transparent text-text text-[15px] outline-none placeholder:text-text-secondary/40 no-drag font-mono"
        autoFocus
        spellCheck={false}
      />
    </div>
  );
}
