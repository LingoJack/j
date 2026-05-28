/**
 * Source 字符串增量编辑工具。
 *
 * source 是一份 markdown 字符串（含 `\n`），所有编辑都在它上面做。
 * Offset 是从开头算起的字符偏移（含换行）。
 */

/** 在 source 的 offset 处插入 text；返回新 source 与新光标 offset */
export function insertAt(
  source: string,
  offset: number,
  text: string,
): { source: string; nextOffset: number } {
  const safe = Math.max(0, Math.min(offset, source.length))
  return {
    source: source.slice(0, safe) + text + source.slice(safe),
    nextOffset: safe + text.length,
  }
}

/** 删除 [start, end) 范围；返回新 source 与新光标 offset（= start） */
export function deleteRange(
  source: string,
  start: number,
  end: number,
): { source: string; nextOffset: number } {
  const a = Math.max(0, Math.min(start, end))
  const b = Math.min(source.length, Math.max(start, end))
  return {
    source: source.slice(0, a) + source.slice(b),
    nextOffset: a,
  }
}

/** 在 offset 前向删除 n 个字符（Backspace） */
export function backspace(
  source: string,
  offset: number,
  n: number = 1,
): { source: string; nextOffset: number } {
  if (offset <= 0) return { source, nextOffset: 0 }
  const start = Math.max(0, offset - n)
  return deleteRange(source, start, offset)
}

/** 在 offset 后向删除 n 个字符（Delete） */
export function forwardDelete(
  source: string,
  offset: number,
  n: number = 1,
): { source: string; nextOffset: number } {
  const end = Math.min(source.length, offset + n)
  return deleteRange(source, offset, end)
}

/** 把 [start, end) 替换为 text */
export function replaceRange(
  source: string,
  start: number,
  end: number,
  text: string,
): { source: string; nextOffset: number } {
  const a = Math.max(0, Math.min(start, end))
  const b = Math.min(source.length, Math.max(start, end))
  return {
    source: source.slice(0, a) + text + source.slice(b),
    nextOffset: a + text.length,
  }
}

/** 找到 offset 所在行的 [行首 offset, 行尾 offset]（不含换行） */
export function lineBoundsAt(
  source: string,
  offset: number,
): { lineStart: number; lineEnd: number; lineText: string } {
  const safe = Math.max(0, Math.min(offset, source.length))
  let lineStart = source.lastIndexOf('\n', safe - 1) + 1
  let lineEnd = source.indexOf('\n', safe)
  if (lineEnd === -1) lineEnd = source.length
  return {
    lineStart,
    lineEnd,
    lineText: source.slice(lineStart, lineEnd),
  }
}

/** 计算 offset 对应的（line, col）—— 行号 0 起算 */
export function offsetToLineCol(
  source: string,
  offset: number,
): { line: number; col: number } {
  const safe = Math.max(0, Math.min(offset, source.length))
  let line = 0
  let lastBreak = -1
  for (let i = 0; i < safe; i++) {
    if (source.charCodeAt(i) === 10) {
      line++
      lastBreak = i
    }
  }
  return { line, col: safe - lastBreak - 1 }
}

/** 反向：（line, col）→ offset */
export function lineColToOffset(
  source: string,
  line: number,
  col: number,
): number {
  if (line <= 0) return Math.min(col, source.length)
  let cur = 0
  let curLine = 0
  while (curLine < line && cur < source.length) {
    if (source.charCodeAt(cur) === 10) curLine++
    cur++
  }
  // 现在 cur 在第 line 行的第 0 列
  // 但不能跨过该行的换行
  const nextBreak = source.indexOf('\n', cur)
  const lineEnd = nextBreak === -1 ? source.length : nextBreak
  return Math.min(cur + col, lineEnd)
}
