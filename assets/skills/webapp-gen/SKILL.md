---
name: webapp-gen
description: "一句话生成完整 Web APP。当用户描述一个 Web 应用需求时触发"
---

# webapp-gen

全栈 Web 应用快速生成工具，完整的从需求到上线的工作流自动化。

技术栈: React + TypeScript (前端) | Go + Gorm + Gin + MySQL (后端)

## 初始化工作流

使用 <Task> 工具创建以下任务，任务内容分别为（只在任务进行的时候才阅读文档内容）：
1. 阅读 references/requirement_analysis.md 指引，完成需求分析
2. 阅读 references/frontend_design.md 指引，完成前端设计，通过多轮迭代原型反推需求及接口设计
3. 阅读 references/api_design.md 指引，根据满足用户需求的原型，完成 API 设计文档，供后续参考
4. 阅读 references/backend_design.md 指引，完成后端设计，设计数据库表结构设计和接口实现细节、服务划分
5. 阅读 references/backend_impl.md 指引，完成后端实现，根据 API 设计文档以及后台设计文档实现后端服务
6. 阅读 references/frontend_impl.md 指引，完成前端实现，根据前端设计文档以及 API 设计文档、后台接口的具体实现来实现前端页面，替换 mock 的接口数据
7. 阅读 references/testing.md 指引，完成测试配置
8. 阅读 references/deployment.md 指引，完成部署

注意，上述所有任务是串行的（所有任务仅依赖上一个任务），前一个任务都必须完成才能继续下一个任务