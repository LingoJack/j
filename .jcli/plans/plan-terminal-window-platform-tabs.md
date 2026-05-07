# 优化 Terminal Window 样式 + 平台 Tab 切换

## 目标
将当前简陋的 "macOS / Linux" / "Windows" 按钮替换为**终端窗口风格框体**，内嵌 **Tab 切换**来区分不同平台，整体呈现仿 macOS 终端窗口的外观（标题栏 + 红黄绿三个圆点 + Tab 切换）。

## 涉及改动

### 1. 新建 `TerminalWindow` 通用组件
**文件**: `web/src/components/common/TerminalWindow.tsx`

一个通用的终端窗口容器组件，支持：
- 仿 macOS 窗口标题栏（三个红黄绿圆点 + 可选标题文字）
- 内嵌 Tab 切换栏（macOS/Linux | Windows），选中态高亮
- 内容区域（children slot）
- 两种尺寸变体：`size="sm"` 和 `size="lg"`
- 窗口阴影 + 圆角 + 细边框，形成浮动感

设计样式参考：
```
┌─────────────────────────────────────────┐
│ ● ● ●   Terminal                        │  ← 标题栏
│─────────────────────────────────────────│
│ [macOS/Linux] [Windows]                 │  ← Tab 栏
│─────────────────────────────────────────│
│ $ curl -fsSL ... | bash                 │  ← 内容区
│                                         │
└─────────────────────────────────────────┘
```

### 2. 重构 `CodeBlock` 组件
**文件**: `web/src/components/common/CodeBlock.tsx`

- 保持原有简单用法（仅展示代码，无终端窗口壳）
- 调整内部样式使其在 `TerminalWindow` 内嵌时更加协调

### 3. 重构 `HeroSection`
**文件**: `web/src/components/home/HeroSection.tsx`

- 移除当前简陋的 platform 按钮组
- 用 `TerminalWindow` 包裹安装命令展示，内嵌 Tab 切换平台
- Tab 切换时内容区域动画过渡

### 4. 重构 `QuickStartSection`
**文件**: `web/src/components/home/QuickStartSection.tsx`

- Installation 区域用 `TerminalWindow` 包裹，Tab 切换平台
- 移除原有的独立按钮组

### 5. 重构 `CTASection`
**文件**: `web/src/components/home/CTASection.tsx`

- 用 `TerminalWindow` 包裹安装命令（深色背景下的终端窗口变体）
- 添加 Tab 切换（同样支持平台切换，需要在 CTASection 接收 platform props）

### 6. 更新 `Home.tsx` 页面
- 传递 platform 状态给 `CTASection`

## 样式细节

### TerminalWindow 标题栏
- 背景: `#e8e6e1`（浅灰暖色）
- 三个圆点: 红 `#ff5f57`、黄 `#febc2e`、绿 `#28c840`
- 圆点尺寸: 10px
- 标题文字: 小号、居中、灰色

### Tab 栏
- 未选中: 透明背景、灰色文字
- 选中: 白色背景、深色文字、微阴影、圆角
- Tab 左侧带小图标：macOS 显示 `⌘`、Windows 显示 `⊞`（或用文字图标）

### 内容区
- 背景: `#1e1e1e`（深色，模拟真实终端）
- 文字: 浅绿色或白色等宽字体
- 左侧显示 `$` 提示符（unix）或 `>` 提示符（windows）

### 响应式
- 移动端：Tab 文字缩短为 "Mac" / "Win"
- 桌面端：完整显示 "macOS / Linux" / "Windows"

## 实现顺序

1. 创建 `TerminalWindow` 组件（含样式）
2. 重构 `HeroSection`，接入 `TerminalWindow`
3. 重构 `QuickStartSection`，接入 `TerminalWindow`
4. 重构 `CTASection`，接入 `TerminalWindow`
5. 更新 `Home.tsx` 传递 props
6. 测试各场景的视觉效果
