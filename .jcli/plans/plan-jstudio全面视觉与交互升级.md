# JStudio 全面视觉与交互升级方案

## 一、升级目标

1. 视觉全面升级（配色、排版、阴影、圆角）
2. 交互全面升级（hover 动效、按钮反馈、过渡流畅度）
3. 布局范围优化（各面板尺寸自适应、边距调整）

---

## 二、视觉升级方案

### 2.1 配色系统升级

**新增品牌色**：
- 主色从 indigo 系升级为 `#6366f1` → `#4f46e5`（更沉稳）
- 添加 accent 色：emerald `#10b981`、amber `#f59e0b`、rose `#f43f5e`
- 中性色调整：slate 替换为 zinc，减少冷色感

**暗色模式精调**：
- 背景从 `#09090b` → `#0a0a0a`（更深邃）
- 卡片从 `#0f0f11` → `#0d0d0d`（更有层次）
- 边框从 `white/5` → `white/[0.04]`（更细腻）

### 2.2 排版升级

- 标题字号层级优化：H1 `3xl` → `4xl`，H2 `2xl` → `3xl`
- 行高统一：body `leading-relaxed` → `1.75`，heading `leading-tight` → `1.3`
- 字重优化：正文 `text-sm` → `text-[15px]`（更接近阅读舒适区）

### 2.3 阴影与圆角

- 圆角体系：`rounded-md` → `rounded-xl`，`rounded-lg` → `rounded-2xl`
- 新增 shadow 层次：`shadow-sm`、`shadow-md`、`shadow-lg`
- 使用 `shadow-color` 实现更自然的阴影

---

## 三、交互升级方案

### 3.1 Hover 效果升级

**全局 Hover 原则**：
- 所有可交互元素添加 `transition-all duration-200` 或 `duration-150`
- Hover 时背景色变化 + 轻微缩放 + 边框高亮
- 文字链接 hover 时添加下划线动画

**具体 Hover 效果**：

1. **侧栏文档列表项**：
   - 当前：仅背景色变化
   - 升级：背景色变化 + 左侧指示条（品牌色）+ 轻微向右平移 `translate-x-0.5`

2. **BlockEditor 块操作按钮**：
   - 当前：opacity 过渡
   - 升级：opacity 过渡 + 背景色圈 + 图标缩放 `scale-110`

3. **工具栏按钮**：
   - 当前：背景色变化
   - 升级：背景色变化 + 图标旋转 + 底部指示线

4. **LocalFolder 文件卡片**：
   - 当前：边框高亮
   - 升级：边框高亮 + 卡片上浮 `translate-y-[-2px]` + shadow 增强

5. **ArticleOutline 大纲项**：
   - 当前：仅文字颜色变化
   - 升级：背景色变化 + 左侧指示条 + 轻微文字缩放

### 3.2 点击反馈

- 所有按钮添加 `active:scale-95` 或 `active:scale-98`
- 表单输入框 focus 时添加 ring + 边框颜色变化
- 表格单元格编辑 focus 时添加内发光效果

### 3.3 加载与过渡

- 页面切换：`animate-in fade-in duration-300`（已有）
- 面板展开/收起：添加 `ease-out` 缓动
- 模态框/下拉菜单：`slide-in-from-bottom-2 duration-150`

---

## 四、布局范围优化

### 4.1 整体布局

**App.tsx 顶层**：
- 去掉固定 `p-2` 内边距，改为响应式 `p-1 md:p-2 lg:p-3`
- 去掉固定 `rounded-xl`，改为 `rounded-lg md:rounded-2xl`
- 阴影增强：`md:shadow-2xl` → `md:shadow-2xl md:shadow-black/5`

**主内容区**：
- 侧栏与主内容间距：`gap-0` → `gap-0 md:gap-px`（添加细微分隔线）
- 主编辑器区域左右 padding：`px-4 md:px-12` → `px-4 md:px-16 lg:px-24`（更大屏幕更宽敞）

### 4.2 侧栏 (DocumentList)

- 宽度：`md:w-60` → `md:w-[16rem]` + `min-w-[14rem]`
- 搜索框：`rounded` → `rounded-xl`，添加 `focus:ring-2` 
- 文档列表项：`py-1.5 px-2` → `py-2 px-3`（更大点击区域）
- Footer 按钮：`rounded-lg` → `rounded-xl`，添加 hover shadow

### 4.3 编辑器 (BlockEditor)

- 工具栏：`py-2` → `py-2.5 px-3`，添加底部阴影而非边框
- 文档标题：`text-3xl` → `text-4xl`，添加 focus 时底部高亮线
- 块之间间距：`space-y-1` → `space-y-2`（更呼吸）

### 4.4 块组件 (BlockItem)

- 左侧操作按钮：`left-[-36px]` → `left-[-40px]`（更好点击）
- 代码块：padding `p-4` → `p-5`，圆角 `rounded-md` → `rounded-xl`
- 表格：cell padding `p-1.5` → `p-2`
- HTML 沙盒 iframe：`h-96` → `h-[400px]`（更高工作区）
- 画布：`h-[500px]` → `h-[600px]`

### 4.5 LocalFolder 面板

- 宽度：`w-80` → `w-[22rem]` + `min-w-[18rem]`
- 文件卡片：`p-2.5` → `p-3.5`，添加 `hover:-translate-y-0.5`
- 拖放区域：`p-3` → `p-4`，虚线边框更明显
- 进度条：`h-1.5` → `h-2`，添加渐变背景

### 4.6 ArticleOutline 大纲

- 大纲项字体：`text-[11px]` → `text-xs`
- 大纲项 padding：添加 `py-1 px-2` hover 背景
- 大纲容器：添加细微背景色 `bg-slate-50/50 dark:bg-white/[0.02]`

---

## 五、新增交互特性

### 5.1 快捷键提示

- 在工具栏按钮添加 `title` 属性的键盘快捷键
- 添加全局快捷键提示面板（? 按钮）

### 5.2 拖拽反馈

- 侧栏文档列表项支持拖拽排序（视觉反馈）
- 文件上传区域拖拽时添加高亮边框 + 背景色变化

### 5.3 滚动条美化

- 自定义滚动条：更细、更圆角、hover 时显示
- 使用 `::-webkit-scrollbar` 定制

### 5.4 加载状态

- 新增区块创建时添加 skeleton loading 效果
- 图片加载时添加占位符 + 淡入效果

---

## 六、执行计划

按以下顺序修改文件：

1. `apps/jstudio/src/index.css` — 全局样式、滚动条、自定义属性
2. `apps/jstudio/src/App.tsx` — 顶层布局、顶部栏
3. `apps/jstudio/src/components/DocumentList.tsx` — 侧栏交互
4. `apps/jstudio/src/components/BlockEditor.tsx` — 编辑器布局
5. `apps/jstudio/src/components/BlockItem.tsx` — 各块组件交互
6. `apps/jstudio/src/components/LocalFolder.tsx` — 面板交互
7. `apps/jstudio/src/components/ArticleOutline.tsx` — 大纲交互
8. 构建验证
