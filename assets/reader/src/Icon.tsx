/**
 * Reader 内部 SVG icon 套件。
 *
 * 设计原则：
 * - 全部 SVG，currentColor 染色 → 由父级 className 控制颜色。
 * - 默认尺寸 16，能覆盖 14~20 的常见档位。
 * - 风格仿 lucide / Typora SeeYue：1.5px stroke、圆角端点。
 *
 * 这里集中管理是为了：1) FileTree / TabBar / TOC 用同一套；
 * 2) 后续替换图标（比如要换到 Tabler）时只改一个文件。
 */
import type { SVGProps } from 'react'

type IconBaseProps = {
  size?: number
  className?: string
} & Omit<SVGProps<SVGSVGElement>, 'width' | 'height'>

function svgProps({ size = 16, className, ...rest }: IconBaseProps) {
  return {
    width: size,
    height: size,
    viewBox: '0 0 24 24',
    fill: 'none',
    stroke: 'currentColor',
    strokeWidth: 1.6,
    strokeLinecap: 'round' as const,
    strokeLinejoin: 'round' as const,
    className,
    'aria-hidden': true,
    ...rest,
  }
}

export function ChevronRight(props: IconBaseProps) {
  return (
    <svg {...svgProps(props)}>
      <path d="m9 6 6 6-6 6" />
    </svg>
  )
}

export function ChevronDown(props: IconBaseProps) {
  return (
    <svg {...svgProps(props)}>
      <path d="m6 9 6 6 6-6" />
    </svg>
  )
}

export function ChevronLeft(props: IconBaseProps) {
  return (
    <svg {...svgProps(props)}>
      <path d="m15 6-6 6 6 6" />
    </svg>
  )
}

export function ChevronsLeft(props: IconBaseProps) {
  return (
    <svg {...svgProps(props)}>
      <path d="m11 17-5-5 5-5" />
      <path d="m18 17-5-5 5-5" />
    </svg>
  )
}

export function ChevronsRight(props: IconBaseProps) {
  return (
    <svg {...svgProps(props)}>
      <path d="m6 17 5-5-5-5" />
      <path d="m13 17 5-5-5-5" />
    </svg>
  )
}

/** 文件夹（折叠态） */
export function FolderClosed(props: IconBaseProps) {
  return (
    <svg {...svgProps(props)}>
      <path d="M3 7.2c0-1.12.9-2.02 2-2.02h4.5l1.8 2.02H19c1.1 0 2 .9 2 2.02v8.6c0 1.11-.9 2.02-2 2.02H5c-1.1 0-2-.9-2-2.02Z" />
    </svg>
  )
}

/** 文件夹（展开态：底部斜板） */
export function FolderOpen(props: IconBaseProps) {
  return (
    <svg {...svgProps(props)}>
      <path d="M3 19.8V7.2c0-1.12.9-2.02 2-2.02h4.5l1.8 2.02H19c1.1 0 2 .9 2 2.02v1.6" />
      <path d="M3 19.8h14.27a2 2 0 0 0 1.94-1.5l1.7-6.5a1 1 0 0 0-.97-1.25H6.84a2 2 0 0 0-1.94 1.5L3 19.8Z" />
    </svg>
  )
}

/** Markdown 文档 */
export function FileMd(props: IconBaseProps) {
  return (
    <svg {...svgProps(props)}>
      <path d="M14 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9Z" />
      <path d="M14 3v6h6" />
      <path d="M8 14v3" />
      <path d="M8 14l2 2 2-2" />
      <path d="M12 14v3" />
      <path d="M16 14v3" />
    </svg>
  )
}

/** 普通文本 */
export function FileText(props: IconBaseProps) {
  return (
    <svg {...svgProps(props)}>
      <path d="M14 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9Z" />
      <path d="M14 3v6h6" />
      <path d="M8 13h8" />
      <path d="M8 17h6" />
    </svg>
  )
}

/** 代码 */
export function FileCode(props: IconBaseProps) {
  return (
    <svg {...svgProps(props)}>
      <path d="M14 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9Z" />
      <path d="M14 3v6h6" />
      <path d="m10 13-2 2 2 2" />
      <path d="m14 13 2 2-2 2" />
    </svg>
  )
}

/** 图片 */
export function FileImage(props: IconBaseProps) {
  return (
    <svg {...svgProps(props)}>
      <path d="M14 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9Z" />
      <path d="M14 3v6h6" />
      <circle cx="10" cy="13" r="1.4" />
      <path d="m20 19-3-3-4 4" />
    </svg>
  )
}

