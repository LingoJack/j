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
