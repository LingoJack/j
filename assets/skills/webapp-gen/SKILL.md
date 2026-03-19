---
name: webapp-gen
description: "一句话生成完整 Web APP。当用户描述一个 Web 应用需求时触发"
---

# webapp-gen

全栈 Web 应用快速生成工具，完整的从需求到上线的工作流自动化。

技术栈: React + TypeScript (前端) | Go + Gorm + Gin + MySQL (后端)

## 初始化工作流

使用 `Task` 工具初始化工作流，创建项目根目录 `<project_name>/`

### 1. 需求分析与扩写

- 创建 `docs/` 目录（如无）
- 根据用户的一句话需求进行详细扩写
- 用 `Ask` 向用户确认，收集反馈并迭代
- 输出需求文档到 `docs/requirement.md`

### 2. 前端原型开发

#### 初始化前端项目

使用 `init_frontend.sh` 脚本快速初始化 React + TypeScript 项目：

```bash
<skill_path>/scripts/init_frontend.sh my-app
```

脚本功能：

- 自动创建 Vite + React + TypeScript 项目
- 安装所有依赖
- 参数验证和错误处理
- 清晰的日志输出

#### 开发原型

- 在 `frontend/<project_name>` 目录下实现 UI 原型
- 使用 Mock 数据进行本地开发
- 使用 `BackgroundRun` 运行 `npm run dev` 启动开发服务器

#### 构建验证

使用 `frontend_check.sh` 脚本进行构建检查：

```bash
cd frontend/<project_name>
<skill_path>/scripts/frontend_check.sh
```

脚本功能：

- 自动安装缺失的依赖
- 执行生产构建检查
- 显示构建输出大小
- 清晰的错误提示

#### 获取用户反馈

- 使用 `TaskOutput` 运行项目，展示给用户
- 用 `Ask` 询问改进意见
- 迭代优化直到用户认为原型满足要求

### 3. 后端架构设计

- Service 划分：根据功能域划分不同的微服务
- 数据建模：梳理每个 Service 的数据实体和关系
- API 设计：列出实现前端所需的所有 API 端点
- 业务逻辑：规划每个 API 的核心处理逻辑

#### 输出文档结构

```
docs/backend/
├── schema.sql                 # SQL 建表语句
└── service/
    ├── user-service.md        # 用户服务 API 和业务逻辑
    ├── product-service.md     # 产品服务 API 和业务逻辑
    └── order-service.md       # 订单服务 API 和业务逻辑
```

每个服务文档应包含：

- API 端点定义（请求/响应格式）
- 数据模型定义
- 业务逻辑描述
- 依赖关系

### 4. 后端实现

- 创建 `backend/` 目录
- 使用 `jen` 自动生成 DAO 层代码（数据库访问层）
- 根据 `docs/backend/service/<service_name>.md` 实现各 Service 的 API
- 遵循 Gin 框架的标准项目结构

### 5. 测试与上线

- 集成测试：前后端联调验证
- 使用 `TaskOutput` 运行项目进行端到端测试
- 修复发现的问题
- 准备上线

## 工具箱

### 已提供的脚本

| 脚本                  | 功能      | 用途                 |
|---------------------|---------|--------------------|
| `init_frontend.sh`  | 初始化前端项目 | 快速创建 React + TS 项目 |
| `frontend_check.sh` | 构建检查    | 验证前端能正常构建          |

### 待开发的脚本

- `init_backend.sh` - 初始化 Go 后端项目
- `gen_dao.sh` - 生成 DAO 层代码
- `dev_server.sh` - 启动开发服务器（前后端）
- `build_all.sh` - 完整项目构建

## 最佳实践

1. 逐步迭代：不要一次性实现所有功能，分阶段交付
2. 文档驱动：先写文档（需求、API、数据模型），再写代码
3. 即时反馈：经常向用户展示进度和中间成果
4. 错误处理：所有脚本都应包含良好的错误处理和日志输出
5. 验证构建：每个阶段都要验证代码能正常编译和运行
