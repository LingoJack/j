## 命令

| 命令 | 描述 |
|------|------|
| `j set <别名> <路径>` | 设置别名（路径 → path 配置，URL → inner_url） |
| `j rm <别名>` | 删除别名（同时清理关联的分类标记） |
| `j rename <别名> <新别名>` | 重命名别名（更新所有分类引用） |
| `j mf <别名> <新路径>` | 修改别名路径 |

## 分类标记

```bash
j note <别名> <分类>   # 为别名标记分类
j find <分类>         # 按分类查找别名
j note chrome browser # 将 chrome 标记为浏览器
j note github outer_url # 将 github 标记为外网（自动连接 VPN）
```

## 分类说明

| 分类 | 描述 |
|------|------|
| `browser` | 浏览器 |
| `editor` | 编辑器 |
| `vpn` | VPN 应用 |
| `script` | 自定义脚本 |
| `inner_url` | 内网 URL |
| `outer_url` | 外网 URL（自动连接 VPN） |

## 示例

```bash
# 设置应用别名
j set chrome "/Applications/Google Chrome.app"
j set safari "/Applications/Safari.app"
j set vscode "/Applications/Visual Studio Code.app"

# 设置 URL 别名
j set github https://github.com
j set google https://google.com

# 设置目录别名
j set proj ~/Projects
j set docs ~/Documents

# 打开应用
j chrome                   # 打开 Chrome
j chrome "rust lang"       # 用 Chrome 搜索
j chrome github            # 用 Chrome 打开 github
j vscode proj              # 用 VSCode 打开 proj 目录
```
