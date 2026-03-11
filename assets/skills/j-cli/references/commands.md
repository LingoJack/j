# j-cli 命令完整参考

## 别名管理

| 命令                        | 说明                                   |
|---------------------------|--------------------------------------|
| `j set <alias> <path>`    | 设置别名（路径自动归类到 path，URL 归类到 inner_url） |
| `j rm <alias>`            | 删除别名（同时清理关联的分类标记）                    |
| `j rename <alias> <new>`  | 重命名别名（同步更新所有分类引用）                    |
| `j mf <alias> <new_path>` | 修改别名指向的路径                            |

## 分类标记

| 命令                            | 说明     |
|-------------------------------|--------|
| `j note <alias> <category>`   | 标记别名分类 |
| `j denote <alias> <category>` | 解除别名分类 |

**可用分类**: `browser`, `editor`, `vpn`, `outer_url`, `script`

> 标记为 browser 后可以用 `j <browser> <url>` 打开链接或搜索
> 标记为 editor 后可以用 `j <editor> <file>` 打开文件

## 列表 & 查找

| 命令                             | 说明                                |
|--------------------------------|-----------------------------------|
| `j ls`                         | 列出常用别名（path/url/browser/editor 等） |
| `j ls all`                     | 列出所有 section 下的别名                 |
| `j ls <section>`               | 列出指定 section（如 `j ls path`）       |
| `j contain <alias>`            | 在所有分类中查找别名                        |
| `j contain <alias> <sections>` | 在指定分类中查找（逗号分隔）                    |

## 打开命令

| 命令                        | 说明                  |
|---------------------------|---------------------|
| `j <alias>`               | 打开应用/文件/URL         |
| `j <browser> <url_alias>` | 用浏览器打开 URL          |
| `j <browser> <text>`      | 用浏览器搜索（默认 Bing，可配置） |
| `j <editor> <file>`       | 用编辑器打开文件            |

> **智能识别**：CLI 可执行文件在当前终端执行（支持管道），GUI 应用(.app)用系统打开

## 日报系统

| 命令                          | 说明                    |
|-----------------------------|-----------------------|
| `j report <content>`        | 写入日报（自动追加日期前缀）        |
| `j reportctl new [date]`    | 开启新的一周（周数+1）          |
| `j reportctl sync [date]`   | 同步周数和日期               |
| `j reportctl push [msg]`    | 推送周报到远程 git 仓库        |
| `j reportctl pull`          | 从远程 git 仓库拉取周报        |
| `j reportctl set-url [url]` | 设置/查看 git 仓库地址        |
| `j reportctl open`          | 用内置 TUI 编辑器打开日报文件全文编辑 |
| `j check [N]`               | 查看日报最近 N 行（默认 5）      |
| `j search <N/all> <kw>`     | 在日报中搜索关键字             |
| `j search <N/all> <kw> -f`  | 模糊搜索（大小写不敏感）          |

> 日报默认路径: `~/.jdata/report/week_report.md`
> 自定义路径: `j change report week_report <path>`
> 配置远程仓库: `j reportctl set-url <repo_url>`

## 待办备忘录

| 命令                          | 说明                  |
|-----------------------------|---------------------|
| `j todo`                    | 进入 TUI 待办管理界面（全屏交互） |
| `j td`                      | 同上（别名）              |
| `j todo add <content>`      | 快速添加一条待办            |
| `j todo list` / `j td list` | 输出待办列表（Markdown 渲染） |

### TUI 界面快捷键

| 按键                | 功能                          |
|-------------------|-----------------------------|
| `n` / `↓` / `j`   | 向下移动                        |
| `N` / `↑` / `k`   | 向上移动                        |
| `空格` / `回车`       | 切换完成状态 `[x]` / `[ ]`        |
| `a`               | 添加新待办                       |
| `e`               | 编辑选中待办                      |
| `d`               | 删除待办（需确认）                   |
| `y`               | 复制选中待办到系统剪切板                |
| `f`               | 过滤切换（全部 / 未完成 / 已完成）        |
| `J` / `K`         | 调整待办顺序（下移 / 上移）             |
| `s`               | 手动保存                        |
| `Alt+↑` / `Alt+↓` | 预览区滚动（长待办内容时可用）             |
| `?`               | 查看完整帮助                      |
| `q`               | 退出（有未保存修改时需先保存或用 `q!` 强制退出） |
| `q!`              | 强制退出（丢弃未保存的修改）              |

### 完成时写入日报联动

| 操作                  | 效果                 |
|---------------------|--------------------|
| `空格` / `回车` 标记完成    | 底部显示确认提示           |
| `Enter` / `y` / `Y` | ✅ 写入日报 + 自动保存 todo |
| 其他任意键               | ✅ 标记完成，不写入日报       |

## 脚本 & 倒计时

