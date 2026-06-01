/**
 * 图片相对路径解析 —— 把 `![](./pics/foo.png)` 的 src 代理到
 * `./api/asset?path=<绝对路径>`。
 *
 * 关键：**只改渲染层**（Decoration.node 的 attrs 会 merge 到 toDOM 出来的
 * `<img>` 上），不动 ProseMirror 节点的 `attrs.src`。否则 toMarkdown 会把
 * 解析后的 API URL 写回源文件，相当于在用户不知情时改写了 markdown。
 *
 * baseDir 是当前打开文件所在目录（由 Reader.tsx 通过 prop 注入），相对路径
 * 在它下面拼绝对路径。绝对路径 / data: / http(s): 直通。
 */

import { $prose } from '@milkdown/kit/utils'
import { Plugin, PluginKey } from '@milkdown/kit/prose/state'
import { Decoration, DecorationSet } from '@milkdown/kit/prose/view'
import type { Node as ProseNode } from '@milkdown/kit/prose/model'
import { resolveAssetUrl } from '../assetUrl'

const KEY = new PluginKey('SEEYUE_IMAGE_SRC')

export function seeyueImageResolver(baseDir: string | null) {
  return $prose(() => {
    const build = (doc: ProseNode) => {
      const decos: Decoration[] = []
      doc.descendants((node, pos) => {
        if (node.type.name !== 'image') return
        const src = (node.attrs.src as string | undefined) ?? ''
        if (!src) return
        const resolved = resolveAssetUrl(src, baseDir)
        if (resolved && resolved !== src) {
          decos.push(Decoration.node(pos, pos + node.nodeSize, { src: resolved }))
        }
      })
      return DecorationSet.create(doc, decos)
    }

    return new Plugin({
      key: KEY,
      state: {
        init: (_config, { doc }) => build(doc),
        apply: (tr, set) => (tr.docChanged ? build(tr.doc) : set),
      },
      props: {
        decorations(state) {
          return this.getState(state) ?? null
        },
      },
    })
  })
}
