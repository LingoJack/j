---
name: 别名 & 打开
order: 2
---

## 别名管理

| 命令 | 说明 |
|------|------|
| `j set <alias> <path>` | 设置别名（路径自动归类到 path，URL 归类到 inner_url） |
| `j rm <alias>` | 删除别名（同时清理关联的分类标记） |
| `j rename <alias> <new>` | 重命名别名（同步更新所有分类引用） |
| `j mf <alias> <new_path>` | 修改别名指向的路径 |

## 分类标记

| 命令 | 说明 |
|------|------|
| `j tag <alias> <category>` | 标记别名分类 |
| `j untag <alias> <category>` | 解除别名分类 |

可用分类: `browser`, `editor`, `vpn`, `outer_url`, `script`

> 标记为 browser 后可以用 `j <browser> <url>` 打开链接或搜索
> 标记为 editor 后可以用 `j <editor> <file>` 打开文件

## 列表 & 查找

| 命令 | 说明 |
|------|------|
| `j ls` | 列出常用别名（path/url/browser/editor 等） |
| `j ls all` | 列出所有 section 下的别名 |
| `j ls <section>` | 列出指定 section（如 `j ls path`） |
| `j contain <alias>` | 在所有分类中查找别名 |
| `j contain <alias> <sections>` | 在指定分类中查找（逗号分隔） |

## 打开

| 命令 | 说明 |
|------|------|
| `j <alias>` | 打开应用/文件/URL |
| `j <browser> <url_alias>` | 用浏览器打开 URL |
| `j <browser> <text>` | 用浏览器搜索（默认 Bing，可配置） |
| `j <editor> <file>` | 用编辑器打开文件 |

> **智能识别**：CLI 可执行文件在当前终端执行（支持管道），GUI 应用(.app)用系统打开
