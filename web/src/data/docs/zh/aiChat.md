## 启动 AI 对话

```bash
j chat              # 进入 TUI 对话界面
j chat "你好"       # 快速提问并打印回复
j chat -c           # 延续上一个会话
j chat --session <id>  # 恢复指定会话
```

## 远程控制

```bash
j chat --remote     # 启用远程控制（手机扫码）
j chat --remote --port 9390  # 指定端口
```

## 快捷键

| 快捷键 | 功能 |
|--------|------|
| `Enter` | 发送消息 |
| `Esc` | 取消响应/退出 |
| `Ctrl+T` | 切换模型 |
| `Ctrl+L` | 归档对话 |
| `Ctrl+Y` | 复制最后一条 AI 回复 |
| `Ctrl+B` | 消息浏览模式 |
| `Ctrl+E` | 打开配置界面 |
| `F1` 或 `?` | 显示帮助 |

## 上下文引用

输入框中以 `@` 触发补全：

```
@skill:<name>       # 引用技能
@command:<name>     # 引用自定义命令
@file:<path>        # 引用文件内容（支持图片）
```

## 多模型支持

支持 OpenAI、Claude、Gemini、Ollama 等模型，通过 `Ctrl+E` 打开配置界面管理。
