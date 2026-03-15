# 模块划分规范

> `modules.yaml` 位于项目根目录 `PROJECT_DIR/modules.yaml`。

## 划分原则

1. **按业务域划分**，不按技术层。一个"订单模块"包含订单相关的模型、接口、逻辑，而不是把所有模型放一个模块、所有接口放另一个模块
2. **单一职责**：每个模块只负责一个业务域的数据一致性
3. **模块自治**：拥有自己的数据库表，不直接读写其他模块的表
4. **显式依赖**：模块间通过 API 调用，依赖关系在 modules.yaml 中声明

## modules.yaml 格式

```yaml
project:
  name: "my-shop"
  description: "电商平台"

modules:
  user:
    description: "用户注册、登录、鉴权、个人信息管理"
    entities:
      - User
      - Role
    apis:
      - "POST /api/users/register"
      - "POST /api/users/login"
      - "GET /api/users/me"
      - "PUT /api/users/me"
    dependencies: []

  product:
    description: "商品 CRUD、分类、搜索"
    entities:
      - Product
      - Category
    apis:
      - "GET /api/products"
      - "GET /api/products/:id"
      - "POST /api/products"
      - "PUT /api/products/:id"
      - "DELETE /api/products/:id"
    dependencies: []

  order:
    description: "购物车、下单、订单状态管理"
    entities:
      - Order
      - OrderItem
      - Cart
    apis:
      - "POST /api/orders"
      - "GET /api/orders"
      - "GET /api/orders/:id"
      - "POST /api/cart/items"
      - "GET /api/cart"
      - "DELETE /api/cart/items/:id"
    dependencies:
      - user
      - product
```

## 划分示例

| 业务场景 | 推荐模块划分 |
|---------|------------|
| 电商平台 | user, product, order, payment, admin |
| 博客系统 | user, post, comment, category |
| 后台管理 | user, role, menu, log |
| 在线教育 | user, course, enrollment, payment |

## 依赖拓扑

模块按依赖关系形成有向无环图（DAG）。生成代码时按拓扑序执行：先生成没有依赖的模块，再生成有依赖的模块。

示例（电商）：

```
user, product（无依赖，可并行）→ order（依赖 user + product）→ payment（依赖 order）
```
