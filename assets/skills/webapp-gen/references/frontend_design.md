# 原型设计阶段

## 技术栈
前端技术栈
- React
- Typescript
- tailwindcss (v4)
使用 `make init-frontend APP=<app_name>` 自动创建项目脚手架，并安装依赖

## 流程要求
阅读需求文档 `docs/requirement.md` 内容，严格按照以下步骤开发：


使用 `TodoWrite` 跟踪以下待办依次执行：
### 计划阶段
编写 `docs/frontend_design.md` 首先思考计划清楚以下内容
- 确定项目结构
- 列出页面组件
- 定义状态管理方案
- 选择 UI 库
- 确定路由规划
- 列出 API 接口定义
- 定义全局样式规范
- 确定响应式断点
- 确定主题色板
- 确定图标系统
- 确定组件库规范

编写完成后，运行以下命令以打开前端设计文档供用户查看，并使用 `Ask` 工具收集反馈
```bash
j code docs/frontend_design.md
```

收集用户反馈后，修改 `docs/frontend_design.md` 并再次运行上述命令，直到用户确认该文档满足要求

### 原型开发阶段

创建 `frontend/` 目录，在目录下执行
```bash
make init-frontend APP=<app_name>
```

按照 `docs/frontend_design.md` 实现原型，接口返回的数据可以先 mock，注意必须是 mock 接口返回的数据

运行以下命令检查前端项目构建
```bash
make check-frontend
```

若检查通过，使用 `BackgroundRun` 运行 `make dev-frontend` 启动开发服务器

用 `Ask` 询问改进意见并按照用户要求优化，直到用户确认该原型满足要求

## 开发原则
- 虽然数据是 mock 的，但必须遵守 "数据从 API 中来" 的规律
- 原型必须满足「闭环理论」

**闭环理论**：
- 数据的产生和消费的逻辑成对存在，不存在只有生产、无消费或只有消费、无生产的数据
- 非特定情况，程序不能存在出现过但是无法触发的交互元素（如：不能点击或点击了无反应的按钮等）