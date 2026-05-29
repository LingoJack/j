import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

// `j read <file>` 命令使用的 Reader SPA 构建配置。
//
// 与现有 `vite.config.ts` 的关键差异：
// - `base: './'`     —— 资源使用相对路径，方便嵌入到 Rust 二进制后由本地 server 任意挂载
// - `outDir: '../assets/reader_web'` —— 输出到 Rust assets 目录，由 rust-embed 编译时打包
// - 入口 HTML 使用 `reader.html`，与现有 docs 站 `index.html` 完全独立
export default defineConfig({
  plugins: [react(), tailwindcss()],
  base: './',
  // Reader SPA 不需要 docs 站的 public 资源（favicon、sitemap、pics 等），
  // 显式禁用 public 目录拷贝，保持产物精简。
  publicDir: false,
  resolve: {
    alias: [
      // 关键：把裸 `refractor` 包重定向到 `refractor/core`（empty kernel），
      // 避免 @milkdown/plugin-prism 的 `import { refractor } from "refractor"`
      // 把 ~280 种语言全打进 bundle。子路径 `refractor/<lang>` 不受影响
      // —— 用精确正则只匹配裸名，子路径仍走 package exports。
      { find: /^refractor$/, replacement: 'refractor/core' },
    ],
  },
  build: {
    outDir: '../assets/reader_web',
    emptyOutDir: true,
    rollupOptions: {
      input: 'reader.html',
      output: {
        manualChunks(id) {
          if (
            id.includes('node_modules/react/') ||
            id.includes('node_modules/react-dom/')
          ) {
            return 'react-vendor'
          }
          // Milkdown / ProseMirror / refractor 整体走单独 chunk（便于浏览器缓存）
          if (
            id.includes('node_modules/@milkdown/') ||
            id.includes('node_modules/prosemirror-') ||
            id.includes('node_modules/refractor/')
          ) {
            return 'milkdown-vendor'
          }
        },
      },
    },
    chunkSizeWarningLimit: 1000,
  },
})
