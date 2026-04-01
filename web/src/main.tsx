import { StrictMode, lazy, Suspense } from 'react'
import { createRoot } from 'react-dom/client'
import { BrowserRouter, Routes, Route } from 'react-router-dom'
import './index.css'
import { Loading } from './components/common/Loading'

// Lazy load pages for better performance
// eslint-disable-next-line react-refresh/only-export-components
const Home = lazy(() => import('./pages/Home'))
// eslint-disable-next-line react-refresh/only-export-components
const Docs = lazy(() => import('./pages/Docs'))

// Get base URL for GitHub Pages subpath deployment
const basename = import.meta.env.BASE_URL || '/'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <BrowserRouter basename={basename}>
      <Suspense fallback={<Loading />}>
        <Routes>
          <Route path="/" element={<Home />} />
          <Route path="/docs" element={<Docs />} />
        </Routes>
      </Suspense>
    </BrowserRouter>
  </StrictMode>,
)
