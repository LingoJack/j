# 笔记本基础

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

> 笔记存储路径: `~/.jdata/notebook/`，支持子目录组织
> 支持对 notebook 内笔记和外部文件统一使用内置 Markdown 编辑器