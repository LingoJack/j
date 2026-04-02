## 概述

日报/周报系统，支持快速记录、周报管理和 Git 同步。

## 基本命令

```bash
j report <内容>         # 快速写入日报
j report                # 打开 TUI 编辑器（预填历史+日期前缀）
j check [n]             # 查看最近 n 行（默认 3 行）
j check open            # 打开 TUI 编辑器编辑全文
j search <n|all> <关键词> [-f]  # 搜索周报（-f 模糊匹配）
```

## 周报管理 (reportctl)

```bash
j reportctl new [日期]      # 开启新的一周
j reportctl sync [日期]     # 同步周数和日期
j reportctl set-url <url>   # 设置 Git 仓库地址
j reportctl push [message]  # 推送到远程仓库
j reportctl pull            # 从远程仓库拉取
j reportctl open            # 打开 TUI 编辑器编辑全文
```

## 日报格式

```markdown
# Week1[2024-01-01 - 2024-01-07]
- 【01-01】 完成项目初始化
- 【01-02】 实现核心功能
- 【01-03】 代码审查和优化
```

## Git 同步

设置远程仓库后可自动同步：

```bash
# 首次设置
j reportctl set-url https://github.com/user/reports.git

# 推送到远程
j reportctl push "更新周报"

# 从远程拉取
j reportctl pull
```

## 配置文件

日报配置存储在两个位置：

| 文件 | 描述 |
|------|------|
| `~/.jdata/config.yaml` | 主配置（report_file_path、git_repo） |
| `<report_dir>/settings.json` | 周报元数据（week_num、last_day） |

## 自动周切换

当当前日期超过 `last_day` 时，写入日报会自动：
1. 生成新周标题 `# WeekN[开始日期 - 结束日期]`
2. 更新 week_num 和 last_day
