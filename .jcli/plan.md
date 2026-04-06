# Plan: 优化导航栏和页内导航

## 问题分析

### 问题 1：导航栏在 docs 和 index 上很不同

**Home 页面 Nav.tsx:**
- 完整导航：Features、Docs、Quick Start、GitHub
- 带下拉菜单
- 响应式设计完善

**Docs 页面 Docs.tsx:**
- 简洁导航：只有 logo + "docs" + 语言切换 + GitHub
- 缺少返回首页的链接（虽然有 footer 里的 back 链接）

**建议：** 统一导航风格，Docs 页面添加返回首页链接。

### 问题 2：右侧页内导航定位不准确

**当前实现问题：**
1. IntersectionObserver 的 rootMargin 设置为 `'-80px 0px -70% 0px'`，但顶部导航高度是 65px
2. 点击 TOC 项后滚动，但没有立即更新 activeId
3. 滚动时 threshold: 0 可能在某些情况下不稳定

**解决方案：**
1. 修正 rootMargin 为 `'-70px 0px -80% 0px'`（顶部 70px 为导航栏高度）
2. 点击时立即设置 activeId，然后平滑滚动
3. 滚动完成后重新计算当前激活项

### 问题 3：字体大小不合适

**当前样式：**
- h2 项：`text-sm` (14px)
- h3 项：`text-xs` (12px)

**建议调整：**
- h2 项：`text-sm` 保持不变，但增加行高
- h3 项：`text-sm` 提升到 14px
- 整体 padding 和间距优化

## 解决方案

### Part 1: 统一导航栏风格

**Docs 页面导航优化：**
- 添加返回首页链接
- 保持简洁风格但增加关键入口

```tsx
// Docs.tsx navigation
<nav>
  <div className="flex items-center gap-3">
    <Link to="/" className="flex items-center gap-2">
      <span className="text-2xl font-bold text-stone-900">j</span>
      <span className="text-stone-400 text-sm hidden sm:inline">docs</span>
    </Link>
  </div>
  
  <div className="flex items-center gap-4">
    <Link to="/" className="text-stone-500 hover:text-stone-900 text-sm">
      {lang === 'zh' ? '首页' : 'Home'}
    </Link>
    <LanguageSwitcher ... />
    <a href="github..." ... />
  </div>
</nav>
```

### Part 2: 修复 TOC 滚动定位

**优化滚动检测逻辑：**

```tsx
// TOC.tsx 改进

// 1. 点击时立即更新 activeId，再滚动
const handleClick = (id: string) => {
  setActiveId(id)  // 立即更新
  const element = document.getElementById(id)
  if (element) {
    const navHeight = 70  // 导航栏高度
    const elementTop = element.offsetTop - navHeight
    window.scrollTo({
      top: elementTop,
      behavior: 'smooth'
    })
  }
}

// 2. IntersectionObserver 使用更精确的 rootMargin
const observer = new IntersectionObserver(
  (entries) => {
    // 从上往下找第一个进入视口的标题
    const visibleEntries = entries
      .filter(e => e.isIntersecting)
      .sort((a, b) => a.boundingClientRect.top - b.boundingClientRect.top)
    
    if (visibleEntries.length > 0) {
      setActiveId(visibleEntries[0].target.id)
    }
  },
  {
    rootMargin: '-70px 0px -80% 0px',  // 顶部 70px 为导航栏
    threshold: 0
  }
)

// 3. 使用 scroll 事件作为备用检测
useEffect(() => {
  const handleScroll = () => {
    // 找到当前视口中最靠近顶部的标题
    const headings = headingsRef.current
    const navHeight = 70
    const scrollY = window.scrollY + navHeight
    
    for (let i = headings.length - 1; i >= 0; i--) {
      const el = document.getElementById(headings[i].id)
      if (el && el.offsetTop <= scrollY) {
        setActiveId(headings[i].id)
        break
      }
    }
  }
  
  window.addEventListener('scroll', handleScroll, { passive: true })
  return () => window.removeEventListener('scroll', handleScroll)
}, [headings])
```

### Part 3: 优化 TOC 字体和样式

**调整字体大小：**
```tsx
// h2 项
className="text-sm py-1.5 px-3 ..."  // 14px，保持不变

// h3 项  
className="text-sm py-1 px-3 pl-6 ..."  // 从 text-xs 改为 text-sm

// 或者区分更明显：
// h2: text-sm font-medium
// h3: text-xs (保持小一号)
```

## 文件变更清单

| 文件 | 操作 |
|------|------|
| `web/src/components/docs/TOC.tsx` | 修复滚动定位 + 优化字体 |
| `web/src/pages/Docs.tsx` | 添加返回首页链接 |

## 详细实现

### 1. TOC.tsx 完整重写

