import ReactDOM from 'react-dom/client';
import App from './App';
import './styles/globals.css';

// 生产环境不需要 StrictMode，开发模式下会导致双重渲染
ReactDOM.createRoot(document.getElementById('root')!).render(<App />);
