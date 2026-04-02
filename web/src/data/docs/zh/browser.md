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

| Action | 描述 |
|--------|------|
| `status` | 检查浏览器状态 |
| `open` | 打开 URL，获取页面内容 |
| `snapshot` | 获取交互元素列表 |
| `content` | 提取正文文本 |
| `tabs` | 列出已打开的标签页 |
| `close` | 关闭标签页 |

限制：不支持 `click`、`type`、`press`、`evaluate`、`screenshot`

## CDP 模式

启用 `browser_cdp` feature 后支持完整浏览器自动化：

| Action | 描述 |
|--------|------|
| `start` | 启动浏览器 |
| `stop` | 停止浏览器 |
| `open` | 打开新标签页 |
| `navigate` | 导航到 URL |
| `screenshot` | 截图（支持全页） |
| `snapshot` | 获取页面快照（含元素 selector） |
| `content` | 提取正文文本 |
| `click` | 点击元素 |
| `type` | 输入文本（支持中文） |
| `press` | 按键（Enter/Tab/Escape 等） |
| `evaluate` | 执行 JavaScript |
| `tabs` | 列出标签页 |
| `close` | 关闭标签页 |

### 典型工作流

```
1. open 打开页面
2. snapshot 获取可交互元素列表（返回 selector）
3. 使用返回的 selector（如 [data-jref="e3"]）进行 click/type
4. content 获取结果
```

### headless 配置

在 `.jcli/config.yaml` 中配置：

```yaml
settings:
  browser_headless: true  # true=无窗口，false=显示窗口
```

或通过参数覆盖：`{ "action": "start", "headless": false }`

## 编译启用 CDP

```bash
cargo build --features browser_cdp
```
