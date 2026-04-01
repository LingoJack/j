## 注册应用别名

```bash
j set chrome "/Applications/Google Chrome.app"
j set vscode "/Applications/Visual Studio Code.app"

# 注册 URL 别名（自动识别为 inner_url）
j set github https://github.com
```

## 标记分类

```bash
j note chrome browser
j note vscode editor
```

## 打开应用

```bash
j chrome                  # 打开 Chrome
j chrome github           # 用 Chrome 打开 github URL
j chrome "rust lang"      # 用 Chrome 搜索 "rust lang"
j vscode ./src            # 用 VSCode 打开 src 目录
```

## 日报

```bash
j report "完成功能开发"
j check                   # 查看最近 10 行
j check 20                # 查看最近 20 行
```

## 待办管理

```bash
j todo add 买牛奶         # 快速添加
j todo                    # 进入 TUI 管理器
```

## AI 对话

```bash
j chat                    # 进入 TUI 对话
j chat 你好               # 快速提问
```

## 交互模式

```bash
j                         # 进入交互模式，支持 Tab 补全
```
