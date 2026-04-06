import { invoke } from '@tauri-apps/api/core';
import type { SearchResult, CommandResult } from '../types';

/** 搜索别名 */
export async function searchAliases(query: string): Promise<SearchResult[]> {
  return invoke('search_aliases', { query });
}

/** 打开别名 */
export async function openAlias(alias: string, args: string[] = []): Promise<string> {
  return invoke('open_alias', { alias, args });
}

/** 执行命令（与 CLI 语法一致） */
export async function executeCommand(input: string): Promise<CommandResult> {
  return invoke('execute_command', { input });
}

/** 隐藏窗口 */
export async function hideWindow(): Promise<void> {
  return invoke('hide_window');
}
