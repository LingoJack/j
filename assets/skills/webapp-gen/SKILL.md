---
name: webapp-gen
description: 完整前后台 Web 应用生成技能包；1. 当用户需要快速生成一个 Web 应用时，触发此技能
---

# webapp-gen

全栈 Web 应用快速生成工具，完整的从需求到上线的工作流自动化。

技术栈: React + TypeScript (前端) | Go + Gorm + Gin + MySQL (后端)

## 初始化项目，在工作目录执行
```bash
git clone https://github.com/LingoJack/proj_template.git .
```
会创建基于 react + ts + tailwindcss v4 + go 的项目脚手架
目录结构如：
```bash
➜  test_web_app git:(main) tree .
.
├── backend
│   └── go.mod
├── frontend
│   ├── eslint.config.js
│   ├── index.html
│   ├── package-lock.json
│   ├── package.json
│   ├── public
│   │   ├── favicon.svg
│   │   └── icons.svg
│   ├── README.md
│   ├── src
│   │   ├── App.css
│   │   ├── App.tsx
│   │   ├── assets
│   │   │   ├── hero.png
│   │   │   ├── react.svg
│   │   │   └── vite.svg
│   │   ├── index.css
│   │   └── main.tsx
│   ├── tsconfig.app.json
│   ├── tsconfig.json
│   ├── tsconfig.node.json
│   └── vite.config.ts
└── Makefile
```
  

## 初始化工作流

**重要**：使用 <Task> (action='create') 工具创建以下任务，任务内容分别为（只在任务进行的时候才阅读文档内容）：

任务列表如下：
1. 阅读 references/requirement_analysis.md 指引，完成需求分析
2. 阅读 references/api_design.md 指引，根据满足用户需求的原型，完成 API 设计文档，供后续参考
3. 阅读 references/frontend_design.md 指引，完成前端设计，通过多轮迭代原型反推需求及接口设计
4. 阅读 references/backend_design.md 指引，完成后端设计，设计数据库表结构设计和接口实现细节、服务划分
5. 阅读 references/backend_impl.md 指引，完成后端实现，根据 API 设计文档以及后台设计文档实现后端服务
6. 阅读 references/frontend_impl.md 指引，完成前端实现，根据前端设计文档以及 API 设计文档、后台接口的具体实现来实现前端页面，替换 mock 的接口数据

注意，上述所有任务是串行的（所有任务仅依赖上一个任务），前一个任务都必须完成才能继续下一个任务

创建完任务之后，开始逐步跟随指引严格执行，不可跳步