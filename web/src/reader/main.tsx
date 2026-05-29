import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './reader.css'
import './milkdown/seeyue.css'
import { Reader } from './Reader'

const root = document.getElementById('reader-root')
if (root) {
  createRoot(root).render(
    <StrictMode>
      <Reader />
    </StrictMode>,
  )
}
