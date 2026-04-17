---
name: webapp-gen
description: 从一句话需求生成完整前后台 Web 应用的工作流技能，技术栈固定为 React + TypeScript + Tailwind v4（前端）与 Go + Gin + Gorm + MySQL（后端），基于 github.com/LingoJack/proj_template 模板。触发场景：用户说"做个 XX 系统/网站/平台"、"生成一个 Web 应用"、"搭一个带前后端的 XXX"。不要用在：只要前端静态页 / 只要后端 API / 非此栈（Next.js、Vue、Python、Java…）/ 给既有项目加功能 的场景。
---

# webapp-gen

## STEP 0 — 初始化项目（强制前置）

在做任何分析、建任何任务、写任何文档之前先做这步。后面所有步骤都依赖这里生成的目录结构。

1. 从用户需求提炼英文 kebab-case 项目名（"做个博客系统" → `blog-system`）。拿不准就用 `Ask` 问用户。
2. 在当前工作目录执行：
   ```bash
   mkdir <project_name> && cd <project_name> && git clone https://github.com/LingoJack/proj_template.git .
   ```
3. 验证：`ls` 应看到 `backend/`、`frontend/`、`Makefile`、`docker-compose.yml`。缺任何一个都要停下排查，不得继续。
4. 后续所有文件操作都在 `<project_name>/` 内。写文件前先 `pwd` 确认。

> 需要了解模板具体目录布局时，跑 `ls backend/ frontend/` 现场看，**不要**凭假设写代码。

## 工作流总览

完成 STEP 0 后，用 `Task` (action='create') 建任务清单，**第一项必须是"项目初始化"**，顺序固定为：

```
1. 项目初始化                  ← STEP 0，必须第一项
2. 需求分析                    → docs/requirement.md
3. API 设计
4. 前端设计                    → docs/frontend_design.md
5. 原型生成                    （mock 数据）
6. 原型反馈                    （用户确认后回写设计文档）
7. 后端设计                    → docs/backend-design.md，用 @skill:sql-to-go-struct-and-dao 生 model/DAO
8. 后端编码                    （需 MySQL — 见 references/backend-mysql.md）
9. 前端编码                    （接入真实 API）
10. 容器化启动与验收            （podman compose 全栈）
```

每一阶段的具体产出物、检查点、命令见 **[references/phases.md](references/phases.md)**。

## 容器化三原则（全局贯穿）

- **编码期本地跑**：`make run-frontend` / `make run-backend`，别每次改代码都 rebuild 镜像
- **依赖服务单容器**：后端要 MySQL 时只起 mysql 一个，backend 仍 `go run`（见 [references/backend-mysql.md](references/backend-mysql.md)）
- **全栈容器化只在最终验收**：`podman compose up -d --build`
- **运行时统一 podman**：compose 文件是标准 v3，`podman compose` 原生兼容。**不要跑 `make docker-up`**（那是 `docker compose` 版本），一律用 `make podman-*` 目标

## 设计原则

**闭环理论**（贯穿 API 设计、原型、编码三个阶段）：
- 数据的生产和消费成对存在，不能只产不消、只消不产
- 程序不能出现"按了没反应"或"无法触发"的交互元素

**数据从 API 来**：原型阶段数据可以 mock，但必须 mock 成"从接口返回"的形态，不能把数据硬编码在组件里。

## 详细指导

- 各阶段产出与检查点：[references/phases.md](references/phases.md)
- MySQL 容器启动、DSN 改写陷阱、最终部署验收：[references/backend-mysql.md](references/backend-mysql.md)
