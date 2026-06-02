import { useEffect, useState } from 'react'
import type { ImagePayload } from './types'

interface Props {
  /** 图片绝对路径（已规范化），用作 /api/asset?path= 参数 */
  path: string
  /** 文件名（仅用于 alt） */
  filename: string
  /** 服务端 render_file 返回的元数据（mime / size） */
  payload: ImagePayload | null
}

/**
 * 只读图片查看器。
 *
 * - 通过 `/api/asset?path=...` 拉原始字节，浏览器自带解码 / 缩放。
 * - object-fit: contain 在容器内等比例适配，不裁剪、不溢出。
 * - 底部 status bar 展示尺寸（自然分辨率）+ 文件大小 + MIME。
 * - 不支持编辑、不显示 dirty / save —— 顶部 EditorBar 仍由 Reader 给出，但保存按钮
 *   按下没意义（PlainTextEditor 同样只是不报错地忽略），未来可在 EditorBar 隐藏。
 */
export function ImageViewer({ path, filename, payload }: Props) {
  const src = `./api/asset?path=${encodeURIComponent(path)}`
  const [meta, setMeta] = useState<{ w: number; h: number } | null>(null)
  const [error, setError] = useState<string | null>(null)

  // 切到新文件时重置 meta，避免显示旧图的尺寸
  useEffect(() => {
    setMeta(null)
    setError(null)
  }, [path])

  return (
    <div className="h-full flex flex-col bg-seeyue-bg">
      <div className="flex-1 min-h-0 flex items-center justify-center overflow-auto p-6 bg-[linear-gradient(45deg,rgba(236,239,244,0.054)_25%,transparent_25%)_0_0/16px_16px,linear-gradient(-45deg,rgba(236,239,244,0.054)_25%,transparent_25%)_0_0/16px_16px,linear-gradient(45deg,transparent_75%,rgba(236,239,244,0.054)_75%)_8px_8px/16px_16px,linear-gradient(-45deg,transparent_75%,rgba(236,239,244,0.054)_75%)_8px_8px/16px_16px]">
        {error ? (
          <div className="font-mono text-[13px] text-seeyue-danger">加载失败：{error}</div>
        ) : (
          <img
            src={src}
            alt={filename}
            draggable={false}
            onLoad={(e) => {
              const img = e.currentTarget
              setMeta({ w: img.naturalWidth, h: img.naturalHeight })
            }}
            onError={() => setError('无法加载图片')}
            className="max-w-full max-h-full object-contain select-none shadow-[0_6px_24px_rgba(0,0,0,0.18)] rounded bg-seeyue-bg-deep"
          />
        )}
      </div>
      <div className="flex items-center gap-2 px-3.5 py-1.5 border-t border-seeyue-border text-xs text-seeyue-fg-dim font-mono bg-seeyue-bg">
        <span className="text-seeyue-fg font-medium max-w-[40%] overflow-hidden text-ellipsis whitespace-nowrap" title={path}>
          {filename}
        </span>
        <span className="opacity-50">·</span>
        <span>
          {meta ? `${meta.w} × ${meta.h}` : '解码中…'}
        </span>
        {payload && (
          <>
            <span className="opacity-50">·</span>
            <span>{formatBytes(payload.size)}</span>
            <span className="opacity-50">·</span>
            <span className="ml-auto opacity-80">{payload.mime}</span>
          </>
        )}
      </div>
    </div>
  )
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`
  return `${(n / (1024 * 1024)).toFixed(2)} MB`
}
