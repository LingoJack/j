import { create } from 'zustand';
import type { SearchResult, CommandResult } from '../types';
import { searchAliases, executeCommand } from '../services/tauri';

interface SearchState {
  query: string;
  results: SearchResult[];
  selectedIndex: number;
  feedback: string | null;
  feedbackType: 'success' | 'error' | null;
  setQuery: (query: string) => void;
  search: (query: string) => Promise<void>;
  execute: (input: string) => Promise<CommandResult>;
  setSelectedIndex: (index: number) => void;
  moveSelection: (delta: number) => void;
  clearFeedback: () => void;
  reset: () => void;
}

/** 判断输入是否为命令（以已知命令关键字开头） */
const COMMAND_PREFIXES = [
  'set ', 's ', 'remove ', 'rm ', 'rename ', 'rn ',
  'modify ', 'mf ', 'list', 'ls', 'version', 'v',
  'help', 'h',
];

function isCommand(input: string): boolean {
  const lower = input.toLowerCase().trim();
  return COMMAND_PREFIXES.some(p => lower === p.trim() || lower.startsWith(p));
}

export const useSearchStore = create<SearchState>((set, get) => ({
  query: '',
  results: [],
  selectedIndex: 0,
  feedback: null,
  feedbackType: null,

  setQuery: (query: string) => {
    set({ query, feedback: null, feedbackType: null });
    // 实时搜索：非命令模式时才搜索
    if (!isCommand(query)) {
      get().search(query);
    } else {
      set({ results: [] });
    }
  },

  search: async (query: string) => {
    try {
      const results = await searchAliases(query);
      set({ results, selectedIndex: 0 });
    } catch {
      set({ results: [] });
    }
  },

  execute: async (input: string) => {
    const result = await executeCommand(input);
    if (result.results && result.results.length > 0) {
      set({
        results: result.results,
        selectedIndex: 0,
        feedback: result.message,
        feedbackType: result.success ? 'success' : 'error',
      });
    } else {
      set({
        feedback: result.message,
        feedbackType: result.success ? 'success' : 'error',
        results: [],
      });
    }
    return result;
  },

  setSelectedIndex: (index: number) => set({ selectedIndex: index }),

  moveSelection: (delta: number) => {
    const { results, selectedIndex } = get();
    if (results.length === 0) return;
    const newIndex = (selectedIndex + delta + results.length) % results.length;
    set({ selectedIndex: newIndex });
  },

  clearFeedback: () => set({ feedback: null, feedbackType: null }),

  reset: () => set({
    query: '',
    results: [],
    selectedIndex: 0,
    feedback: null,
    feedbackType: null,
  }),
}));
