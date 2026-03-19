---
name: webapp-gen
description: "一句话生成完整 Web APP。当用户描述一个 Web 应用需求时触发"
---

# webapp-gen

## 初始化工作流

技术栈固定为： React + Typescript + Go + Gorm + Gin + MySQL

使用 `Task` 工具初始化工作流：

创建项目根目录 `<project_name>/`

- 需求扩写

  - 创建 `docs/` 目录（如无）
  - 根据用户的一句话需求扩写
  - 向用户确认，使用 `Ask` 询问用户反馈，修改
  - 产出需求文档到 `docs/requirement.md`

- 创建前端原型

  - 创建 `frontend/` 目录

      ```bash
      # 防止交互式阻塞
      echo "" | npx -y create-vite <project_name> --template react-ts
      cd <project_name>
      npm install
      ```

  - 实现前端原型，数据 mock

  - 检查项目

      ```bash
      npm run build
      ```

  - `BackgroudRun` 运行项目

  - `Ask` 用户询问改进意见，直到用户认为此原型满足要求，才可以进入下一步

- 后端设计文档梳理

  - 划分不同的 Service 服务
  - 梳理 Service 服务的数据实体
  - 梳理出实现「前端原型」需要的 API，划归到对应的服务
  - 梳理 API 内部应该实现的逻辑
  - 整理出一份文档：
    - SQL 建表语句 `docs/backend/schema.sql`
    - 服务划分，每个服务单独一份 `docs/backend/service/<service_name>.md` ，包含该服务下的 API 协议，以及内部的逻辑

- 后台编码实现

  - 创建目录 `backend/`
  - 通过自动化工具 `jen` 完整项目的 dao 层代码生成
  - 根据 `docs/backend/service/<service_name>.md` 实现每个服务的 API

- 测试与验证

  - `BackgroudRun` 运行项目

## 工具箱

- 生成 React + TS 的代码脚手架：
