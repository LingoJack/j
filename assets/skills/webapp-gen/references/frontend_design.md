使用 `init_frontend.sh` 脚本快速初始化 React + TypeScript 项目（自动创建项目脚手架，并安装依赖）


阅读需求文档 `docs/requirement.md` 内容，严格按照以下步骤开发：
1. 创建 `frontend/` 目录，在目录下执行
    ```bash
    <skill_base_path>/scripts/init_frontend.sh <app_name>
    ```
2. 按照用户的要求实现原型，数据可以先 mock
3. 运行以下脚本检查前端项目
    ```bash
    <skill_base_path>/scripts/check_frontend.sh
    ```
4. 使用 `BackgroundRun` 运行 `npm run dev` 启动开发服务器
5. 用 `Ask` 询问改进意见并按照用户要求优化，直到用户确认该原型满足要求