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
          if (id.includes('node_modules/react-syntax-highlighter/')) {
            return 'syntax-highlight'
          }
        },
      },
    },
    chunkSizeWarningLimit: 1000,
  },
})
