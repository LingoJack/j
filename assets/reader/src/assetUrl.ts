/**
 * 把 markdown 里写的图片/资源 URL 转成浏览器可加载的实际 src。
 *
 * - `http(s):` / `data:` 直通
 * - 绝对路径（以 `/` 起头）→ `./api/asset?path=<原样>`
 * - 相对路径 → 与 `baseDir`（当前文件所在目录）拼接、规范化、再走 `/api/asset`
 *
 * baseDir 由 Reader.tsx 在打开 tab 时计算并通过 React Context 注入。
 *
 * 中文/特殊字符处理：很多 markdown 编辑器（Typora 等）写图片路径时会把
 * 非 ASCII 字符 percent-encode 后再写进 markdown 源文件，例如
 * `![](内存修复测试计划.assets/foo.png)` 实际存为
 * `![](%E5%86%85%E5%AD%98...assets/foo.png)`。这里在 join / normalize
 * 之前先 decode 一次，把它还原成真实路径字节；最后再统一 encodeURIComponent，
 * 否则会出现 `%25E5` 这样的双重编码，后端 canonicalize 找不到文件 → 400。
 */
export function resolveAssetUrl(url: string, baseDir: string | null): string {
  if (/^(https?:|data:)/i.test(url)) return url
  const decoded = safeDecode(url)
  if (decoded.startsWith('/')) {
    return `./api/asset?path=${encodeURIComponent(decoded)}`
  }
  if (!baseDir) return url
  const joined = (baseDir.endsWith('/') ? baseDir : baseDir + '/') + decoded
  const normalized = normalizePath(joined)
  return `./api/asset?path=${encodeURIComponent(normalized)}`
}

/**
 * 容错的 percent-decode：失败（malformed % 序列）时退回原串。
 *
 * decodeURIComponent 对包含字面量 `%` 但不是合法 escape 的字符串会抛
 * URIError；真实文件名里出现孤立 `%` 不常见但合法。
 */
function safeDecode(s: string): string {
  try {
    return decodeURIComponent(s)
  } catch {
    return s
  }
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