| 命令                            | 说明                      |
|-------------------------------|-------------------------|
| `j concat <name> "<content>"` | 创建脚本并注册为别名              |
| `j concat <name>`             | 脚本已存在时打开 TUI 编辑器修改      |
| `j <script> [args...]`        | 在当前终端执行脚本               |
| `j <script> -w [args...]`     | 在**新终端窗口**中执行脚本         |
| `j time countdown <duration>` | 启动倒计时（支持 30s / 5m / 1h） |

### 脚本环境变量注入

执行脚本时，所有已注册的别名路径会自动注入为环境变量：

- 命名规则：`J_<别名大写>`（`-` 转为 `_`）
- 覆盖 section：`path`、`inner_url`、`outer_url`、`script`

```bash
#!/bin/bash
# 已注册: chrome → /Applications/Google Chrome.app
open -a "$J_CHROME" https://example.com
```

> ⚠️ 路径含空格时，脚本中必须用双引号包裹变量

## 系统设置

| 命令                                 | 说明                         |
|------------------------------------|----------------------------|
| `j log mode <verbose/concise>`     | 设置日志模式                     |
| `j change <section> <field> <val>` | 直接修改配置字段                   |
| `j clear`                          | 清屏                         |
| `j version`                        | 版本信息                       |
| `j help`                           | 帮助信息                       |
| `j exit`                           | 退出（交互模式）                   |
| `j completion [shell]`             | 生成 shell 补全脚本（支持 zsh/bash） |

## 语音转文字

| 命令                           | 说明                                   |
|------------------------------|--------------------------------------|
| `j voice`                    | 录音 → Whisper 离线转写 → 输出文字             |
| `j voice -c`                 | 录音转写并复制结果到剪贴板                        |
| `j voice -m <model>`         | 指定模型大小（tiny/base/small/medium/large） |
| `j voice download`           | 下载默认模型（small）                        |
| `j voice download -m medium` | 下载指定大小的模型                            |
| `j vc`                       | 同 `j voice`（别名）                      |

> 模型存储路径: `~/.jdata/voice/model/`
> 推荐中文用 small（466MB）或 medium（1.5GB）模型

## AI 对话

| 命令                                    | 说明                |
|---------------------------------------|-------------------|
| `j chat` / `j ai`                     | 进入 TUI 对话界面（全屏交互） |
| `j chat <message>` / `j ai <message>` | 进入对话并发送首条消息       |

### 对话界面快捷键

| 按键                    | 功能            |
|-----------------------|---------------|
| `Enter`               | 发送消息          |
| `↑` / `↓`             | 滚动对话记录        |
| `PageUp` / `PageDown` | 快速滚动（10行）     |
| `←` / `→`             | 移动输入光标        |
| `Home` / `End`        | 跳到输入行首/行尾     |
| `Ctrl+T`              | 切换模型提供方       |
| `Ctrl+L`              | 归档当前对话（保存并清空） |
| `Ctrl+R`              | 还原归档对话        |
| `Ctrl+Y`              | 复制最后一条 AI 回复  |
| `Ctrl+B`              | 进入消息浏览模式      |
| `Ctrl+S`              | 切换流式/整体输出     |
| `Ctrl+E`              | 打开配置界面        |
| `?`                   | 显示帮助          |
| `Esc` / `Ctrl+C`      | 退出对话          |

### 配置界面快捷键

| 按键                | 功能                  |
|-------------------|---------------------|
| `↑` / `k`         | 向上移动光标              |
| `↓` / `j`         | 向下移动光标              |
| `Tab` / `→`       | 切换到下一个 Provider     |
| `Shift+Tab` / `←` | 切换到上一个 Provider     |
| `Enter`           | 进入编辑模式              |
| `a`               | 新增 Provider         |
| `d`               | 删除当前 Provider       |
| `s`               | 将当前 Provider 设为活跃模型 |
| `Esc`             | 保存配置并返回对话           |

### 消息浏览模式

| 按键            | 功能           |
|---------------|--------------|
| `↑` / `k`     | 选中上一条消息      |
| `↓` / `j`     | 选中下一条消息      |
| `A`           | 消息内容向上滚动 1 行 |
| `D`           | 消息内容向下滚动 1 行 |
| `y` / `Enter` | 复制选中消息到剪切板   |
| `Esc`         | 返回对话模式       |

### 归档对话功能

**归档（Ctrl+L）**：

- 保存到归档目录 `~/.j/chat/archives/`
- 默认名称：`archive-YYYY-MM-DD`
- 同名已存在时自动添加后缀

**还原（Ctrl+R）**：

- 进入归档列表选择
- `d` 删除归档
- `Enter` 还原归档

## 安装 & 更新

### 一键安装

```bash
curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh
```

### 从 crates.io 安装

```bash
cargo install j-cli
```

### 更新

```bash
# 一键更新
curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh

# 从 crates.io 更新
cargo install j-cli
```

## 卸载

```bash
# 一键卸载
curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh -s -- --uninstall

# cargo 安装的用户
cargo uninstall j-cli

# 手动删除
sudo rm /usr/local/bin/j  # 一键安装方式
rm ~/.cargo/bin/j          # cargo 安装方式

# 彻底清理（可选）
rm -rf ~/.jdata
```
