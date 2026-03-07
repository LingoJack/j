# 命令详细说明

> 本文档详细描述 `j` 的所有命令及其使用方式。

---

## 别名管理

### set — 设置别名

```bash
j set <alias> <path>      # 设置别名
j s chrome /Applications/Google\ Chrome.app
j s github https://github.com
```

- 路径自动归类到 `path` section
- URL 自动识别并归类到 `inner_url` section
- 路径含空格时用引号包裹

### remove / rm — 删除别名

```bash
j rm <alias>              # 删除别名（同时清理关联的分类标记）
j rm chrome
```

### rename / rn — 重命名别名

```bash
j rn <alias> <new>        # 重命名（同步更新所有分类引用）
j rn chrome browser
```

### modify / mf — 修改路径

```bash
j mf <alias> <new_path>   # 修改别名指向的路径
j mf chrome /Applications/Chrome.app
```

---

## 分类标记

### note / nt — 标记分类

```bash
j note <alias> <category>   # 标记别名分类
j nt chrome browser
j nt vscode editor
```

可用分类: `browser`, `editor`, `vpn`, `outer_url`, `script`

### denote / dnt — 解除分类

```bash
j denote <alias> <category> # 解除别名分类
j dnt chrome browser
```

---

## 列表 & 查找

### list / ls — 列出别名

```bash
j ls              # 列出常用别名（path/url/browser/editor 等）
j ls all          # 列出所有 section 下的别名
j ls path         # 列出指定 section
```

### contain / find — 查找别名

```bash
j contain <alias>              # 在所有分类中查找别名
j contain chrome               # 输出: path, browser
j contain <alias> <sections>   # 在指定分类中查找（逗号分隔）
j contain chrome path,browser
```

---

## 打开命令

这是最核心的命令，支持多种打开模式：

```bash
j <alias>                   # 直接打开（app/文件/URL）
j chrome

j <browser> <url_alias>     # 用指定浏览器打开 URL
j chrome github

j <browser> <任意文本>      # 用浏览器搜索（默认 Bing）
j chrome "rust lang"

j <editor> <文件路径>       # 用编辑器打开文件
j vscode ./src

j <alias> <额外参数...>     # 带参数打开
j vscode ./src --new-window

j <script_alias> -w         # 在新终端窗口中执行脚本
j my-script -w arg1 arg2
```

### 智能识别

- **CLI 可执行文件**：在当前终端执行，继承 stdin/stdout，支持管道
- **GUI 应用**（`.app`）：系统 `open` 命令打开新窗口
- **URL**：系统 open

---

## 日报系统

### report / r — 写入日报

```bash
j report <content>          # 写入日报（自动追加日期前缀）
j r "完成功能开发"

j report                    # 无参数 → 进入全屏 TUI 编辑器
```

### reportctl / rctl — 日报元数据操作

```bash
j reportctl new [date]      # 开启新的一周（周数+1）
j reportctl sync [date]     # 同步周数和日期
j reportctl push [msg]      # 推送周报到远程 git 仓库
j reportctl pull            # 从远程 git 仓库拉取周报
j reportctl set-url [url]   # 设置/查看 git 仓库地址
j reportctl open            # 用 TUI 编辑器打开日报文件全文编辑
```

### check / c — 查看日报

```bash
j check           # 查看最近 5 行
j check 20        # 查看最近 20 行
j check open      # 用 TUI 编辑器打开日报文件
```

### search — 搜索日报

```bash
j search <N> <keyword>      # 在最近 N 行中搜索
j search all <keyword>      # 在全部日报中搜索
j search 20 bug -f          # 模糊搜索（大小写不敏感）
```

---

## 待办备忘录

### 基本命令

```bash
j todo                # 进入 TUI 待办管理界面
j td                  # 同上（别名）
j todo add 买牛奶     # 快速添加一条待办
j todo list           # 输出待办列表
j todo list --done    # 仅显示已完成的待办
j todo list --undone  # 仅显示未完成的待办
```

### TUI 快捷键

| 按键 | 功能 |
|------|------|
| `n` / `↓` / `j` | 向下移动 |
| `N` / `↑` / `k` | 向上移动 |
| `空格` / `回车` | 切换完成状态 `[x]` / `[ ]` |
| `a` | 添加新待办 |
| `e` | 编辑选中待办 |
| `d` | 删除待办（需确认） |
| `y` | 复制到剪贴板 |
| `f` | 过滤切换 |
| `J` / `K` | 调整顺序 |
| `s` | 手动保存 |
| `q` / `Esc` | 退出 |

### 完成时写入日报联动

待办标记为完成时弹出确认框：
- `Enter` / `y` → 写入日报 + 自动保存 todo
- 其他键 → 跳过写入

---

## 脚本管理

### concat — 创建脚本

```bash
j concat <name> "<content>"    # 创建脚本并注册为别名
j concat deploy "echo 'deploying...'"

j concat <name>                # 脚本已存在 → 打开 TUI 编辑器修改
j concat <name>                # 无 content → 打开 TUI 编辑器创建
```

### 执行脚本

```bash
j <script> [args...]           # 在当前终端执行
j my-script arg1 arg2

j <script> -w [args...]        # 在新终端窗口中执行
j deploy -w
```

### 环境变量注入

执行脚本时自动注入所有别名路径为环境变量：

```bash
# 已注册: chrome → /Applications/Google Chrome.app
# 已注册: vscode → /Applications/Visual Studio Code.app

#!/bin/bash
open -a "$J_CHROME" https://example.com
"$J_VSCODE" ./src
```

命名规则：`J_<别名大写>`，`-` 转 `_`

---

## 倒计时器

```bash
j time countdown <duration>    # 启动倒计时
j time countdown 5m            # 5 分钟
j time countdown 30s           # 30 秒
j time countdown 1h            # 1 小时
```

---

## 系统命令

### log — 日志设置

```bash
j log mode verbose    # 详细日志
j log mode concise    # 简洁日志（默认）
```

### change — 修改配置

```bash
j change <section> <field> <value>
j change report week_report /custom/path/report.md
j change settings search-engine google
```

### 其他

```bash
j clear              # 清屏
j version            # 版本信息
j help               # 帮助信息
j completion zsh     # 生成 zsh 补全脚本
j completion bash    # 生成 bash 补全脚本
```

---

## Shell 补全

```bash
# 临时生效
eval "$(j completion zsh)"

# 持久化（推荐）
j completion zsh > ~/.zsh/completions/_j
# 在 .zshrc 中添加：
fpath=(~/.zsh/completions $fpath)
autoload -Uz compinit && compinit
```

---

## AI 对话

详见 [AI 对话系统文档](./chat.md)

```bash
j chat              # 进入 TUI 对话界面
j ai                # 同上（别名）
j chat 你好         # 快速提问
```

---

## 语音转文字

详见 [语音转文字文档](./voice.md)

```bash
j voice download           # 下载模型
j voice                    # 录音转文字
j voice -c                 # 转文字并复制到剪贴板
```
