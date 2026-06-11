# JStudio Tailwind CSS v4 样式优化方案

## 当前状态分析

**已完成的部分（无需变更）：**
- Tailwind CSS v4 已通过 `@tailwindcss/vite` 插件集成
- `@theme` 已正确定义设计令牌（颜色、字体、动画）
- 大部分组件已使用 Tailwind 类名
- 编辑器 CSS 使用 `@apply` 指令
- 主题切换通过 `[data-theme='warm']` CSS 变量覆盖实现
- `prefers-reduced-motion` 动效降级已实现

**可优化的方向：**

### 1. 减少 JSX 中的内联 style 属性
部分组件仍然混合使用 `style={{ ... }}` 和 Tailwind 类，可以统一为纯 Tailwind：

| 文件 | 当前用法 | 优化方案 |
|------|---------|---------|
| `FileTree.tsx` | `style={{ paddingLeft: indent(depth) }}` | 使用 CSS 变量 `style={{ '--indent': depth }}` + Tailwind `pl-[var(--indent)]` |
| `TabBar.tsx` | `style={{ left: menu.x, top: menu.y }}` | 定位计算必须用内联，保留 |
| `TabBar.tsx` | `style={{ width: 14, height: 14, color: ... }}` | 转为 Tailwind `w-3.5 h-3.5 text-seeyue-*` |
| `FileTree.tsx` | `style={{ width: 18, height: 18 }}` + `style={{ fontSize: 11 }}` | 转为 Tailwind 类 |

### 2. 提取长 className 为 @apply 组件类
以下组件的 className 字符串过长（>200 字符），建议在 CSS 中用 `@apply` 提取为语义化类名：

- `ActivityBar.tsx` 中的按钮类（~240字符）
- `TabBar.tsx` 中的 tab 项类（~350字符）
- `FileTree.tsx` 中的 `EntryRow` 类（~380字符）
- `Toast.tsx` 中的容器类（~300字符）
- `TableOfContents.tsx` 中的链接类

### 3. CSS 文件结构优化
`reader.css` 中混合了三种关注点，建议拆分注释分区使其更清晰：
- `@theme` → 设计令牌（已清晰）
- `@layer base` → 全局基线（已清晰）
- `@layer components` → 组件类（可进一步分区）

`editor/editor.css` 已经结构良好，仅做微调。

### 4. 统一 data attribute 模式
当前混用 `data-active`、`data-open`、`data-dirty`、`data-tone`、`data-selected` 等，模式一致，保持即可。

### 5. 优化按钮/交互元素的通用样式
多处重复的按钮基础样式（`border-0 bg-transparent cursor-pointer transition-colors duration-150`）可提取为 `@layer components` 中的基础类。

## 具体实施计划

### Phase 1: 提取通用组件类到 CSS
在 `reader.css` 的 `@layer components` 中新增：

```css
/* 通用按钮基础 */
.seeyue-btn-ghost { /* 透明背景按钮 */ }
.seeyue-btn-icon { /* 图标按钮（方形/圆形） */ }
.seeyue-menu-item { /* 右键菜单项 */ }
.seeyue-context-menu { /* 右键上下文菜单容器 */ }
```

### Phase 2: 简化各组件 JSX
将组件中重复的长 className 替换为提取的组件类 + 条件类：
- `TabBar.tsx` → 使用 `.seeyue-tab-item`
- `ActivityBar.tsx` → 使用 `.seeyue-activity-btn`
- `FileTree.tsx` → 使用 `.seeyue-tree-row`
- `Toast.tsx` → 使用 `.seeyue-toast`
- `TableOfContents.tsx` → 使用 `.seeyue-toc-link`

### Phase 3: 消除可替代的内联 style
- `TabBar.tsx` 中 `TabIcon` 的 `style={{ width: 14, height: 14, color: ... }}` → Tailwind 类
- `FileTree.tsx` 中清除按钮的 `style={{ width: 18, height: 18 }}` + `style={{ fontSize: 11 }}` → Tailwind 类
- `FileTree.tsx` 中缩进使用 CSS 自定义属性方案

### Phase 4: 验证
- `pnpm build` 确认构建通过
- 视觉对比确认无回归

## 预期收益

1. **可读性**：JSX 中 className 长度大幅缩短，组件结构更清晰
2. **可维护性**：样式变更集中在 CSS 文件中，而非散落在各组件
3. **一致性**：通用 UI 元素（按钮、菜单等）样式统一管理
4. **Tailwind v4 最佳实践**：充分利用 `@apply` + `@layer components` 的组合

## 不做的事

- 不改变现有视觉设计
- 不改变 Tailwind v4 配置方式
- 不引入新的运行时依赖
- 不重构编辑器 CSS（`editor/editor.css` 已经结构良好）
