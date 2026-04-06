# Plan: UI 样式重构 - 极简暗色风格

## 概述

将当前的毛玻璃效果(glass morphism)样式改为**极简暗色**设计，配合**大圆角**(16-20px)风格。

## 当前状态分析

### 现有样式特点
- **毛玻璃背景**: `backdrop-filter: blur(40px) saturate(180%)`
- **半透明背景**: `rgba(30, 30, 30, 0.85)`
- **边框**: 0.5px 细边框 + 大阴影
- **圆角**: 10-12px (`rounded-xl`)

### 需要修改的文件
1. `src-ui/src/styles/globals.css` - 主要样式定义
2. `src-ui/src/components/layout/SpotlightWindow.tsx` - 主容器组件
3. `src-tauri/tauri.conf.json` - 窗口透明设置(可选)

---

## 实施步骤

### Step 1: 重构 `globals.css` 样式

**修改内容:**

1. **移除 `.glass` 类**，替换为新的 `.minimal-dark` 类：
   ```css
   .minimal-dark {
     background: rgba(22, 22, 26, 0.98);
     border: none;
     box-shadow:
       0 25px 50px -12px rgba(0, 0, 0, 0.6),
       0 0 0 1px rgba(255, 255, 255, 0.05);
   }
   ```

2. **更新主题变量**：
   ```css
   --color-background: rgba(22, 22, 26, 0.98);
   --color-surface: rgba(32, 32, 38, 1);
   --color-border: rgba(255, 255, 255, 0.08);
   --radius-lg: 18px;
   ```

3. **添加细微内部高光**（可选，增加层次感）

### Step 2: 更新 `SpotlightWindow.tsx`

**修改内容:**

1. 将 `glass` 类名改为 `minimal-dark`
2. 将 `rounded-xl` 改为 `rounded-[18px]`
3. 调整分隔线颜色为更柔和：`border-white/[0.08]`

### Step 3: 更新 `SearchBar.tsx`（可选微调）

- 调整图标和 placeholder 的透明度，适配新背景

### Step 4: 更新 `ResultList.tsx`（可选微调）

- 调整选中状态背景色，适配深色背景
- 可考虑使用更柔和的高亮：`bg-white/[0.08]`

### Step 5: Tauri 配置（可选）

如果不再需要透明毛玻璃效果，可以考虑：
- 保持 `transparent: true`（推荐，保持圆角窗口）
- 或移除透明，使用纯色窗口边框

---

## 新样式设计规范

### 颜色系统
| 用途 | 颜色值 |
|------|--------|
| 主背景 | `rgba(22, 22, 26, 0.98)` |
| 表面层 | `rgba(32, 32, 38, 1)` |
| 悬停状态 | `rgba(255, 255, 255, 0.06)` |
| 选中状态 | `rgba(255, 255, 255, 0.10)` |
| 边框/分隔线 | `rgba(255, 255, 255, 0.08)` |
| 主文字 | `#FFFFFF` |
| 次要文字 | `rgba(255, 255, 255, 0.55)` |

### 圆角
- 主容器: `18px`
- 内部元素: `6-8px`

### 阴影
- 外阴影: `0 25px 50px -12px rgba(0, 0, 0, 0.6)`
- 边缘描边: `0 0 0 1px rgba(255, 255, 255, 0.05)`

---

## 文件变更清单

| 文件 | 操作 |
|------|------|
| `src-ui/src/styles/globals.css` | 重构样式 |
| `src-ui/src/components/layout/SpotlightWindow.tsx` | 更新类名 |
| `src-ui/src/components/layout/ResultList.tsx` | 微调样式 |
| `src-ui/src/components/layout/SearchBar.tsx` | 微调样式 |

---

## 预期效果

- 深色扁平外观，无毛玻璃模糊效果
- 大圆角 (18px)，现代 macOS Big Sur+ 风格
- 极简边框，仅保留细微描边
- 更清晰的文字对比度
- 整体更轻量、更现代的视觉感受
