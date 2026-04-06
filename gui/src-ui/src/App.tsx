import { useState, useCallback, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { LogicalSize } from '@tauri-apps/api/dpi';

interface SearchResult {
  alias: string;
  description: string;
  category: string;
}

interface CommandResult {
  success: boolean;
  command: string;
  message: string;
  results?: SearchResult[];
  output_type: string;
  raw_output?: string;
}

const WIDTH = 560;
const SEARCH_H = 72;
const ITEM_H = 44;
const MAX_ITEMS = 7;
const LIST_PAD = 16;

function calcHeight(itemCount: number, msgLines: number): number {
  let h = SEARCH_H;
  if (itemCount > 0) h += Math.min(itemCount, MAX_ITEMS) * ITEM_H + LIST_PAD;
  if (msgLines > 0) h += Math.min(msgLines, 12) * 22 + 40;
  return h;
}

async function resize(h: number) {
  try {
    await getCurrentWindow().setSize(new LogicalSize(WIDTH, h));
  } catch (_) {}
}

export function App() {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<SearchResult[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [isLoading, setIsLoading] = useState(false);
  const [cmdResult, setCmdResult] = useState<CommandResult | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  const performSearch = useCallback(async (q: string) => {
    if (!q.trim()) {
      setResults([]);
      setCmdResult(null);
      resize(SEARCH_H);
      return;
    }
    setIsLoading(true);
    try {
      const res = await invoke<SearchResult[]>('search_aliases', { query: q });
      setResults(res);
      setSelectedIndex(0);
      setCmdResult(null);
      resize(calcHeight(res.length, 0));
    } catch {
      setResults([]);
      resize(SEARCH_H);
    } finally {
      setIsLoading(false);
    }
  }, []);

  const executeCommand = useCallback(async (input: string) => {
    try {
      const result = await invoke<CommandResult>('execute_command', { input });
      if (result.command === 'open' && result.success) {
        await invoke('hide_window');
        setQuery('');
        setResults([]);
        setCmdResult(null);
        resize(SEARCH_H);
        return;
      }
      setCmdResult(result);
      if (result.results && result.results.length > 0) {
        setResults(result.results);
        resize(calcHeight(result.results.length, 1));
      } else {
        setResults([]);
        const lines = result.raw_output ? result.raw_output.split('\n').length : 1;
        resize(calcHeight(0, lines + 1));
      }
    } catch {}
  }, []);

  const resetState = useCallback(() => {
    setQuery('');
    setResults([]);
    setSelectedIndex(0);
    setCmdResult(null);
    resize(SEARCH_H);
  }, []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      switch (e.key) {
        case 'Escape':
          e.preventDefault();
          invoke('hide_window');
          resetState();
          break;
        case 'Enter':
          e.preventDefault();
          if (results.length > 0 && !cmdResult) {
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
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [results, selectedIndex, query, executeCommand, resetState, cmdResult]);

  useEffect(() => {
    const t = setTimeout(() => performSearch(query), 150);
    return () => clearTimeout(t);
  }, [query, performSearch]);

  useEffect(() => {
    inputRef.current?.focus();
    resize(SEARCH_H);
  }, []);

  useEffect(() => {
    const onFocus = () => inputRef.current?.focus();
    window.addEventListener('focus', onFocus);
    return () => window.removeEventListener('focus', onFocus);
  }, []);

  return (
    <div className="flex flex-col w-full overflow-hidden">
      {/* 搜索框 */}
      <div className="flex items-center h-[56px] px-5 gap-3">
        <span className="text-[#007AFF] text-lg font-semibold font-mono shrink-0">j</span>
        <input
          ref={inputRef}
          type="text"
          value={query}
          onChange={e => setQuery(e.target.value)}
          placeholder="输入命令或搜索..."
          className="flex-1 bg-transparent border-none outline-none text-base text-white placeholder-white/50 font-normal"
          spellCheck={false}
        />
        {isLoading && (
          <span className="w-4 h-4 rounded-full border-2 border-white/15 border-t-[#007AFF] animate-spin shrink-0" />
        )}
      </div>

      {/* 命令结果 */}
      {cmdResult && (
        <div className="px-5 py-3 border-t border-white/10">
          <p className={`text-sm leading-relaxed ${cmdResult.success ? 'text-white/85' : 'text-red-400'}`}>
            {cmdResult.message}
          </p>
          {cmdResult.raw_output && (
            <pre className="mt-2 text-xs font-mono text-white/70 leading-relaxed whitespace-pre-wrap break-all max-h-64 overflow-y-auto">
              {cmdResult.raw_output}
            </pre>
          )}
        </div>
      )}

      {/* 搜索结果列表 */}
      {results.length > 0 && (
        <div className="overflow-y-auto overflow-x-hidden p-2 border-t border-white/10">
          {results.map((result, index) => (
            <div
              key={result.alias}
              className={`flex items-center justify-between px-4 py-2.5 rounded-xl cursor-pointer transition-colors duration-150
                ${index === selectedIndex ? 'bg-[#007AFF]/20' : 'hover:bg-white/5'}`}
            >
              <span className="text-[15px] font-medium text-white font-mono">{result.alias}</span>
              <span className="text-[13px] text-white/50 max-w-[50%] truncate">{result.description}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
