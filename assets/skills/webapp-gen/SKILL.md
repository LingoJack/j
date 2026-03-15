---
name: webapp-gen
description: "一句话生成完整 Web APP。当用户描述一个 Web 应用需求时触发（如：做一个商城、做一个博客、做一个管理后台）。覆盖完整流程：需求对齐 → 模块划分 → 前端页面生成（Mock 数据，即时预览） → 用户确认 → 后端实现 → 容器化部署 → 文档交付。技术栈：Go + React + TailwindCSS + MySQL，前后端分离。"
---

# Web APP 生成器

技术栈：Go（后端）+ React + TailwindCSS（前端）+ MySQL，前后端分离。

核心原则：**前端先行，即时反馈**。先用 Mock 数据生成可运行的前端页面，用户确认满意后再生成后端实现。

## 项目目录规范

**所有生成的 Web APP 项目统一存放在 `~/jcli_playground/` 目录下。**

```
~/jcli_playground/
├── <project-name>/          # 每个项目独立目录
│   ├── frontend/
│   ├── backend/
│   ├── docs/
│   ├── modules.yaml
│   ├── REQUIREMENTS.md
│   └── ...
└── ...
```

项目名从用户需求中提取（如"做一个商城" → `shopping-mall`）。

**定义项目根目录变量，后续所有 shell 命令通过 `cwd` 参数指向该目录，不再使用 `cd`：**

```
PROJECT_DIR = ~/jcli_playground/<project-name>
```

## Shell 工具使用规范

本 Skill 中所有 `RunShell` 调用遵循以下规则：

1. **始终使用 `cwd` 参数**指定工作目录，不要用 `cd xxx &&` 拼接
2. **交互式命令加 `--yes`**（如 `npm create vite@latest ... --yes`），避免命令等待输入导致超时
3. **不要后台启动 dev server**（`npm run dev &` 会卡住），改用 `npm run build` 检查编译是否通过
4. **长时间命令设置合理 `timeout`**（如 `npm install` 设 60s，`docker compose up --build` 设 180s）

示例调用：

```json
{ "command": "npm install", "cwd": "~/jcli_playground/my-app/frontend", "timeout": 60 }
```

## 工作流总览

```
Phase 1: 需求对齐
Phase 2: 模块划分
Phase 3: 前端生成（Mock 数据） ←→ 用户反馈循环（满意 commit / 不满意 restore）
Phase 4: 后端生成 + 测试
Phase 5: 前后端联调（替换 Mock 为真实 API）
Phase 6: 容器化部署
Phase 7: 文档交付
```

严格按顺序执行。每个 Phase 的具体规范在 references/ 目录下，**执行到对应阶段时再读取**。

## Phase 1: 需求对齐

用 `ask` 工具确认（每次最多 3 个问题）：

1. 核心实体和业务流程
2. 用户角色和权限
3. MVP 边界：第一版必须有什么

将确认结果写入 `REQUIREMENTS.md`，用 `ask` 做最终确认。

然后初始化项目（项目名从需求中提取，如"做一个商城" → `shopping-mall`）：

```json
{ "command": "mkdir -p ~/jcli_playground/shopping-mall && git init", "cwd": "~/jcli_playground/shopping-mall" }
```

如果目录已存在，先用 `ask` 询问用户是否覆盖。

## Phase 2: 模块划分

将业务拆为自治模块。规范见 [references/module_design.md](references/module_design.md)。

1. 按业务域划分（如：用户、商品、订单），不按技术层划分
2. 定义每个模块的职责、核心实体、对外 API、模块间依赖
3. 写入 `modules.yaml`
4. 用 `ask` 让用户确认

确认后：

```json
{ "command": "git add . && git commit -m 'docs: 需求文档 + 模块划分'", "cwd": "PROJECT_DIR" }
```

## Phase 3: 前端生成（Mock 数据）

**这是核心阶段——用户即时反馈循环。**

React + TailwindCSS 规范见 [references/react_frontend.md](references/react_frontend.md)。

### 3.1 初始化前端项目

```json
{ "command": "npm create vite@latest frontend -- --template react --yes && cd frontend && npm install && npm install -D tailwindcss @tailwindcss/vite && npm install react-router-dom", "cwd": "PROJECT_DIR", "timeout": 60 }
```

生成基础布局（导航栏、侧边栏、路由），配置 TailwindCSS。

### 3.2 按模块生成页面

对每个模块，生成对应的前端页面和组件。**所有数据使用 Mock**：

- 在 `src/mocks/` 下为每个模块创建 mock 数据文件
- 页面组件直接 import mock 数据渲染
- API 调用层（`src/api/`）暂时返回 mock 数据，但**接口签名与未来真实 API 一致**

### 3.3 用户反馈循环

每生成一个模块的页面后：

1. 用 `shell` 执行编译检查，确认无语法/类型错误：

```json
{ "command": "npm run build", "cwd": "PROJECT_DIR/frontend", "timeout": 30 }
```

2. 如果编译失败 → 读取错误输出，修复代码，重新编译（最多 3 次）
3. 编译通过后，用 `ask` 告知用户：

```
"前端页面已生成并编译通过。请手动运行以下命令预览：
  cd PROJECT_DIR/frontend && npm run dev

查看后是否满意？"
选项：
- 满意，继续
- 不满意，需要调整（请描述）
```

4. **满意** → 提交：

