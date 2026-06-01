import { createRoot } from 'react-dom/client'
import './reader.css'
import './cm6/cm6.css'
import { Reader } from './Reader'

// 注意：刻意不开 StrictMode。
// StrictMode 会让所有 effect、ref 初始化跑两次，且把所有 state setter
// 跑两次以暴露并发渲染问题。Reader 是重度副作用 + 大量 ref 桥接 + 重型
// 第三方编辑器（Milkdown/ProseMirror）的页面，StrictMode 下首屏几乎慢
// 一倍，且每次切 tab 都付双倍 mount 代价。
// 这是浏览器里给最终用户看的页面，不是 dev 调试入口，关掉。
const root = document.getElementById('reader-root')
if (root) {
  createRoot(root).render(<Reader />)
}
