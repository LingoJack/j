import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import '../index.css'
import { Reader } from './Reader'

const root = document.getElementById('reader-root')
if (root) {
  createRoot(root).render(
    <StrictMode>
      <Reader />
    </StrictMode>,
  )
}
