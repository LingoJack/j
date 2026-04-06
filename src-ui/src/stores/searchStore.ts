import { create } from 'zustand';
import type { SearchResult, CommandResult } from '../types';
import { searchAliases, executeCommand } from '../services/tauri';
import { isCommand } from '../types';

interface SearchState {
  /** 当前输入 */
  query: string;
  /** 搜索/列表结果 */
  results: SearchResult[];
  /** 当前选中的结果索引 */
  selectedIndex: number;
  /** 反馈消息 */
  feedback: string | null;
  /** 反馈类型 */
  feedbackType: 'success' | 'error' | null;
  /** 输出模式 */
  outputMode: 'list' | 'text' | 'empty';
  /** 文本输出内容 */
  textOutput: string;
  /** 设置输入 */
  setQuery: (query: string) => void;
  /** 搜索别名 */
  search: (query: string) => Promise<void>;
  /** 执行命令 */
  execute: (input: string) => Promise<CommandResult>;
  /** 设置选中索引 */
  setSelectedIndex: (index: number) => void;
  /** 移动选择 */
  moveSelection: (delta: number) => void;
  /** 清除反馈 */
  clearFeedback: () => void;
  /** 重置状态 */
  reset: () => void;
}

export const useSearchStore = create<SearchState>((set, get) => ({
  query: '',
  results: [],
  selectedIndex: 0,
  feedback: null,
  feedbackType: null,
  outputMode: 'empty',
  textOutput: '',

  setQuery: (query: string) => {
    set({ query, feedback: null, feedbackType: null });
    
    // 实时搜索：非命令模式时才搜索别名
    if (!isCommand(query)) {
      get().search(query);
    } else {
      // 命令模式：清空结果，等待用户按回车执行
      set({ results: [], outputMode: 'empty', textOutput: '' });
    }
  },

  search: async (query: string) => {
    try {
      const results = await searchAliases(query);
      set({ 
        results, 
        selectedIndex: 0,
        outputMode: results.length > 0 ? 'list' : 'empty',
        textOutput: '',
      });
    } catch {
      set({ results: [], outputMode: 'empty', textOutput: '' });
    }
  },

  execute: async (input: string) => {
    const result = await executeCommand(input);
    
    // 根据输出类型设置状态
    const outputType = result.output_type || 'simple';
    
    if (outputType === 'text' && result.raw_output) {
      // 文本输出模式
      set({
        results: [],
        selectedIndex: 0,
        feedback: result.message,
        feedbackType: result.success ? 'success' : 'error',
        outputMode: 'text',
        textOutput: result.raw_output,
      });
    } else if (result.results && result.results.length > 0) {
      // 列表输出模式
      set({
        results: result.results,
        selectedIndex: 0,
        feedback: result.message,
        feedbackType: result.success ? 'success' : 'error',
        outputMode: 'list',
        textOutput: '',
      });
    } else {
      // 简单消息模式
      set({
        feedback: result.message,
        feedbackType: result.success ? 'success' : 'error',
        results: [],
        outputMode: 'empty',
        textOutput: '',
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
    outputMode: 'empty',
    textOutput: '',
  }),
}));
