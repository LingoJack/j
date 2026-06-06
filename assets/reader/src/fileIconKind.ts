export type FileIconKind = 'markdown' | 'text' | 'code' | 'image' | 'generic'

const CODE_EXTS = new Set([
  'js',
  'jsx',
  'ts',
  'tsx',
  'mjs',
  'cjs',
  'json',
  'yml',
  'yaml',
  'toml',
  'rs',
  'go',
  'py',
  'rb',
  'java',
  'c',
  'h',
  'cc',
  'cpp',
  'hpp',
  'cs',
  'sh',
  'zsh',
  'bash',
  'fish',
  'ps1',
  'sql',
  'css',
  'scss',
  'sass',
  'less',
  'html',
  'xml',
  'svg',
  'vue',
  'svelte',
  'lua',
  'php',
  'kt',
  'swift',
])

const IMAGE_EXTS = new Set(['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp', 'ico', 'avif', 'tiff'])

const TEXT_EXTS = new Set(['txt', 'log', 'csv', 'tsv'])

/**
 * 根据文件名扩展名挑选合适的文件 icon。
 * - md / markdown / mdx → FileMd
 * - txt / log → FileText
 * - 常见代码 → FileCode
 * - 常见图片 → FileImage
 * - 其它 → FileGeneric
 */
export function pickFileIconKind(name: string): FileIconKind {
  const lower = name.toLowerCase()
  const dot = lower.lastIndexOf('.')
  const ext = dot >= 0 ? lower.slice(dot + 1) : ''
  if (ext === 'md' || ext === 'markdown' || ext === 'mdx') return 'markdown'
  if (TEXT_EXTS.has(ext)) return 'text'
  if (CODE_EXTS.has(ext)) return 'code'
  if (IMAGE_EXTS.has(ext)) return 'image'
  return 'generic'
}
