/**
 * 通用垂直分割条 —— 鼠标按下后跟随 X 拖动，回调 onResize(newWidth)。
 *
 * 设计要点：
 * - mousedown 时 capture 全局 mousemove / mouseup，移动期间禁用 user-select
 *   避免选中文字。
 * - 父级负责把 newWidth 写回 state + 持久化到 localStorage。
 * - 双击恢复默认宽度，方便误拖回不去。
 * - 走 React 渲染层，没有外部依赖。
 */
import { useCallback, useRef } from 'react'

interface Props {
  /** 当前宽度（受控） */
  width: number
  /** 允许的范围，超出会被夹紧 */
  min: number
  max: number
  /** 双击恢复 */
  defaultWidth: number
  onResize: (next: number) => void
  /** 视觉提示用：移动时 body 上加 col-resize 光标 */
  ariaLabel?: string
}

export function VerticalSplitter({
  width,
  min,
  max,
  defaultWidth,
  onResize,
  ariaLabel,
}: Props) {
  const startRef = useRef<{ startX: number; startW: number } | null>(null)

  const onPointerDown = useCallback(
    (e: React.PointerEvent<HTMLDivElement>) => {
      e.preventDefault()
      startRef.current = { startX: e.clientX, startW: width }
      // 加全局光标 + 禁用选区，移动期间体感更稳
      document.body.style.cursor = 'col-resize'
      document.body.style.userSelect = 'none'

      const onMove = (ev: PointerEvent) => {
        const s = startRef.current
        if (!s) return
        const next = Math.max(min, Math.min(max, s.startW + (ev.clientX - s.startX)))
        onResize(next)
      }
      const onUp = () => {
        startRef.current = null
        document.body.style.cursor = ''
        document.body.style.userSelect = ''
        window.removeEventListener('pointermove', onMove)
        window.removeEventListener('pointerup', onUp)
        window.removeEventListener('pointercancel', onUp)
      }
      window.addEventListener('pointermove', onMove)
      window.addEventListener('pointerup', onUp)
      window.addEventListener('pointercancel', onUp)
    },
    [width, min, max, onResize],
  )

  const onDoubleClick = useCallback(() => {
    onResize(defaultWidth)
  }, [defaultWidth, onResize])

  return (
    <div
      className="group relative w-[5px] h-full cursor-col-resize bg-transparent select-none touch-none before:content-[''] before:absolute before:top-0 before:bottom-0 before:left-0.5 before:w-px before:bg-seeyue-border before:transition-colors before:duration-150 hover:before:bg-seeyue-accent active:before:bg-seeyue-accent"
      role="separator"
      aria-orientation="vertical"
      aria-label={ariaLabel ?? '调节宽度'}
      onPointerDown={onPointerDown}
      onDoubleClick={onDoubleClick}
      title="拖动调节宽度（双击恢复默认）"
    >
      <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[3px] h-9 rounded-sm bg-transparent transition-colors duration-150 group-hover:bg-seeyue-accent group-active:bg-seeyue-accent" />
    </div>
  )
}
