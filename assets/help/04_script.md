---
name: 脚本 & 系统
order: 4
---

## 脚本 & 倒计时

| 命令 | 说明 |
|------|------|
| `j script <name> "<content>"` | 创建脚本并注册为别名（保存到 `~/.jdata/scripts/`） |
| `j script <name>` | 打开 TUI 编辑器创建或编辑脚本 |
| `j <script> [args...]` | 在当前终端执行脚本 |
| `j <script> -w [args...]` | 在**新终端窗口**中执行脚本 |
| `j time countdown <duration>` | 启动倒计时（支持 `30s` / `5m` / `1h`，不带单位默认按分钟） |

> `-w` 或 `--new-window` 标志可让脚本在新终端窗口中执行，用于需要后台运行的场景

### 脚本环境变量注入

执行脚本时，所有已注册的别名路径会自动注入为环境变量，命名规则为 `J_<别名大写>`（`-` 转为 `_`）：

```bash
#!/bin/bash
# 已注册: chrome -> /Applications/Google Chrome.app
# 已注册: my-tool -> /usr/local/bin/my-tool

open -a "$J_CHROME" https://example.com
"$J_MY_TOOL" --version
```

> 覆盖 section: `path`、`inner_url`、`outer_url`、`script`
> 路径含空格时，脚本中必须用双引号包裹变量：`"$J_CHROME"` 而非 `$J_CHROME`

## Markdown 笔记本

| 命令 | 说明 |
|------|------|
| `j md` / `j notebook` | 进入 TUI 笔记管理界面（全屏交互） |
| `j nb` | 同上（别名） |
| `j md <title>` | 打开指定笔记（不存在则新建） |
| `j md <file-path>` | 直接编辑任意 Markdown/文本文件（支持 `~/`、相对路径、绝对路径） |
| `j md list` | 列出所有笔记 |
| `j md search <keyword>` | 搜索笔记（标题+内容） |
| `j md delete <title>` | 删除笔记 |
| `j md open` | 在系统文件管理器中打开 notebook 根目录 |
| `j md rename <old> <new>` | 重命名笔记 |
| `j md mkdir <dir>` | 创建子目录 |
| `j md mv <src> <dest>` | 移动笔记到新路径 |

### TUI 界面快捷键

| 按键 | 功能 |
|------|------|
| `↓` / `j` / `n` | 向下移动 |
| `↑` / `k` / `N` | 向上移动 |
| `Enter` / `e` | 编辑选中笔记（打开内置 Markdown 编辑器） |
| `a` | 新建笔记（输入标题后打开编辑器） |
| `d` | 删除笔记（需确认） |
| `r` | 重命名笔记 |
| `p` | 全屏预览模式 |
| `/` | 打开命令面板（搜索/重命名/删除/新建目录/移动/调整比例/帮助） |
| `s` | 刷新笔记列表 |
| `y` | 复制笔记名到剪切板 |
| `o` | 在系统文件管理器中打开笔记目录 |
| `[` / `]` | 缩小/放大左侧面板比例（每次 5%） |
| `Tab` | 展开/折叠目录 |
| `Esc` | 退出（有搜索过滤时先清除过滤） |
| `?` | 查看帮助 |

> 笔记存储路径: `~/.jdata/notebook/`，支持子目录组织
> 支持对 notebook 内笔记和外部文件统一使用内置 Markdown 编辑器
> 命令面板可执行 `search`、`rename`、`delete`、`mkdir`、`mv`、`open`、`ratio`、`help`

## 系统设置

| 命令 | 说明 |
|------|------|
| `j log mode <verbose/concise>` | 设置日志模式 |
| `j config <section> <field> <val>` | 直接修改配置字段 |
| `j clear` | 清屏 |
| `j version` / `j v` | 版本信息 |
| `j help` / `j h` | 打开多标签帮助 TUI |
| `j exit` / `j q` / `j quit` | 退出（交互模式，或按 `Ctrl+Q` / `Ctrl+D`） |
| `j completion [shell]` | 生成 shell 补全脚本（支持 `zsh` / `bash`，默认 `zsh`） |
| `j update` / `j up` | 更新到最新版本（自动检测安装来源） |
| `j update --check` | 仅检查是否有新版本 |

### 帮助界面快捷键

| 按键 | 功能 |
|------|------|
| `←` / `→` / `h` / `l` | 切换帮助 Tab |
| `Tab` / `Shift+Tab` | 切换到下一个 / 上一个 Tab |
| `1`-`0` | 直接跳到指定 Tab |
| `↑` / `↓` / `j` / `k` | 滚动内容 |
| `PageUp` / `PageDown` | 快速滚动 |
| `Home` / `End` | 跳到顶部 / 底部 |
| `q` / `Esc` / `Ctrl+C` | 退出帮助 |
