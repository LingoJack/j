---
name: webapp-gen
description: "一句话生成完整 Web APP。当用户描述一个 Web 应用需求时触发"
---

# webapp-gen

全栈 Web 应用快速生成工具，完整的从需求到上线的工作流自动化。

技术栈: React + TypeScript (前端) | Go + Gorm + Gin + MySQL (后端)

## 初始化工作流

创建项目根目录 `<project_name>/`
使用 `Task` 工具初始化以下工作流并执行

### 1. 需求分析与扩写

- 创建 `docs/` 目录（如无）
- 根据用户的一句话需求进行详细扩写
- 用 `Ask` 向用户确认，收集反馈并迭代
- 输出需求文档到 `docs/requirement.md`

### 2. 前端原型开发

#### 初始化前端项目

使用 `init_frontend.sh` 脚本快速初始化 React + TypeScript 项目（自动创建项目脚手架，并安装依赖）：

```bash
<skill_path>/scripts/init_frontend.sh my-app
```

#### 开发原型

- 在 `frontend/<project_name>` 目录下实现 UI 原型
- 使用 Mock 数据进行本地开发
- 使用 `BackgroundRun` 运行 `npm run dev` 启动开发服务器

#### 构建验证

使用 `frontend_check.sh` 脚本进行构建检查（自动安装缺失的依赖，检查错误）：

```bash
cd frontend/<project_name>
<skill_path>/scripts/frontend_check.sh
```

#### 获取用户反馈

- 用 `Ask` 询问改进意见
- 迭代优化直到用户认为原型满足要求，方能继续下一步

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