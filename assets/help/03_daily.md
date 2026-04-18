---
name: 日报 & 待办
order: 3
---

## 日报系统

| 命令 | 说明 |
|------|------|
| `j report <content>` | 写入日报（自动追加日期前缀） |
| `j report` | 打开内置编辑器写日报（带最近几行上下文） |
| `j reportctl new [date]` | 开启新的一周（周数+1） |
| `j reportctl sync [date]` | 同步周数和日期 |
| `j reportctl push [msg]` | 推送周报到远程 git 仓库 |
| `j reportctl pull` | 从远程 git 仓库拉取周报 |
| `j reportctl set-url [url]` | 设置/查看 git 仓库地址 |
| `j reportctl open` | 用内置 TUI 编辑器打开日报文件全文编辑 |
| `j check [N]` | 查看日报最近 N 行（默认 10） |
| `j check open` | 直接打开日报全文编辑 |
| `j search <N/all> <kw>` | 在日报中搜索关键字 |
| `j search <N/all> <kw> -f` | 模糊搜索（大小写不敏感） |

> 日报默认路径: `~/.jdata/report/week_report.md`
> 自定义路径: `j config report week_report <path>`
> 配置远程仓库: `j reportctl set-url <repo_url>`

## 待办备忘录

| 命令 | 说明 |
|------|------|
| `j todo` | 进入 TUI 待办管理界面（全屏交互） |
| `j td` | 同上（别名） |
| `j todo add 买牛奶` | 快速添加一条待办 |
| `j todo list` / `j td list` | 输出待办列表（Markdown 渲染）|
| `j todo list --done` / `j td list -d` | 仅显示已完成的待办 |
| `j todo list --undone` / `j td list -u` | 仅显示未完成的待办 |

### TUI 界面快捷键

| 按键 | 功能 |
|------|------|
| `n` / `↓` / `j` | 向下移动 |
| `N` / `↑` / `k` | 向上移动 |
| `空格` / `回车` | 切换完成状态 `[x]` / `[ ]` |
| `a` | 添加新待办 |
| `e` | 编辑选中待办 |
| `d` | 删除待办（需确认） |
| `y` | 复制选中待办到系统剪切板 |
| `f` | 过滤切换（全部 / 未完成 / 已完成） |
| `J` / `K` | 调整待办顺序（下移 / 上移） |
| `s` | 手动保存 |
| `/` | 打开命令面板（toggle/edit/add/delete/copy/filter/move/save/quit/help） |
| `?` | 查看完整帮助 |
| `q` | 退出（有未保存修改时需先保存或用 `q!` 强制退出） |

### 完成时写入日报联动

标记完成时自动询问是否写入日报：

| 操作 | 效果 |
|------|------|
| `空格` / `回车` 标记完成 | 底部显示确认提示 |
| `Enter` / `y` / `Y` | 写入日报 + 自动保存 todo |
| 其他任意键 | 标记完成，不写入日报 |

> 数据存储路径: `~/.jdata/report/todo.json`
