/** 搜索结果项 */
export interface SearchResult {
  alias: string;
  path: string;
  kind: 'app' | 'url' | 'outer_url' | 'script' | 'editor' | 'browser' | 'vpn';
}

/** 命令执行结果 */
export interface CommandResult {
  success: boolean;
  command: string;
  message: string;
  results?: SearchResult[];
}