/** 通用文件 */
export function FileGeneric(props: IconBaseProps) {
  return (
    <svg {...svgProps(props)}>
      <path d="M14 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9Z" />
      <path d="M14 3v6h6" />
    </svg>
  )
}

/** 新建文件：文件 icon + 加号 */
export function FilePlus(props: IconBaseProps) {
  return (
    <svg {...svgProps(props)}>
      <path d="M14 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9Z" />
      <path d="M14 3v6h6" />
      <path d="M12 13v6" />
      <path d="M9 16h6" />
    </svg>
  )
}

export function Search(props: IconBaseProps) {
  return (
    <svg {...svgProps(props)}>
      <circle cx="11" cy="11" r="7" />
      <path d="m20 20-3.4-3.4" />
    </svg>
  )
}

export function Eye(props: IconBaseProps) {
  return (
    <svg {...svgProps(props)}>
      <path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12Z" />
      <circle cx="12" cy="12" r="3" />
    </svg>
  )
}

export function EyeOff(props: IconBaseProps) {
  return (
    <svg {...svgProps(props)}>
      <path d="M9.88 5.18A10 10 0 0 1 12 5c6.5 0 10 7 10 7a17.6 17.6 0 0 1-3.06 3.94" />
      <path d="M6.6 6.6A17.7 17.7 0 0 0 2 12s3.5 7 10 7c1.6 0 3-.36 4.2-.93" />
      <path d="M9.9 9.9a3 3 0 0 0 4.2 4.2" />
      <path d="m3 3 18 18" />
    </svg>
  )
}

export function Close(props: IconBaseProps) {
  return (
    <svg {...svgProps(props)}>
      <path d="m6 6 12 12" />
      <path d="m18 6-12 12" />
    </svg>
  )
}

export function Save(props: IconBaseProps) {
  return (
    <svg {...svgProps(props)}>
      <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2Z" />
      <path d="M17 21v-7H7v7" />
      <path d="M7 3v4h8" />
    </svg>
  )
}

export function Copy(props: IconBaseProps) {
  return (
    <svg {...svgProps(props)}>
      <rect x="9" y="9" width="11" height="11" rx="2" />
      <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
    </svg>
  )
}

export function FolderRoot(props: IconBaseProps) {
  return (
    <svg {...svgProps(props)}>
      <path d="M3 7.2c0-1.12.9-2.02 2-2.02h4.5l1.8 2.02H19c1.1 0 2 .9 2 2.02v8.6c0 1.11-.9 2.02-2 2.02H5c-1.1 0-2-.9-2-2.02Z" />
      <circle cx="12" cy="13" r="1.5" />
    </svg>
  )
}

export function ListTree(props: IconBaseProps) {
  return (
    <svg {...svgProps(props)}>
      <path d="M21 12h-8" />
      <path d="M21 6H8" />
      <path d="M21 18h-8" />
      <path d="M3 6v.01" />
      <path d="M3 12v.01" />
      <path d="M3 18v.01" />
    </svg>
  )
}

export function Files(props: IconBaseProps) {
  return (
    <svg {...svgProps(props)}>
      <path d="M15.5 2H8.6c-.97 0-1.6.81-1.6 1.79v13.42c0 .98.63 1.79 1.6 1.79h9.8c.97 0 1.6-.81 1.6-1.79V6.5Z" />
      <path d="M15 2v5h5" />
      <path d="M4 7v13.21c0 .98.63 1.79 1.6 1.79h9.8" />
    </svg>
  )
}

export function AlertTriangle(props: IconBaseProps) {
  return (
    <svg {...svgProps(props)}>
      <path d="M10.3 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0Z" />
      <path d="M12 9v4" />
      <path d="M12 17h.01" />
    </svg>
  )
}

export function CheckCircle(props: IconBaseProps) {
  return (
    <svg {...svgProps(props)}>
      <circle cx="12" cy="12" r="9" />
      <path d="m9 12 2 2 4-4" />
    </svg>
  )
}

export function Info(props: IconBaseProps) {
  return (
    <svg {...svgProps(props)}>
      <circle cx="12" cy="12" r="9" />
      <path d="M12 8h.01" />
      <path d="M11 12h1v4h1" />
    </svg>
  )
}

