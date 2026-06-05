/**
 * 轻量代码高亮 tokenizer。
 *
 * 用正则匹配关键字、字符串、注释、数字，生成带 CSS class 的 <span> DOM。
 * 不依赖外部库，在 createCodeBlockElement 中使用。
 *
 * CSS class 映射（在 editor.css 中定义颜色）：
 *   .hl-kw   关键字
 *   .hl-str  字符串
 *   .hl-num  数字
 *   .hl-cmt  注释
 *   .hl-type 类型名
 */

// ---- 语言关键字表 ----

const KEYWORDS: Record<string, Set<string>> = {
  rust: new Set(
    'fn let mut const if else match loop while for in return break continue struct enum impl trait type where pub use mod self super as ref static async await move dyn unsafe extern crate macro inline'.split(
      ' '
    )
  ),
  typescript: new Set(
    'function let const var if else switch case default for while do return break continue class interface type enum extends implements import export from as new throw try catch finally async await yield of in keyof readonly declare module namespace abstract static private protected public super this void'.split(
      ' '
    )
  ),
  javascript: new Set(
    'function let const var if else switch case default for while do return break continue class extends import export from as new throw try catch finally async await yield of in this void'.split(
      ' '
    )
  ),
  python: new Set(
    'def class if elif else for while return break continue import from as with try except finally raise pass lambda yield async await global nonlocal and or not in is True False None self'.split(
      ' '
    )
  ),
  go: new Set(
    'func package import if else switch case default for range return break continue go defer chan select type struct interface map const var iota nil error true false range'.split(
      ' '
    )
  ),
  bash: new Set(
    'if then else elif fi for while do done case esac in function return exit export local declare readonly set unset source alias echo read printf test true false'.split(
      ' '
    )
  ),
  sh: new Set(
    'if then else elif fi for while do done case esac in function return exit export local declare readonly set unset source alias echo read printf test true false'.split(
      ' '
    )
  ),
  sql: new Set(
    'select from where insert into update delete create table alter drop index join left right inner outer on and or not null is in like between exists group by order asc desc having limit offset set values as distinct count sum avg min max union all case when then else end primary key foreign references default constraint'.split(
      ' '
    )
  ),
  yaml: new Set('true false null yes no'.split(' ')),
  json: new Set('true false null'.split(' ')),
  toml: new Set('true false'.split(' ')),
}

const COMMENT_PREFIX: Record<string, string> = {
  rust: '//',
  typescript: '//',
  javascript: '//',
  go: '//',
  java: '//',
  cpp: '//',
  c: '//',
  cs: '//',
  swift: '//',
  kotlin: '//',
  python: '#',
  bash: '#',
  sh: '#',
  yaml: '#',
  toml: '#',
  ruby: '#',
  perl: '#',
  r: '#',
  sql: '--',
  lua: '--',
  haskell: '--',
  html: '',
  css: '',
}

function normalizeLang(lang: string): string {
  const l = lang.toLowerCase().trim()
  if (l === 'rs') return 'rust'
  if (l === 'ts') return 'typescript'
  if (l === 'js') return 'javascript'
  if (l === 'golang') return 'go'
  if (l === 'shell' || l === 'zsh') return 'bash'
  if (l === 'yml') return 'yaml'
  return l
}

// ---- Tokenizer ----

type TokenType = 'text' | 'kw' | 'str' | 'num' | 'cmt' | 'type' | 'punct'

interface Token {
  type: TokenType
  text: string
}

