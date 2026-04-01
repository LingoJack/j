## 模式

| 模式 | 描述 |
|------|------|
| **Lite** | 轻量级 HTTP 控制（默认） |
| **CDP** | 通过 Chrome DevTools Protocol 实现完整浏览器自动化 |

## Lite 模式

```bash
# 启动 lite 模式
j browser lite

# 打开 URL
j browser open https://example.com

# 截图
j browser screenshot
```

## CDP 模式

```bash
# 启动 CDP 模式（需要 Chrome/Chromium）
j browser cdp

# 导航
j browser goto https://example.com

# 点击元素
j browser click "#submit-button"

# 输入文本
j browser type "#search" "查询内容"

# 截图
j browser screenshot
```

## 功能特性

- 截图捕获
- 元素交互
- 页面导航
- 脚本注入
- Cookie 管理
