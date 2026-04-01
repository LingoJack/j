## 概述

Browser 是 AI 对话中的工具，支持网页浏览、交互和内容提取。

## 模式

| 模式 | 描述 |
|------|------|
| **Lite** | 轻量级 HTTP 控制（默认，无需浏览器） |
| **CDP** | 通过 Chrome DevTools Protocol 实现完整浏览器自动化（需 `browser_cdp` feature） |

## 在 AI 对话中使用

```
打开 https://example.com 并总结内容

截取当前页面的截图

点击提交按钮
```

## Lite 模式

默认模式，使用 HTTP 请求获取网页内容：
- 获取页面文本
- 提取页面结构
- 获取交互元素列表

## CDP 模式

启用 `browser_cdp` feature 后支持完整浏览器自动化：
- 截图捕获
- 元素点击和输入
- 页面导航
- 脚本注入
- Cookie 管理

## 编译启用 CDP

```bash
cargo build --features browser_cdp
```
