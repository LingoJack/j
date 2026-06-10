# Makefile Submodule 操作优化方案

## 当前痛点

1. **重复代码严重** — `commit-jstudio`、`push-jstudio`、`push-jstudio-non-ai` 中有大量重复逻辑（diff 检测、AI commit、主仓库指针更新）
2. **硬编码单一 submodule** — 所有命令只针对 jstudio，如果将来新增 submodule 需要大量复制
3. **缺少常用 submodule 操作**：
   - 没有 `pull` submodule（同步远程更新）
   - 没有 `diff` submodule（查看变更详情）
   - 没有 `log` submodule（查看提交历史）
   - 没有 `cd` 进入 submodule 的快捷方式
   - 没有 `sync` 全部 submodule 的命令
4. **命名不统一** — 有的用 `xxx-jstudio`，有的用 `jstudio-xxx`，且缺乏通用 `sm-` 前缀分组

## 优化方案

### 核心思路：定义通用的 submodule 操作宏/函数

新增一组 `sm-` 前缀的通用命令，接受 `DIR` 参数指定 submodule 路径：

```
make sm-status                   # 所有 submodule 状态总览
make sm-status DIR=apps/jstudio  # 单个 submodule 状态
make sm-diff                     # 所有 submodule diff
make sm-log                      # 所有 submodule 最近提交
make sm-pull                     # 拉取并更新所有 submodule
make sm-commit                   # 非 AI 提交（DIR 必填）
make sm-push                     # AI 提交+推送（DIR 必填）
make sm-push-quick               # 非 AI 提交+推送（DIR 必填）
make sm-update-pointer           # 更新主仓库 submodule 指针
make sm-cd                       # 打印 submodule 路径方便 cd
```

### 保留向后兼容

原有的 jstudio 特定命令保留，内部重定向到新的 `sm-` 通用命令：

```makefile
status-jstudio:      ## → make sm-status DIR=apps/jstudio
commit-jstudio:      ## → make sm-commit DIR=apps/jstudio
push-jstudio:        ## → make sm-push DIR=apps/jstudio
push-jstudio-non-ai: ## → make sm-push-quick DIR=apps/jstudio
```

### 具体变更

#### 1. 新增 `SUBMODULES` 变量
自动检测 `.gitmodules` 中所有 submodule 路径：
```makefile
SUBMODULES := $(shell git config --file .gitmodules --get-regexp path | awk '{print $$2}')
```

#### 2. 新增通用 `sm-*` 目标

| 命令 | 说明 |
|------|------|
| `sm-status` | 显示所有/单个 submodule 状态（git status + 当前 commit） |
| `sm-diff` | 显示 submodule 的 diff |
| `sm-log` | 显示 submodule 最近 5 条提交 |
| `sm-pull` | 拉取远程 submodule 更新（git submodule update --remote） |
| `sm-commit` | 非 AI 自动提交 + 更新主仓库指针 |
| `sm-push` | AI 生成 commit + push + 更新主仓库指针 |
| `sm-push-quick` | 非 AI 快速 push + 更新主仓库指针 |
| `sm-update-pointer` | 仅更新主仓库 submodule 指针 |
| `sm-cd` | 打印 submodule 绝对路径 |

#### 3. 提取公共逻辑为 Make 函数

- `sm_update_pointer` — 更新主仓库指针并提交
- `sm_auto_commit` — 非 AI 自动生成 commit message

#### 4. `.PHONY` 更新

新增所有 `sm-*` 目标到 `.PHONY`。

### 不变的部分

- jstudio 构建相关命令（`init-jstudio`、`dev-jstudio`、`build-jstudio`、`install-jstudio`、`clean-jstudio`）保持不变，这些是构建特定逻辑
- 非 submodule 相关的命令不变
