## 启动 AI 对话

```bash
j chat              # 打开 TUI 对话
j chat "你好"       # 快速提问
```

## 功能特性

- **多模型支持**：OpenAI、Claude、Gemini、Ollama
- **流式输出**：实时响应
- **工具调用**：AI 可以使用工具
- **上下文引用**：包含文件和 URL

## 上下文引用

```bash
# 包含本地文件
@file:src/main.rs 解释这段代码

# 包含目录
@dir:src/ 分析这个代码库

# 包含 URL
@url:https://example.com 总结这个页面
```

## 命令

| 命令 | 描述 |
|------|------|
| `/help` | 显示可用命令 |
| `/compact` | 压缩对话上下文 |
| `/clear` | 清空对话历史 |
| `/model` | 切换 AI 模型 |
| `/export` | 导出对话 |

## 网络搜索

启用网络搜索让 AI 获取最新信息：

```bash
React 19 有哪些新功能？
```
