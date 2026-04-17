# 各阶段详细指导

工作流 10 阶段（STEP 0 已在 SKILL.md）。每阶段执行前先 `pwd` 确认在 `<project_name>/` 内。

## 2. 需求分析

1. 进入 PLAN MODE
2. 把用户一句话需求扩写为完整需求文档，至少覆盖：
   - 标题：需求名称
   - 用例场景（Use Cases）
   - 非功能性需求
3. 反复与用户确认直到计划完善
4. 写入 `docs/requirement.md`

## 3. API 设计

按需求梳理 REST 接口清单。原则：
- 接口语义闭环：每个数据都有生产（写接口）和消费（读接口）
- 响应体结构统一（参考模板 `backend/pkg/response/response.go`）
- 记下每个接口的 method、path、入参、出参，供后端设计和前端 mock 共用

## 4. 前端设计

阅读 `docs/requirement.md`，PLAN MODE 下先把以下内容想清楚再写代码：

- 项目结构（模板已给出基础结构，在 `frontend/src/`）
- 页面组件清单
- 状态管理方案（模板已内置 zustand，见 `frontend/src/stores/`）
- UI 库选型
- 路由规划
- API 接口定义（与阶段 3 对齐）
- 全局样式规范 / 响应式断点 / 主题色板 / 图标系统 / 组件库规范

产出写入 `docs/frontend_design.md`。

## 5. 原型生成

按 `docs/frontend_design.md` 实现页面，接口数据 **mock 成"从接口返回"的形态**，不要硬编码到组件里。

构建检查：

```bash
npx tsc --noEmit 2>&1
make check-frontend
```

有错必须改到过为止。

## 6. 原型反馈

构建通过后，用 `Bash` 后台跑 `make run-frontend` 起开发服务器，再用 `Ask` 征求用户意见，迭代直到确认。定稿后**回写** `docs/frontend_design.md` 和 API 设计文档，保证三者一致。

## 7. 后端设计

根据原型、需求、接口文档设计数据表，写入 `docs/backend-design.md`。表结构写成 `CREATE TABLE` 语句，然后通过 `@skill:sql-to-go-struct-and-dao` 生成 model 与 DAO 层代码。

生成完成后：
- 按生成代码的约定把 DAO 接入模板 `backend/repository/` 层
- 生成代码里 `Database()` 返回的占位符改成真实逻辑库名（默认用 `appdb`）

## 8. 后端编码

实现 controller / service / repository 逻辑。连数据库跑测试前必须先起 MySQL 容器，且**改 `backend/config/config.yaml` 里的 DSN**（模板默认值连不上）。完整步骤见 [backend-mysql.md](backend-mysql.md)。

迭代命令：
- 本地跑：`make run-backend`
- 单测：`make podman-mysql-up && make test-backend`

## 9. 前端编码

把阶段 5 的 mock 调用替换成真实接口。API client 在 `frontend/src/api/client.ts`，按模板约定扩展。

## 10. 容器化启动与验收

最终全栈验收，见 [backend-mysql.md](backend-mysql.md) 的"最终全栈验收"节。
