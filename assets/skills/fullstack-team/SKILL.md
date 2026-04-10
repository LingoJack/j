---
name: fullstack-team
description: 多 Agent 协作全栈应用开发；当用户需要多个 agent 协作构建完整前后端应用时触发此技能
---

# fullstack-team

多 Agent 协作全栈应用开发技能。使用 Teammate 系统组建开发团队，各 agent 并行工作、通过消息协调。

## 触发条件

当用户要求:
- 多个 agent 协作开发
- 构建全栈应用（前端 + 后端）
- 团队协作模式
- 并行开发前后端

## 工作流

### 1. 需求分析

先理解用户需求，确定技术栈和架构。默认推荐:
- 前端: React + TypeScript
- 后端: Express/FastAPI/Go（根据用户偏好）
- 数据库: 根据需求选择

### 2. 创建团队

使用 `AgentTeam` 工具批量创建 teammate:

```json
{
  "members": [
    {
      "name": "Frontend",
      "role": "前端开发者，精通 React + TypeScript",
      "prompt": "你负责前端开发。请根据以下需求创建 React 应用:\n\n[需求描述]\n\n技术要求:\n- React + TypeScript\n- 组件放在 src/components/\n- API 调用放在 src/api/\n- 使用 fetch 调用后端 API\n\n开始前先用 SendMessage 工具通知 @Backend 你需要的 API 接口格式。完成后通知 @Main。"
    },
    {
      "name": "Backend",
      "role": "后端开发者，精通 Node.js/Express",
      "prompt": "你负责后端开发。请根据以下需求创建后端 API:\n\n[需求描述]\n\n技术要求:\n- Express + TypeScript\n- 路由放在 server/routes/\n- 模型放在 server/models/\n- 提供 RESTful API\n\n开始前先等待 @Frontend 的接口需求，或主动设计 API 并通知 @Frontend。完成后通知 @Main。"
    }
  ]
}
```

### 3. 协调工作

作为 Main agent（团队协调者），你的职责:
- 监控各 teammate 的进度（通过聊天室中的消息）
- 解决 teammate 之间的冲突或疑问
- 确保前后端接口对齐
- 在所有 teammate 完成后进行集成测试

### 4. 协作约定

Teammate 之间的沟通规范:
- 使用 `SendMessage` 工具发送消息
- 用 `@AgentName` 指定接收者
- 接口定义变更时必须通知对方
- 完成任务后通知 `@Main`

### 5. 文件组织

推荐项目结构:
```
project/
├── src/                    # 前端（Frontend 负责）
│   ├── components/
│   ├── api/
│   ├── App.tsx
│   └── main.tsx
├── server/                 # 后端（Backend 负责）
│   ├── routes/
│   ├── models/
│   ├── middleware/
│   └── index.ts
├── package.json
└── README.md
```

**重要**: 前后端使用不同的目录，避免文件编辑冲突。

## 注意事项

- 每个 Teammate 有独立的 LLM 连接和上下文，互不干扰
- 同一时刻只允许一个 Agent 编辑同一个文件（自动互斥锁）
- Teammate 是 session 级别的，关闭聊天后自动清理
- Teammate 空闲超过 2 分钟会自动退出
