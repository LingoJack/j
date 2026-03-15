---
name: webapp-gen
description: "一句话生成完整 Web APP。当用户描述一个 Web 应用需求时触发（如：做一个商城、做一个博客、做一个管理后台）。覆盖完整流程：需求对齐 → 模块划分 → 前端页面生成（Mock 数据，即时预览） → 用户确认 → 后端实现 → 容器化部署 → 文档交付。技术栈：Go + React + TailwindCSS + MySQL，前后端分离。"
---

# Web APP 生成器

技术栈：Go（后端）+ React + TailwindCSS（前端）+ MySQL，前后端分离。

核心原则：**前端先行，即时反馈**。先用 Mock 数据生成可运行的前端页面，用户确认满意后再生成后端实现。

## 项目目录规范

**所有生成的 Web APP 项目统一存放在 `~/jcli_playground/` 目录下：**

```
~/jcli_playground/
├── <project-name>/          # 每个项目的独立目录
│   ├── frontend/
│   ├── backend/
│   ├── docs/
│   ├── modules.yaml
│   ├── REQUIREMENTS.md
│   └── ...
├── another-project/
└── ...
```

项目名从用户需求中提取（如"做一个商城" → `shopping-mall`），如果目录已存在则询问用户是否覆盖。

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

然后初始化项目：

```bash
# 项目名从需求中提取（如"做一个商城" → shopping-mall）
PROJECT_NAME="<project-name>"
PROJECT_DIR="$HOME/jcli_playground/$PROJECT_NAME"

# 检查目录是否存在
if [ -d "$PROJECT_DIR" ]; then
  echo "目录已存在，请确认是否覆盖"
fi

mkdir -p "$PROJECT_DIR" && cd "$PROJECT_DIR" && git init
```

## Phase 2: 模块划分

将业务拆为自治模块。规范见 [references/module_design.md](references/module_design.md)。

1. 按业务域划分（如：用户、商品、订单），不按技术层划分
2. 定义每个模块的职责、核心实体、对外 API、模块间依赖
3. 写入 `modules.yaml`
4. 用 `ask` 让用户确认

确认后：

```bash
git add . && git commit -m "docs: 需求文档 + 模块划分"
```

## Phase 3: 前端生成（Mock 数据）

**这是核心阶段——用户即时反馈循环。**

React + TailwindCSS 规范见 [references/react_frontend.md](references/react_frontend.md)。

### 3.1 初始化前端项目

用 Vite 创建 React + TailwindCSS 项目：

```bash
npm create vite@latest frontend -- --template react && cd frontend && npm install
npm install -D tailwindcss @tailwindcss/vite
```

生成基础布局（导航栏、侧边栏、路由），配置 TailwindCSS。

### 3.2 按模块生成页面

对每个模块，生成对应的前端页面和组件。**所有数据使用 Mock**：

- 在 `src/mocks/` 下为每个模块创建 mock 数据文件
- 页面组件直接 import mock 数据渲染
- API 调用层（`src/api/`）暂时返回 mock 数据，但**接口签名与未来真实 API 一致**

### 3.3 用户反馈循环

每生成一个模块的页面后：

1. `shell` 启动开发服务器：`cd frontend && npm run dev`（后台运行）
2. 用 `ask` 告知用户访问地址（通常 http://localhost:5173），展示已生成的页面列表
3. 用 `ask` 问用户：

```
"请查看页面效果，是否满意？"
选项：
- 满意，继续
- 不满意，需要调整（请描述）
```

4. **满意** → `git add frontend/ && git commit -m "feat(frontend): <模块名>页面"`，继续下一个模块
5. **不满意** → 根据用户反馈修改代码，回到步骤 2 重新展示。如果改崩了，用 `git checkout -- frontend/` 恢复到上次 commit，重新生成

### 3.4 前端完整后

所有模块页面都确认 OK 后：

```bash
git add . && git commit -m "feat(frontend): 所有页面完成（Mock 数据）"
```

## Phase 4: 后端生成

**根据前端已确认的接口签名（`src/api/` 中的函数定义）反推后端实现。**

Go 后端规范见 [references/go_backend.md](references/go_backend.md)。

### 4.1 初始化后端项目

```bash
mkdir backend && cd backend && go mod init <module-name>
```

### 4.2 逐模块生成

对每个模块（按依赖拓扑序，先生成被依赖模块），用 `NewTask` 创建子任务：

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
  1. 执行 shell: cd backend && go test ./internal/<module>/...
  2. 全部通过 → 跳出循环
  3. 有失败 → 读取错误输出，分析原因，用 edit 修复代码，回到步骤 1
如果 5 次仍失败 → 用 ask 告知用户，请求指导
```

每个模块测试通过后立即提交：

```bash
git add backend/internal/<module>/ && git commit -m "feat(backend): <模块名>模块 — 模型/接口/测试"
```

## Phase 5: 前后端联调

### 5.1 替换 Mock 为真实 API

将前端 `src/api/` 中的 mock 返回替换为真实的 fetch 调用，后端地址通过环境变量 `VITE_API_BASE_URL` 配置。

### 5.2 联调验证

```
重复（最多 3 次）：
  1. shell 启动后端: cd backend && go run cmd/server/main.go &
  2. shell 启动前端: cd frontend && npm run dev &
  3. 用 shell + curl 验证核心接口
  4. 全部正常 → 跳出
  5. 有问题 → 读取错误日志，修复，回到步骤 1
```

```bash
git add . && git commit -m "feat: 前后端联调完成"
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
  1. shell: docker compose up --build -d
  2. shell: curl 健康检查接口
  3. 正常 → 跳出
  4. 失败 → 读 docker compose logs，修复，回到步骤 1
```

```bash
git add . && git commit -m "chore: 容器化部署配置"
```

## Phase 7: 文档交付

文档规范见 [references/doc_tracking.md](references/doc_tracking.md)。

生成：
1. `README.md` — 项目说明、快速启动、技术栈、目录结构
2. `docs/api.md` — 各模块 API 文档（接口、请求/响应示例）
3. `docs/architecture.md` — 架构图、模块依赖、数据流

```bash
git add . && git commit -m "docs: 项目文档"
```

## 关键约束

1. **前端先行**：先生成可运行的前端（Mock 数据），用户确认后再写后端
2. **每个确认点都可回退**：不满意就 `git checkout -- .` 恢复
3. **模块自治**：模块间通过 API 调用，不共享数据库表
4. **测试失败不跳过**：进入重试循环，最多 N 次后请求用户介入
5. **每个阶段结束必须 git commit**
6. **前端通过环境变量配置后端地址**，不硬编码
7. **生成的代码必须能直接运行**，不留 TODO 占位
