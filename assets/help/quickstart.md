---
name: 快速上手
order: 1
---

```bash
# ====== macOS / Linux ======

# 注册应用别名
j set chrome "/Applications/Google Chrome.app"
j set vscode "/Applications/Visual Studio Code.app"

# 注册 URL 别名（自动识别为 inner_url）
j set github https://github.com

# 标记分类（标记后支持组合打开）
j tag chrome browser
j tag vscode editor

# 一键打开
j chrome                  # 打开 Chrome
j chrome github           # 用 Chrome 打开 github 对应的 URL
j chrome "rust lang"      # 用 Chrome 搜索 "rust lang"
j vscode ./src            # 用 VSCode 打开 src 目录

# 写日报 & 查看
j report "完成功能开发"    # 写入今日日报
j check                   # 查看最近 10 行
j check 20                # 查看最近 20 行

# 进入交互模式（带 Tab 补全 + 历史建议）
j
```

```powershell
# ====== Windows ======

# 注册应用别名
j set notepad "C:\Windows\notepad.exe"
j set vscode "C:\Users\%USERNAME%\AppData\Local\Programs\Microsoft VS Code\Code.exe"

# 注册 URL 别名
j set github https://github.com

# 一键打开
j notepad                 # 打开记事本
j vscode .\src            # 用 VSCode 打开 src 目录

# 写日报 & 查看
j report "完成功能开发"    # 写入今日日报
j check                   # 查看最近 10 行

# 进入交互模式
j
```
