# work-copilot (j) — 快捷命令行工具 🚀

## 📦 别名管理

| 命令 | 说明 |
|------|------|
| `j set <alias> <path>` | 设置别名（路径/URL） |
| `j rm <alias>` | 删除别名 |
| `j rename <alias> <new>` | 重命名别名 |
| `j mf <alias> <new_path>` | 修改别名路径 |

## 🏷️ 分类标记

| 命令 | 说明 |
|------|------|
| `j note <alias> <category>` | 标记别名分类 |
| `j denote <alias> <category>` | 解除别名分类 |

category: *browser*, *editor*, *vpn*, *outer_url*, *script*

## 📋 列表 & 查找

| 命令 | 说明 |
|------|------|
| `j ls` | 列出常用别名 |
| `j ls all` | 列出所有别名 |
| `j ls <section>` | 列出指定 section |
| `j contain <alias>` | 在所有分类中查找别名 |
| `j contain <alias> <sections>` | 在指定分类中查找（逗号分隔） |

## 🚀 打开

| 命令 | 说明 |
|------|------|
| `j <alias>` | 打开应用/文件/URL |
| `j <browser> <url_alias>` | 用浏览器打开 URL |
| `j <browser> <text>` | 用浏览器搜索 |
| `j <editor> <file>` | 用编辑器打开文件 |

## 📝 日报系统

| 命令 | 说明 |
|------|------|
| `j report <content>` | 写入日报 |
| `j reportctl new [date]` | 开启新的一周（周数+1） |
| `j reportctl sync [date]` | 同步周数和日期 |
| `j reportctl push [msg]` | 推送周报到远程 git 仓库 |
| `j reportctl pull` | 从远程 git 仓库拉取周报 |
| `j reportctl set-url <url>` | 设置/查看 git 仓库地址 |
| `j check [N]` | 查看日报最近 N 行（默认 5） |
| `j search <N/all> <kw>` | 在日报中搜索关键字 |
| `j search <N/all> <kw> -f` | 模糊搜索（大小写不敏感） |

## 📜 脚本 & ⏳ 倒计时

| 命令 | 说明 |
|------|------|
| `j concat <name> "<content>"` | 创建脚本并注册为别名 |
| `j time countdown <duration>` | 启动倒计时（30s/5m/1h） |

## ⚙️ 系统设置

| 命令 | 说明 |
|------|------|
| `j log mode <verbose/concise>` | 设置日志模式 |
| `j change <part> <field> <val>` | 直接修改配置字段 |
| `j clear` | 清屏 |
| `j version` | 版本信息 |
| `j help` | 帮助信息 |
| `j exit` | 退出（交互模式） |

## 💡 提示

- 不带参数运行 `j` 进入**交互模式**
- 交互模式下用 `!` 前缀执行 shell 命令
- 路径可使用引号包裹处理空格
- URL 会自动识别并归类到 inner_url
- 日报默认存储在 `~/.jdata/report/week_report.md`
- 配置 git 仓库: `j reportctl set-url <repo_url>`