export function tokenizeCode(code: string, lang: string): Token[] {
  const normLang = normalizeLang(lang)
  const keywords = KEYWORDS[normLang]
  const commentPrefix = COMMENT_PREFIX[normLang] ?? ''
  const tokens: Token[] = []

  const lines = code.split('\n')
  for (let li = 0; li < lines.length; li++) {
    const line = lines[li]
    let pos = 0

    while (pos < line.length) {
      // ---- 多行注释 (/* ... */) ----
      if (line[pos] === '/' && pos + 1 < line.length && line[pos + 1] === '*') {
        const start = pos
        pos += 2
        // 在单行内找结束
        const endIdx = line.indexOf('*/', pos)
        if (endIdx >= 0) {
          tokens.push({ type: 'cmt', text: line.slice(start, endIdx + 2) })
          pos = endIdx + 2
        } else {
          // 跨行注释：取剩余所有行
          tokens.push({ type: 'cmt', text: line.slice(start) })
          // 把后续行也吃掉直到找到 */
          for (let ni = li + 1; ni < lines.length; ni++) {
            const endI = lines[ni].indexOf('*/')
            if (endI >= 0) {
              tokens.push({ type: 'cmt', text: '\n' + lines[ni].slice(0, endI + 2) })
              break
            } else {
              tokens.push({ type: 'cmt', text: '\n' + lines[ni] })
            }
          }
          pos = line.length
        }
        continue
      }

      // ---- 行注释 ----
      if (commentPrefix && line.slice(pos).startsWith(commentPrefix)) {
        tokens.push({ type: 'cmt', text: line.slice(pos) })
        pos = line.length
        continue
      }

      // ---- 双引号字符串 ----
      if (line[pos] === '"') {
        const start = pos
        pos++
        while (pos < line.length) {
          if (line[pos] === '\\' && pos + 1 < line.length) {
            pos += 2
            continue
          }
          if (line[pos] === '"') {
            pos++
            break
          }
          pos++
        }
        tokens.push({ type: 'str', text: line.slice(start, pos) })
        continue
      }

      // ---- 单引号字符串（非 Rust） ----
      if (line[pos] === "'" && !matchesLang(normLang, 'rust')) {
        const start = pos
        pos++
        while (pos < line.length) {
          if (line[pos] === '\\' && pos + 1 < line.length) {
            pos += 2
            continue
          }
          if (line[pos] === "'") {
            pos++
            break
          }
          pos++
        }
        tokens.push({ type: 'str', text: line.slice(start, pos) })
        continue
      }

      // ---- 反引号字符串 ----
      if (line[pos] === '`') {
        const start = pos
        pos++
        while (pos < line.length && line[pos] !== '`') pos++
        if (pos < line.length) pos++
        tokens.push({ type: 'str', text: line.slice(start, pos) })
        continue
      }

      // ---- 数字 ----
      if (
        isDigit(line[pos]) ||
        (line[pos] === '.' && pos + 1 < line.length && isDigit(line[pos + 1]))
      ) {
        const start = pos
        if (
          line[pos] === '0' &&
          pos + 1 < line.length &&
          (line[pos + 1] === 'x' || line[pos + 1] === 'X')
        ) {
          pos += 2
          while (pos < line.length && isHexDigit(line[pos])) pos++
        } else {
          while (pos < line.length && (isDigit(line[pos]) || line[pos] === '.')) pos++
        }
        // 后缀 (f64, u32, i64, etc.)
        while (pos < line.length && isAlpha(line[pos])) pos++
        tokens.push({ type: 'num', text: line.slice(start, pos) })
        continue
      }

      // ---- 标识符 / 关键字 ----
      if (isIdentStart(line[pos])) {
        const start = pos
        while (pos < line.length && isIdentPart(line[pos])) pos++
        const word = line.slice(start, pos)
        if (keywords && keywords.has(word)) {
          tokens.push({ type: 'kw', text: word })
        } else if (word[0] === word[0].toUpperCase() && isAlpha(word[0])) {
          tokens.push({ type: 'type', text: word })
        } else {
          tokens.push({ type: 'text', text: word })
        }
        continue
      }

      // ---- 其他字符 ----
      tokens.push({ type: 'punct', text: line[pos] })
      pos++
    }

    // 行间换行
    if (li < lines.length - 1) {
      tokens.push({ type: 'text', text: '\n' })
    }
  }

  return tokens
}

/** 将 token 列表渲染为 DOM fragment */
export function renderHighlightedCode(code: string, lang: string): DocumentFragment {
  const tokens = tokenizeCode(code, lang)
  const frag = document.createDocumentFragment()

  for (const token of tokens) {
    if (token.type === 'text') {
      frag.appendChild(document.createTextNode(token.text))
    } else {
      const span = document.createElement('span')
      span.className = `hl-${token.type}`
      span.textContent = token.text
      frag.appendChild(span)
    }
  }

  return frag
}

// ---- Helpers ----

function isDigit(c: string): boolean {
  return c >= '0' && c <= '9'
}
function isHexDigit(c: string): boolean {
  return isDigit(c) || (c >= 'a' && c <= 'f') || (c >= 'A' && c <= 'F')
}
function isAlpha(c: string): boolean {
  return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z')
}
function isIdentStart(c: string): boolean {
  return isAlpha(c) || c === '_'
}
function isIdentPart(c: string): boolean {
  return isAlpha(c) || isDigit(c) || c === '_'
}
function matchesLang(lang: string, ...targets: string[]): boolean {
  return targets.includes(lang)
}
