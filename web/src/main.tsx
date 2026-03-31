import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { BrowserRouter, Routes, Route } from 'react-router-dom'
import './index.css'
import App from './App.tsx'
import Docs from './Docs.tsx'

// 获取 base URL，如果是 GitHub Pages 的子路径部署
const basename = import.meta.env.BASE_URL || '/'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <BrowserRouter basename={basename}>
      <Routes>
        <Route path="/" element={<App />} />
        <Route path="/docs" element={<Docs />} />
      </Routes>
    </BrowserRouter>
  </StrictMode>,
)
