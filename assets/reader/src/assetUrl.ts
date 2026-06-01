/**
 * 把 markdown 里写的图片/资源 URL 转成浏览器可加载的实际 src。
 *
 * - `http(s):` / `data:` 直通
 * - 绝对路径（以 `/` 起头）→ `./api/asset?path=<原样>`
 * - 相对路径 → 与 `baseDir`（当前文件所在目录）拼接、规范化、再走 `/api/asset`
 *
 * baseDir 由 Reader.tsx 在打开 tab 时计算并通过 React Context 注入。
 */
export function resolveAssetUrl(url: string, baseDir: string | null): string {
  if (/^(https?:|data:)/i.test(url)) return url
  if (url.startsWith('/')) {
    return `./api/asset?path=${encodeURIComponent(url)}`
  }
  if (!baseDir) return url
  const joined = (baseDir.endsWith('/') ? baseDir : baseDir + '/') + url
  const normalized = normalizePath(joined)
  return `./api/asset?path=${encodeURIComponent(normalized)}`
}

/** 规范化绝对路径：消除空段、`./`、`../` */
function normalizePath(p: string): string {
  const parts = p.split('/')
  const out: string[] = []
  for (const seg of parts) {
    if (seg === '' || seg === '.') continue
    if (seg === '..') {
      if (out.length > 0) out.pop()
      continue
    }
    out.push(seg)
  }
  return '/' + out.join('/')
}
