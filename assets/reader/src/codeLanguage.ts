import { normalizeCodeLanguage } from './editor/code-highlight'

const EXTENSION_LANGUAGE: Record<string, string> = {
  bash: 'bash',
  c: 'c',
  cc: 'cpp',
  cpp: 'cpp',
  cs: 'csharp',
  css: 'css',
  go: 'go',
  h: 'c',
  hpp: 'cpp',
  htm: 'markup',
  html: 'markup',
  java: 'java',
  js: 'javascript',
  json: 'json',
  jsx: 'jsx',
  kt: 'kotlin',
  lua: 'lua',
  mjs: 'javascript',
  md: 'markdown',
  php: 'php',
  py: 'python',
  rb: 'ruby',
  rs: 'rust',
  sh: 'bash',
  sql: 'sql',
  swift: 'swift',
  toml: 'toml',
  ts: 'typescript',
  tsx: 'tsx',
  vue: 'markup',
  xml: 'markup',
  yaml: 'yaml',
  yml: 'yaml',
  zsh: 'bash',
}

const CODE_FILE_NAMES: Record<string, string> = {
  dockerfile: 'docker',
  makefile: 'makefile',
}

export function detectCodeLanguage(path: string): string {
  const fileName = path.split('/').pop()?.toLowerCase() ?? ''
  const exact = CODE_FILE_NAMES[fileName]
  if (exact) return normalizeCodeLanguage(exact)

  const ext = fileName.includes('.') ? fileName.split('.').pop() : undefined
  if (!ext) return ''

  return normalizeCodeLanguage(EXTENSION_LANGUAGE[ext] ?? '')
}

export function isCodeFile(path: string): boolean {
  return detectCodeLanguage(path).length > 0
}