```tsx
import { useMemo, useEffect, useState, useRef, useCallback } from 'react'
import type { Language } from '../../types'

interface TOCItem {
  id: string
  text: string
  level: number
}

interface TOCProps {
  content: string
  lang: Language
}

const tocTitleI18n: Record<Language, string> = {
  en: 'On This Page',
  zh: '本文目录'
}

const NAV_HEIGHT = 70  // 顶部导航栏高度

function slugify(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^\w\u4e00-\u9fa5]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 50)
}

function extractHeadings(content: string): TOCItem[] {
  const lines = content.split('\n')
  const headings: TOCItem[] = []
  const usedIds = new Set<string>()
  
  lines.forEach(line => {
    let text: string
    let level: number
    
    if (line.startsWith('## ')) {
      text = line.slice(3).trim()
      level = 2
    } else if (line.startsWith('### ')) {
      text = line.slice(4).trim()
      level = 3
    } else {
      return
    }
    
    text = text.replace(/\*\*([^*]+)\*\*/g, '$1')
    text = text.replace(/\*([^*]+)\*/g, '$1')
    text = text.replace(/`([^`]+)`/g, '$1')
    
    let id = slugify(text)
    let counter = 1
    while (usedIds.has(id)) {
      id = `${slugify(text)}-${counter}`
      counter++
    }
    usedIds.add(id)
    
    headings.push({ id, text, level })
  })
  
  return headings
}

export function TOC({ content, lang }: TOCProps) {
  const headings = useMemo(() => extractHeadings(content), [content])
  const [activeId, setActiveId] = useState<string | null>(null)
  const isScrollingRef = useRef(false)
  
  // 点击滚动
  const scrollToHeading = useCallback((id: string) => {
    const element = document.getElementById(id)
    if (!element) return
    
    setActiveId(id)
    isScrollingRef.current = true
    
    const elementTop = element.offsetTop - NAV_HEIGHT
    window.scrollTo({
      top: elementTop,
      behavior: 'smooth'
    })
    
    // 滚动结束后恢复检测
    setTimeout(() => {
      isScrollingRef.current = false
    }, 500)
  }, [])
  
  // 滚动检测
  useEffect(() => {
    if (headings.length === 0) return
    
    const handleScroll = () => {
      if (isScrollingRef.current) return
      
      const scrollY = window.scrollY + NAV_HEIGHT + 10
      
      // 找到当前滚动位置对应的标题
      let currentId: string | null = null
      for (const heading of headings) {
        const el = document.getElementById(heading.id)
        if (el && el.offsetTop <= scrollY) {
          currentId = heading.id
        }
      }
      
      if (currentId) {
        setActiveId(currentId)
      }
    }
    
    // 初始化
    handleScroll()
    
    window.addEventListener('scroll', handleScroll, { passive: true })
    return () => window.removeEventListener('scroll', handleScroll)
  }, [headings])
  
  if (headings.length === 0) return null
  
  return (
    <nav className="hidden xl:block fixed right-0 top-[65px] w-52 h-[calc(100vh-65px)] border-l border-stone-200/70 bg-[#faf9f6]/95 backdrop-blur-sm">
      <div className="sticky top-0 px-4 py-3 border-b border-stone-200/50 bg-[#faf9f6]">
        <span className="text-xs font-semibold text-stone-400 uppercase tracking-wider">
          {tocTitleI18n[lang]}
        </span>
      </div>
      
      <ul className="p-2 overflow-y-auto max-h-[calc(100vh-120px)]">
        {headings.map(({ id, text, level }) => (
          <li key={id}>
            <button
              onClick={() => scrollToHeading(id)}
              className={`
                relative w-full text-left py-1.5 px-3 rounded-lg transition-all duration-200
                ${level === 3 ? 'pl-6 text-xs text-stone-400' : 'text-sm'}
                ${activeId === id 
                  ? 'text-stone-900 font-medium bg-stone-100 before:absolute before:left-0 before:top-1/2 before:-translate-y-1/2 before:w-0.5 before:h-4 before:bg-stone-900 before:rounded-full' 
                  : 'text-stone-500 hover:text-stone-700 hover:bg-stone-50'
                }
              `}
            >
              {text}
            </button>
          </li>
        ))}
      </ul>
    </nav>
  )
}
```

### 2. Docs.tsx 导航栏优化

```tsx
// 在导航栏右侧添加返回首页链接
<div className="flex items-center gap-3 sm:gap-5">
  <Link 
    to="/" 
    className="text-stone-500 hover:text-stone-900 transition-colors text-sm hidden sm:inline"
  >
    {lang === 'zh' ? '首页' : 'Home'}
  </Link>
  <LanguageSwitcher lang={lang} onChange={setLang} />
  <a href="github..." ... />
</div>
```

## 预期效果

### 导航栏
- Docs 页面导航增加返回首页入口
- 风格与 Home 页面保持一致

### TOC 滚动定位
- 点击 TOC 项立即高亮
- 滚动位置准确（考虑顶部导航栏 70px 高度）
- 滚动过程中正确追踪当前阅读位置

### 字体大小
- h2 项：14px (text-sm)
- h3 项：12px (text-xs)，比 h2 小一号，层次分明