export function Pencil(props: IconBaseProps) {
  return (
    <svg {...svgProps(props)}>
      <path d="M12 20h9" />
      <path d="M16.5 3.5a2.12 2.12 0 1 1 3 3L7 19l-4 1 1-4Z" />
    </svg>
  )
}

export function Power(props: IconBaseProps) {
  return (
    <svg {...svgProps(props)}>
      <path d="M12 2v10" />
      <path d="M18.36 6.64a9 9 0 1 1-12.73 0" />
    </svg>
  )
}

export function BookOpen(props: IconBaseProps) {
  return (
    <svg {...svgProps(props)}>
      <path d="M2 4.5C2 3.67 2.67 3 3.5 3h6A2.5 2.5 0 0 1 12 5.5v15A1.5 1.5 0 0 0 10.5 19h-7A1.5 1.5 0 0 1 2 17.5Z" />
      <path d="M22 4.5c0-.83-.67-1.5-1.5-1.5h-6A2.5 2.5 0 0 0 12 5.5v15a1.5 1.5 0 0 1 1.5-1.5h7a1.5 1.5 0 0 0 1.5-1.5Z" />
    </svg>
  )
}

export function Sparkles(props: IconBaseProps) {
  return (
    <svg {...svgProps(props)}>
      <path d="M12 3v3" />
      <path d="M12 18v3" />
      <path d="M4 12H1" />
      <path d="M23 12h-3" />
      <path d="m6 6 1.8 1.8" />
      <path d="m16.2 16.2 1.8 1.8" />
      <path d="m6 18 1.8-1.8" />
      <path d="m16.2 7.8 1.8-1.8" />
      <circle cx="12" cy="12" r="3" />
    </svg>
  )
}

/** 工具箱（活动栏 toolbox tab）：lucide toolbox 风 —— 一个带把手的箱子 */
export function Toolbox(props: IconBaseProps) {
  return (
    <svg {...svgProps(props)}>
      {/* 把手 */}
      <path d="M8 8V6a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
      {/* 箱体 */}
      <rect x="3" y="8" width="18" height="13" rx="2" />
      {/* 中线 + 锁扣 */}
      <path d="M3 14h18" />
      <path d="M10 14v2" />
      <path d="M14 14v2" />
    </svg>
  )
}

/**
 * 文本 diff 工具图标：lucide git-compare 风。
 * 左上 → 右下 两个分支节点 + 拐角箭头，比之前的"双圆 + 直线"更有"对比/分支"语义。
 */
export function GitCompare(props: IconBaseProps) {
  return (
    <svg {...svgProps(props)}>
      <circle cx="6" cy="6" r="2.6" />
      <circle cx="18" cy="18" r="2.6" />
      <path d="M11 6h4a3 3 0 0 1 3 3v6.4" />
      <path d="m15 13 3 3-3 3" />
      <path d="M13 18H9a3 3 0 0 1-3-3V8.6" />
      <path d="m9 11-3-3 3-3" />
    </svg>
  )
}

/**
 * JSON 工具图标：一对花括号 { }，象征 JSON 的字面量边界。
 * lucide braces 风。
 */
export function Braces(props: IconBaseProps) {
  return (
    <svg {...svgProps(props)}>
      <path d="M8 3H7a2 2 0 0 0-2 2v5a2 2 0 0 1-2 2 2 2 0 0 1 2 2v5a2 2 0 0 0 2 2h1" />
      <path d="M16 21h1a2 2 0 0 0 2-2v-5a2 2 0 0 1 2-2 2 2 0 0 1-2-2V5a2 2 0 0 0-2-2h-1" />
    </svg>
  )
}

/** 展开 / 折叠：圆里加号 / 减号，给 JSON 树用 */
export function PlusSquare(props: IconBaseProps) {
  return (
    <svg {...svgProps(props)}>
      <rect x="3" y="3" width="18" height="18" rx="2" />
      <path d="M12 8v8" />
      <path d="M8 12h8" />
    </svg>
  )
}

export function MinusSquare(props: IconBaseProps) {
  return (
    <svg {...svgProps(props)}>
      <rect x="3" y="3" width="18" height="18" rx="2" />
      <path d="M8 12h8" />
    </svg>
  )
}

/**
 * 根据文件名扩展名挑选合适的文件 icon。
 * - md / markdown / mdx → FileMd（蓝）
 * - txt / log → FileText
 * - 常见代码 → FileCode
 * - 常见图片 → FileImage
 * - 其它 → FileGeneric
 */
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
