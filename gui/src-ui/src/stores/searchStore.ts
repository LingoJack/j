import { create } from 'zustand';
import type { CommandResult } from '../types';
import { executeCommand } from '../services/tauri';

interface SearchState {
  /** 当前输入 */
  query: string;
  /** 设置输入 */
  setQuery: (query: string) => void;
  /** 执行命令 */
  execute: (input: string) => Promise<CommandResult>;
  /** 重置状态 */
  reset: () => void;
}

export const useSearchStore = create<SearchState>((set) => ({
  query: '',

  setQuery: (query: string) => {
    set({ query });
  },

  execute: async (input: string) => {
    const result = await executeCommand(input);
    return result;
  },

  reset: () => set({
    query: '',
  }),
}));