```json
{ "command": "git add frontend/ && git commit -m 'feat(frontend): <模块名>页面'", "cwd": "PROJECT_DIR" }
```

5. **不满意** → 根据用户反馈修改代码，回到步骤 1。如果改崩了：

```json
{ "command": "git checkout -- frontend/", "cwd": "PROJECT_DIR" }
```

### 3.4 前端完整后

所有模块页面都确认 OK 后：

```json
{ "command": "git add . && git commit -m 'feat(frontend): 所有页面完成（Mock 数据）'", "cwd": "PROJECT_DIR" }
```

## Phase 4: 后端生成

**根据前端已确认的接口签名（`src/api/` 中的函数定义）反推后端实现。**

Go 后端规范见 [references/go_backend.md](references/go_backend.md)。

### 4.1 初始化后端项目

```json
{ "command": "mkdir -p backend && cd backend && go mod init <module-name>", "cwd": "PROJECT_DIR" }
```

### 4.2 逐模块生成

对每个模块（按依赖拓扑序，先生成被依赖模块）：

1. 数据模型（`internal/<module>/model.go`）— 结构体 + GORM 标签
2. Repository 层（`internal/<module>/repository.go`）— 数据访问
3. Service 层（`internal/<module>/service.go`）— 业务逻辑
4. Handler 层（`internal/<module>/handler.go`）— HTTP 接口（与前端 API 签名对齐）
5. 路由注册

### 4.3 测试 + 自动修复

测试策略见 [references/test_strategy.md](references/test_strategy.md)。

每个模块生成后执行：

```
重复（最多 5 次）：
  1. RunShell: { "command": "go test ./internal/<module>/...", "cwd": "PROJECT_DIR/backend", "timeout": 30 }
  2. 全部通过 → 跳出循环
  3. 有失败 → 读取错误输出，分析原因，用 edit 修复代码，回到步骤 1
如果 5 次仍失败 → 用 ask 告知用户，请求指导
```

每个模块测试通过后立即提交：

```json
{ "command": "git add backend/internal/<module>/ && git commit -m 'feat(backend): <模块名>模块'", "cwd": "PROJECT_DIR" }
```

## Phase 5: 前后端联调

### 5.1 替换 Mock 为真实 API

将前端 `src/api/` 中的 mock 返回替换为真实的 fetch 调用，后端地址通过环境变量 `VITE_API_BASE_URL` 配置。

### 5.2 联调验证

分别编译前后端，确认无错误：

```json
{ "command": "npm run build", "cwd": "PROJECT_DIR/frontend", "timeout": 30 }
{ "command": "go build ./cmd/server/...", "cwd": "PROJECT_DIR/backend", "timeout": 30 }
```

用 `ask` 告知用户手动启动前后端进行联调验证：

```
"前后端代码已生成并编译通过。请手动验证：
  终端 1: cd PROJECT_DIR/backend && go run cmd/server/main.go
  终端 2: cd PROJECT_DIR/frontend && npm run dev
  
验证核心接口是否正常工作。"
```

编译通过后提交：

```json
{ "command": "git add . && git commit -m 'feat: 前后端联调完成'", "cwd": "PROJECT_DIR" }
```

## Phase 6: 容器化部署

部署规范见 [references/docker_deploy.md](references/docker_deploy.md)。

生成：
1. `backend/Dockerfile`（多阶段构建）
2. `frontend/Dockerfile`（构建 + nginx）
3. `docker-compose.yaml`（前端 + 后端 + MySQL）
4. `.env.example`

验证：

```
重复（最多 3 次）：
  1. RunShell: { "command": "docker compose up --build -d", "cwd": "PROJECT_DIR", "timeout": 180 }
  2. RunShell: { "command": "sleep 5 && curl -s http://localhost:8080/api/health", "cwd": "PROJECT_DIR", "timeout": 15 }
  3. 正常 → 跳出
  4. 失败 → RunShell: { "command": "docker compose logs --tail=50", "cwd": "PROJECT_DIR" } → 分析日志，修复，回到步骤 1
```

```json
{ "command": "git add . && git commit -m 'chore: 容器化部署配置'", "cwd": "PROJECT_DIR" }
```

## Phase 7: 文档交付

文档规范见 [references/doc_tracking.md](references/doc_tracking.md)。

生成：
1. `README.md` — 项目说明、快速启动、技术栈、目录结构
2. `docs/api.md` — 各模块 API 文档（接口、请求/响应示例）
3. `docs/architecture.md` — 架构图、模块依赖、数据流

```json
{ "command": "git add . && git commit -m 'docs: 项目文档'", "cwd": "PROJECT_DIR" }
```

## 关键约束

1. **前端先行**：先生成可运行的前端（Mock 数据），用户确认后再写后端
2. **每个确认点都可回退**：不满意就 `git checkout -- .` 恢复
3. **模块自治**：模块间通过 API 调用，不共享数据库表
4. **测试失败不跳过**：进入重试循环，最多 N 次后请求用户介入
5. **每个阶段结束必须 git commit**
6. **前端通过环境变量配置后端地址**，不硬编码
7. **生成的代码必须能直接运行**，不留 TODO 占位
8. **所有 shell 命令使用 `cwd` 参数**，不用 `cd xxx &&` 拼接
9. **不后台启动 dev server**，用 `npm run build` 检查编译，让用户手动预览
10. **交互式命令加 `--yes`**，避免等待输入超时
