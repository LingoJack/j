---
name: webapp-gen
description: 完整前后台 Web 应用生成技能包；1. 当用户需要快速生成一个 Web 应用时，触发此技能
---

# webapp-gen

全栈 Web 应用快速生成工具，完整的从需求到上线的工作流自动化。

技术栈: React + TypeScript (前端) | Go + Gorm + Gin + MySQL (后端)

## STEP 0：初始化项目（强制前置，必须在一切之前执行）

**在创建任何任务、写任何文档之前**，先完成本步骤。后续所有步骤（`docs/requirement.md`、`docs/frontend_design.md`、前后端编码）都依赖这一步生成的目录结构，跳过会直接失败。

执行流程：
1. 从用户一句话需求中提炼英文 kebab-case 项目名（例如"做个博客系统" → `blog-system`）。如无法确定，用 `Ask` 工具向用户确认项目名后再继续。
2. 在当前工作目录执行：
   ```bash
   mkdir <project_name> && cd <project_name> && git clone https://github.com/LingoJack/proj_template.git .
   ```
3. 验证初始化成功：确认当前目录下存在 `backend/`、`frontend/`、`Makefile`。若不存在，必须停下来排查，不得继续后续步骤。
4. **后续所有工作都在 `<project_name>/` 目录内进行**（包括写 `docs/*.md`）。

会创建基于 react + ts + tailwindcss v4 + go 的项目脚示例项目
目录结构如：
```bash
➜  proj_template git:(main) ✗ tree . -I node_modules
.
├── backend
│   ├── cmd
│   │   └── server
│   │       ├── main.go
│   │       ├── wire_gen.go
│   │       └── wire.go
│   ├── config
│   │   ├── config.go
│   │   └── config.yaml
│   ├── controller
│   │   ├── health_test.go
│   │   ├── health.go
│   │   └── post_controller.go
│   ├── Dockerfile
│   ├── docs
│   │   └── docs.go
│   ├── go.mod
│   ├── go.sum
│   ├── middleware
│   │   ├── auth.go
│   │   ├── cors.go
│   │   ├── logger.go
│   │   ├── passthrough.go
│   │   ├── rate_limit.go
│   │   ├── recover.go
│   │   └── request_id.go
│   ├── model
│   │   └── post.go
│   ├── pkg
│   │   ├── database
│   │   │   └── database.go
│   │   ├── logger
│   │   │   └── logger.go
│   │   ├── response
│   │   │   └── response.go
│   │   └── validator
│   │       └── validator.go
│   ├── repository
│   │   ├── post_repository_test.go
│   │   └── post_repository.go
│   ├── router
│   │   └── router.go
│   ├── service
│   │   ├── post_service_test.go
│   │   └── post_service.go
│   └── tool
│       ├── aes.go
│       ├── chinese_to_letter.go
│       ├── concurrent.go
│       ├── conf
│       │   └── conf_loader.go
│       ├── copy.go
│       ├── cos.go
│       ├── custom.go
│       ├── encode.go
│       ├── env.go
│       ├── file.go
│       ├── format.go
│       ├── hash.go
│       ├── id.go
│       ├── ip.go
│       ├── json_fix.go
│       ├── json_schema.go
│       ├── jwt.go
│       ├── llm_json_extract.go
│       ├── ptr.go
│       ├── snowflask.go
│       ├── str.go
│       └── template_render.go
├── docker-compose.yml
├── frontend
│   ├── dist
│   │   ├── assets
│   │   │   ├── index-CiJpUzvu.css
│   │   │   └── index-vvgvxU9P.js
│   │   ├── favicon.svg
│   │   ├── icons.svg
│   │   └── index.html
│   ├── Dockerfile
│   ├── eslint.config.js
│   ├── index.html
│   ├── nginx.conf
│   ├── package-lock.json
│   ├── package.json
│   ├── public
│   │   ├── favicon.svg
│   │   └── icons.svg
│   ├── README.md
│   ├── src
│   │   ├── api
│   │   │   ├── client.ts
│   │   │   └── posts.ts
│   │   ├── App.tsx
│   │   ├── assets
│   │   │   ├── hero.png
│   │   │   ├── react.svg
│   │   │   └── vite.svg
│   │   ├── components
│   │   │   └── Layout.tsx
│   │   ├── hooks
│   │   ├── index.css
│   │   ├── main.tsx
│   │   ├── pages
│   │   │   ├── Home.tsx
│   │   │   └── Posts.tsx
│   │   ├── stores
│   │   │   └── postStore.ts
│   │   └── types
│   │       ├── api.ts
│   │       └── post.ts
│   ├── tsconfig.app.json
│   ├── tsconfig.json
│   ├── tsconfig.node.json
│   └── vite.config.ts
└── Makefile
```
  

## 初始化工作流

**重要**：先完成上面的 **STEP 0 项目初始化**，再使用 <Task> (action='create') 工具创建以下任务。**任务清单的第一项必须是"项目初始化"**，缺失就是流程错误。

预期工作流为：
```
项目初始化（git clone 模板，见 STEP 0，必须第一项）
需求分析
api 设计
前端设计
原型生成
原型反馈
根据最终原型完善api设计和前端设计
开始后端设计
后端编码实现
前端编码实现
```

> 开始任何写文件动作前，先 `pwd` 或 `ls` 确认当前已在 `<project_name>/` 目录下，且存在 `backend/`、`frontend/`。否则回到 STEP 0。

### 需求分析阶段

进入 PLAN MODE
根据用户的一句话需求进行详细扩写
需求文档必须包含如下内容：
- 标题：需求名称
- 一些预期的用例场景 Use Case
- 非功能性需求
直到用户认为计划完善
将任务输出到 docs/requirement.md


### api设计阶段
开发原则
- 虽然数据是 mock 的，但必须遵守 "数据从 API 中来" 的规律
- 原型必须满足「闭环理论」

**闭环理论**：
- 数据的产生和消费的逻辑成对存在，不存在只有生产、无消费或只有消费、无生产的数据
- 非特定情况，程序不能存在出现过但是无法触发的交互元素（如：不能点击或点击了无反应的按钮等）


### 前端设计阶段
阅读需求文档 `docs/requirement.md` 内容，严格按照以下步骤开发：
进入 PLAN MODE，首先思考计划清楚以下内容
- 确定项目结构
- 列出页面组件
- 定义状态管理方案
- 选择 UI 库
- 确定路由规划
- 列出 API 接口定义
- 定义全局样式规范
- 确定响应式断点
- 确定主题色板
- 确定图标系统
- 确定组件库规范


### 原型生成阶段
按照 `docs/frontend_design.md` 实现原型，接口返回的数据可以先 mock，注意必须是 mock 接口返回的数据

运行以下命令检查前端项目构建
```bash
npx tsc --noEmit 2>& 1
make check-frontend
```


### 原型反馈阶段

若检查通过，使用 `Bash` 后台运行 `make run-frontend` 启动开发服务器
用 `Ask` 询问改进意见并按照用户要求优化，直到用户确认该原型满足要求
根据最终原型完善api设计和前端设计


### 后端设计阶段
根据原型和需求文档，以及前期设计的接口文档，设计数据表
写到 `docs/backend-design.md` 文档
通过 `@skill:sql-to-go-struct-and-dao` 生成 model 和 dao 层代码


### 后台开发阶段
实现后台接口以及逻辑


### 前端开发阶段
原型修改，替换为调用实际的后台接口