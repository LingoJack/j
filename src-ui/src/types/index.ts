/** 搜索结果项 */
export interface SearchResult {
  alias: string;
  path: string;
  kind: 'app' | 'url' | 'outer_url' | 'script' | 'editor' | 'browser' | 'vpn';
}

/** 输出类型 */
export type OutputType = 'simple' | 'list' | 'text' | 'table';

/** 命令执行结果 */
export interface CommandResult {
  /** 是否成功 */
  success: boolean;
  /** 执行的命令类型 */
  command: string;
  /** 结果消息 */
  message: string;
  /** 搜索结果（如果是搜索/列表类命令） */
  results?: SearchResult[];
  /** 输出类型 */
  output_type?: OutputType;
  /** 原始文本输出（用于多行文本展示） */
  raw_output?: string;
}

/** 命令前缀映射（用于识别命令类型） */
export const COMMAND_PREFIXES = {
  // 别名管理
  set: ['set', 's'],
  remove: ['remove', 'rm'],
  rename: ['rename', 'rn'],
  modify: ['modify', 'mf'],
  note: ['note', 'nt'],
  denote: ['denote', 'dnt'],
  
  // 列表 & 搜索
  list: ['list', 'ls'],
  contain: ['contain', 'find'],
  
  // 日报系统
  report: ['report', 'r'],
  reportctl: ['reportctl', 'rctl'],
  check: ['check', 'c'],
  search: ['search', 'select', 'look', 'sch'],
  
  // 待办备忘
  todo: ['todo', 'td'],
  
  // AI 对话
  chat: ['chat', 'ai'],
  
  // 脚本
  concat: ['concat'],
  
  // 计时器
  time: ['time'],
  
  // 系统设置
  log: ['log'],
  change: ['change', 'chg'],
  
  // 更新
  update: ['update', 'up'],
  
  // 版本 & 帮助
  version: ['version', 'v'],
  help: ['help', 'h', '?'],
} as const;

/** 检查输入是否为命令（而非别名） */
export function isCommand(input: string): boolean {
  const firstWord = input.trim().split(/\s+/)[0]?.toLowerCase();
  if (!firstWord) return false;
  
  return Object.values(COMMAND_PREFIXES).some(
    (prefixes) => prefixes.includes(firstWord as never)
  );
}

/** 获取命令类型 */
export function getCommandType(input: string): string | null {
  const firstWord = input.trim().split(/\s+/)[0]?.toLowerCase();
  if (!firstWord) return null;
  
  for (const [type, prefixes] of Object.entries(COMMAND_PREFIXES)) {
    if (prefixes.includes(firstWord as never)) {
      return type;
    }
  }
  return null;
}
