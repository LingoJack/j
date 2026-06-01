/**
 * 临时的性能探针 ProseMirror 插件 —— 把每个 transaction 的耗时打到 console。
 *
 * 用法：在 MilkdownEditor 里 `.use(perfProbe)` 启用。开启后，浏览器
 * console 每 2 秒打印一组聚合数据：
 *
 *   📊 reader perf (last 2s)
 *   ┌──────────┬───────┬──────┬──────┐
 *   │ stage    │ count │ avg  │ p95  │
 *   │ tx total │  37   │ 12.4 │ 24.1 │
 *   │ prism    │  12   │  7.3 │ 18.0 │
 *   └──────────┴───────┴──────┴──────┘
 *
 * 关注点：
 * - tx total > 16ms：单帧渲染掉帧（用户感觉到延迟）
 * - prism > 5ms：缓存效果不好（很可能命中率低 / 代码块超大）
 *
 * 启用受 `window.__SEEYUE_PROBE__` 控制，默认 true（开发期）；要关掉
 * 在 console 里 `window.__SEEYUE_PROBE__ = false` 即可。
 *
 * 完成性能调优后整体删掉这个文件 + 它在 MilkdownEditor 里的注册。
 */

import { Plugin, PluginKey } from '@milkdown/prose/state'
import { $prose } from '@milkdown/utils'

interface Sample {
  total: number
  // 其它阶段日后再加
}

const samples: Sample[] = []

declare global {
  interface Window {
    __SEEYUE_PROBE__?: boolean
  }
}

if (typeof window !== 'undefined' && window.__SEEYUE_PROBE__ === undefined) {
  window.__SEEYUE_PROBE__ = true
}

let flushTimer: number | null = null
let totalSinceLastFlush = 0
let killSwitchTripped = false

function scheduleFlush() {
  if (flushTimer != null) return
  flushTimer = window.setTimeout(() => {
    flushTimer = null
    if (samples.length === 0) return
    const totals = samples.map((s) => s.total).sort((a, b) => a - b)
    const sum = totals.reduce((a, b) => a + b, 0)
    const avg = sum / totals.length
    const p95 = totals[Math.min(totals.length - 1, Math.floor(totals.length * 0.95))]
    const max = totals[totals.length - 1]
    // —— 死循环防御：1 秒采样窗里 > 200 个 transaction，几乎必定是循环 ——
    if (samples.length > 200 && !killSwitchTripped) {
      killSwitchTripped = true
      window.__SEEYUE_PROBE__ = false
      // eslint-disable-next-line no-console
      console.warn(
        `🚨 reader perf probe 检测到 1s 内 ${samples.length} 个 transaction，` +
          `疑似插件死循环。已自动关闭 probe。检查最近改过的 ProseMirror plugin。`,
      )
    } else {
      // eslint-disable-next-line no-console
      console.log(
        `📊 reader perf · last ${samples.length} tx → avg=${avg.toFixed(1)}ms · p95=${p95.toFixed(1)}ms · max=${max.toFixed(1)}ms`,
      )
    }
    samples.length = 0
    totalSinceLastFlush = 0
  }, 1000)
}

export const perfProbe = $prose(() => {
  return new Plugin({
    key: new PluginKey('SEEYUE_PERF_PROBE'),
    state: {
      init: () => null,
      apply: (tr, _val) => {
        if (!window.__SEEYUE_PROBE__) return _val
        if (!tr.docChanged) return _val
        // 早期 cap：单帧采样 1000 条以上立刻关 probe，防止真有死循环把日志/内存撑爆
        if (totalSinceLastFlush > 1000) {
          if (!killSwitchTripped) {
            killSwitchTripped = true
            window.__SEEYUE_PROBE__ = false
            // eslint-disable-next-line no-console
            console.warn(
              '🚨 reader perf probe 单帧 transaction 数已破 1000，强制关闭。',
            )
          }
          return _val
        }
        totalSinceLastFlush++
        const t0 = performance.now()
        requestAnimationFrame(() => {
          const t1 = performance.now()
          samples.push({ total: t1 - t0 })
          scheduleFlush()
        })
        return _val
      },
    },
  })
})
