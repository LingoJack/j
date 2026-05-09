export function escHtml(s) {
  if (!s) return ''
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/\n/g, '<br>')
}

export function renderMd(text) {
  if (!text) return ''
  let html = escHtml(text)
  html = html.replace(/```(\w*)<br>([\s\S]*?)```/g, '<pre><code>$2</code></pre>')
  html = html.replace(/`([^`]+)`/g, '<code>$1</code>')
  html = html.replace(/\*\*([^*]+)\*\*/g, '<b>$1</b>')
  html = html.replace(/\*([^*]+)\*/g, '<i>$1</i>')
  html = html.replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2" target="_blank" style="color:var(--green)">$1</a>')
  return html
}

export function truncate(s, n) {
  return s && s.length > n ? s.substring(0, n) + '...' : (s || '')
}

export function formatRelativeTime(ts) {
  if (!ts) return ''
  const now = Math.floor(Date.now() / 1000)
  const diff = now - ts
  if (diff < 60) return '刚刚'
  if (diff < 3600) return `${Math.floor(diff / 60)} 分钟前`
  if (diff < 86400) return `${Math.floor(diff / 3600)} 小时前`
  if (diff < 604800) return `${Math.floor(diff / 86400)} 天前`
  const d = new Date(ts * 1000)
  return `${d.getMonth() + 1}/${d.getDate()}`
}
