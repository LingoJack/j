import { useState, useCallback, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface SearchResult {
  alias: string;
  description: string;
  category: string;
}

export function App() {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<SearchResult[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [isLoading, setIsLoading] = useState(false);
  const [showResults, setShowResults] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  // 搜索
  const performSearch = useCallback(async (q: string) => {
    if (!q.trim()) {
      setResults([]);
      setShowResults(false);
      return;
    }

    setIsLoading(true);
    try {
      const res = await invoke<SearchResult[]>('search_aliases', { query: q });
      setResults(res);
      setSelectedIndex(0);
      setShowResults(true);
    } catch (e) {
      console.error('Search failed:', e);
      setResults([]);
      setShowResults(false);
    } finally {
      setIsLoading(false);
    }
  }, []);

  // 执行命令
  const executeCommand = useCallback(async (input: string) => {
    try {
      await invoke('execute_command', { input });
      await invoke('hide_window');
      setQuery('');
      setResults([]);
      setShowResults(false);
    } catch (e) {
      console.error('Execute failed:', e);
    }
  }, []);

  // 键盘事件
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      switch (e.key) {
        case 'Escape':
          e.preventDefault();
          invoke('hide_window');
          setQuery('');
          setResults([]);
          setShowResults(false);
          break;
        case 'Enter':
          e.preventDefault();
          if (results.length > 0) {
            executeCommand(results[selectedIndex].alias);
          } else if (query.trim()) {
            executeCommand(query.trim());
          }
          break;
        case 'ArrowUp':
          e.preventDefault();
          setSelectedIndex(i => Math.max(0, i - 1));
          break;
        case 'ArrowDown':
          e.preventDefault();
          setSelectedIndex(i => Math.min(results.length - 1, i + 1));
          break;
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [results, selectedIndex, query, executeCommand]);

  // 输入变化时搜索
  useEffect(() => {
    const timer = setTimeout(() => {
      performSearch(query);
    }, 150);

    return () => clearTimeout(timer);
  }, [query, performSearch]);

  // 聚焦输入框
  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  return (
    <div className="spotlight-container">
      <div className="spotlight-window">
        <div className="search-box">
          <span className="search-icon">j</span>
          <input
            ref={inputRef}
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="输入命令或搜索..."
            className="search-input"
            spellCheck={false}
          />
          {isLoading && <span className="loading-indicator" />}
        </div>

        {showResults && results.length > 0 && (
          <div className="result-list">
            {results.map((result, index) => (
              <div
                key={result.alias}
                className={`result-item ${index === selectedIndex ? 'selected' : ''}`}
                onClick={() => executeCommand(result.alias)}
              >
                <span className="result-alias">{result.alias}</span>
                <span className="result-description">{result.description}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
