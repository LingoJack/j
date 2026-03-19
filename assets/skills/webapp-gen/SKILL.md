---
name: webapp-gen
description: "一句话生成完整 Web APP。当用户描述一个 Web 应用需求时触发"
---

# webapp-gen

全栈 Web 应用快速生成工具，完整的从需求到上线的工作流自动化。

技术栈: React + TypeScript (前端) | Go + Gorm + Gin + MySQL (后端)

## 初始化工作流

使用 `Task` 工具初始化创建以下任务，任务内容分别为：
1. 阅读 references/requirement_analysis.md 指引，完成需求分析
2. 阅读 references/requirement_analysis.md 指引，完成需求分析
3. 阅读 references/requirement_analysis.md 指引，完成需求分析


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