# 文档化追踪规范

> 所有 shell 命令通过 `cwd` 参数指定工作目录，不使用 `cd`。

## Git 提交规范

每个阶段结束后必须 commit，提交信息格式：

```
<type>(<scope>): <中文描述>
```

type 取值：
- `feat` — 新功能/新模块
- `fix` — 修复
- `docs` — 文档
- `chore` — 构建/部署配置
- `test` — 测试

scope 取值：模块名（如 `user`, `product`, `frontend`）或 omit。

示例：

```
feat(user): 用户模块 — 注册/登录/鉴权
feat(frontend): 商品列表和详情页面
fix(order): 修复库存扣减并发问题
docs: 项目文档 — README/API/架构
chore: Docker 容器化部署配置
```

## 每个阶段的 commit 节点

| 阶段 | commit 时机 | 示例 |
|------|-----------|------|
| Phase 2 完成 | 需求 + 模块划分 | `docs: 需求文档 + 模块划分` |
| Phase 3 每个模块 | 前端页面用户确认后 | `feat(frontend): 商品页面` |
| Phase 4 每个模块 | 后端测试通过后 | `feat(product): 商品模块` |
| Phase 5 完成 | 联调通过 | `feat: 前后端联调完成` |
| Phase 6 完成 | 部署验证通过 | `chore: 容器化部署配置` |
| Phase 7 完成 | 文档生成 | `docs: 项目文档` |

所有 git 命令通过 `cwd` 指定项目根目录：

```json
{ "command": "git add . && git commit -m 'feat(frontend): 商品页面'", "cwd": "PROJECT_DIR" }
```

## 回退机制

用户不满意时用 git 回退：

```json
{ "command": "git checkout -- .", "cwd": "PROJECT_DIR" }
{ "command": "git reset --soft HEAD~1", "cwd": "PROJECT_DIR" }
```

## 生成的文档清单

### README.md

```markdown
# <项目名>

<一句话描述>

## 技术栈

- 前端：React + TailwindCSS + Vite
- 后端：Go + Gin + GORM
- 数据库：MySQL 8.0
- 部署：Docker Compose

## 快速启动

### Docker 方式（推荐）

\`\`\`bash
cp .env.example .env
docker compose up --build -d
\`\`\`

访问：http://localhost

### 手动启动

\`\`\`bash
# 后端
cd backend && cp .env.example .env && go run cmd/server/main.go

# 前端
cd frontend && npm install && npm run dev
\`\`\`

## 目录结构

<按实际生成的结构填写>

## 模块说明

<从 modules.yaml 生成>
```

### docs/api.md

按模块列出所有接口：

```markdown
# API 文档

## 用户模块

### POST /api/users/register

注册新用户。

**请求体：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| username | string | 是 | 用户名 |
| email | string | 是 | 邮箱 |
| password | string | 是 | 密码 |

**响应示例：**

\`\`\`json
{
  "code": 0,
  "message": "ok",
  "data": { "id": 1, "username": "testuser", "email": "test@example.com" }
}
\`\`\`
```

### docs/architecture.md

包含：
- 架构概览图（用文本描述即可）
- 模块划分和职责
- 模块间依赖关系
- 数据流说明
- 技术选型理由
