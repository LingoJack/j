/**
 * Heading id slug 规则 —— 唯一定义源（single source of truth）。
 *
 * 这套规则被以下三处共同引用：
 * 1. `milkdown/headingId.ts` —— 让 Milkdown 输出的 <h*> 拿到正确 id
 * 2. `toc.ts` —— TOC 从服务端 IR 提取 heading 时，预测 id 用于
 *    `document.getElementById` 滚到对应 DOM
 * 3. （历史）服务端 `MarkdownIR` 已删，但服务端 `/api/parse` 出 IR 仍
 *    沿用此规则
 *
 * 重复规则 → 多源 → 漂移 → TOC 点击不跳。这里集中。
 */

/** 把任意文本压成 heading id slug。**未** 去重，去重逻辑在调用方处理。 */
export function slugify(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^\w一-鿿]+/g, '-')
    .replace(/^-|-$/g, '')
}

/**
 * 给定按出现顺序的 slug 序列，返回带 `-1` `-2` ... 后缀去重后的 id 列表。
 * 与服务端 / 旧 MarkdownIR 行为一致：第一次出现 = `slug`，后续 = `slug-N`（N≥1）。
 */
export function dedupSlugs(slugs: string[]): string[] {
  const counts: Record<string, number> = {}
  return slugs.map((slug) => {
    if (!slug) return slug
    const n = (counts[slug] = (counts[slug] ?? 0) + 1)
    return n === 1 ? slug : `${slug}-${n - 1}`
  })
}
