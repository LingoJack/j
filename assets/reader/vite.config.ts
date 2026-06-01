import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

// `j read <file>` 命令使用的 Reader SPA 构建配置。
//
// 要点：
// - `base: './'`     —— 资源使用相对路径，方便嵌入到 Rust 二进制后由本地 server 任意挂载
// - `outDir: 'dist'` —— 产物落在本目录下，由 rust-embed (`src/command/read/embed.rs`) 编译时打包
// - 入口 HTML 使用 `reader.html`
export default defineConfig({
  plugins: [react(), tailwindcss()],
  base: './',
  // Reader SPA 不需要任何 public 资源，禁用 public 目录拷贝以保持产物精简。
  publicDir: false,
  resolve: {
    alias: [
      // 关键：把裸 `refractor` 包重定向到 `refractor/core`（empty kernel），
      // 避免把 ~280 种语言全打进 bundle。子路径 `refractor/<lang>` 不受影响
      // —— 用精确正则只匹配裸名，子路径仍走 package exports。
      { find: /^refractor$/, replacement: 'refractor/core' },
    ],
  },
  build: {
    outDir: 'dist',
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
          // CodeMirror 6 + refractor 整体走单独 chunk（便于浏览器缓存）
          if (
            id.includes('node_modules/@codemirror/') ||
            id.includes('node_modules/@lezer/') ||
            id.includes('node_modules/refractor/')
          ) {
            return 'editor-vendor'
          }
        },
      },
    },
    chunkSizeWarningLimit: 1000,
  },
})
