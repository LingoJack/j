# webapp-gen 脚本工具箱

改进后的脚本集合，支持完整的 Web 应用快速生成工作流。

## 🎯 脚本概览

### 前端脚本

#### `init_frontend.sh` - 初始化前端项目
```bash
./scripts/init_frontend.sh <project_name>
```

**功能**:
- ✅ 创建 React + TypeScript + Vite 项目
- ✅ 自动安装所有依赖
- ✅ 项目名称验证
- ✅ 重复项目检测
- ✅ 详细的日志输出和错误处理

**参数**:
- `<project_name>`: 项目名称（只允许字母数字、连字符、下划线）

**示例**:
```bash
./scripts/init_frontend.sh my-todo-app
# 输出: frontend/my-todo-app/
```

---

#### `frontend_check.sh` - 前端构建验证
```bash
cd frontend/<project_name>
../../scripts/frontend_check.sh
```

**功能**:
- ✅ 验证项目结构
- ✅ 自动安装缺失依赖
- ✅ 执行生产构建
- ✅ 显示构建输出大小
- ✅ 失败时给出清晰提示

**检查项**:
- package.json 存在性
- node_modules 依赖
- 构建成功
- dist 输出大小

---

### 后端脚本

#### `init_backend.sh` - 初始化后端项目
```bash
./scripts/init_backend.sh <project_name> [module_name]
```

**功能**:
- ✅ 创建 Go + Gin + GORM 项目
- ✅ 初始化 Go Module
- ✅ 创建标准项目结构
- ✅ 安装核心依赖 (Gin, GORM, MySQL Driver)
- ✅ 生成示例代码和 README

**参数**:
- `<project_name>`: 项目名称
- `[module_name]`: Go Module 名称 (可选，默认: github.com/example/<project_name>)

**项目结构**:
```
backend/
├── cmd/                  # 入口程序
├── internal/
│   ├── service/         # 业务逻辑层
│   ├── repository/      # 数据访问层
│   ├── model/           # 数据模型
│   └── middleware/      # 自定义中间件
├── config/              # 配置管理
├── pkg/                 # 公开包
├── docs/                # API 文档
├── bin/                 # 编译输出
├── go.mod
├── go.sum
└── README.md
```

**示例**:
```bash
./scripts/init_backend.sh my-todo-api com.mycompany/todo-api
```

---

#### `backend_check.sh` - 后端构建验证
```bash
cd backend
../scripts/backend_check.sh
```

**功能**:
- ✅ 验证 Go 环境
- ✅ 下载依赖
- ✅ 依赖完整性检查
- ✅ 运行单元测试 (如存在)
- ✅ 编译为二进制文件
- ✅ 显示二进制大小

**输出**:
- 编译后的二进制: `bin/app`
- 测试结果摘要
- 依赖状态报告

---

## 📋 使用流程

### 快速开始

```bash
# 1. 初始化项目
mkdir my-app && cd my-app

# 2. 初始化前端 (在项目根目录)
../path/to/scripts/init_frontend.sh my-app

# 3. 初始化后端 (在项目根目录)
../path/to/scripts/init_backend.sh my-app

# 4. 验证构建
cd frontend/my-app && ../../scripts/frontend_check.sh
cd ../../backend && ../scripts/backend_check.sh
```

### 完整工作流

1. **创建项目结构**
   ```bash
   ./scripts/init_frontend.sh my-project
   ./scripts/init_backend.sh my-project
   ```

2. **前端开发**
   ```bash
   cd frontend/my-project
   npm run dev        # 开发模式
   ```

3. **后端开发**
   ```bash
   cd backend
   go run cmd/main.go # 运行服务器
   ```

4. **验证构建**
   ```bash
   # 前端
   cd frontend/my-project
   ../../scripts/frontend_check.sh
   
   # 后端
   cd backend
   ../scripts/backend_check.sh
   ```

---

## 🛠️ 脚本特性

### 通用特性

- **颜色输出**: 清晰的信息分类
  - 🔵 INFO - 信息消息
  - 🟢 SUCCESS - 成功操作
  - 🟡 WARN - 警告信息
  - 🔴 ERROR - 错误信息

- **错误处理**: `set -euo pipefail` 确保
  - 命令失败时立即退出
  - 未定义变量时报错
  - 管道中的错误被捕获

- **输入验证**: 
  - 参数完整性检查
  - 项目名称格式验证
  - 重复项目检测

- **详细日志**: 每个步骤都有清晰的进度提示

---

## ⚙️ 系统要求

### 前端
- Node.js >= 16
- npm >= 7

### 后端
- Go >= 1.21
- MySQL >= 5.7 (后续使用)

---

## 🔧 自定义和扩展

### 修改前端 Vite 版本
编辑 `init_frontend.sh`，改变 `create-vite@latest` 版本:
```bash
echo "" | npx -y create-vite@<version> "$PROJECT_NAME" --template react-ts
```

### 添加额外的 Go 依赖
编辑 `init_backend.sh` 的 `go get` 部分:
```bash
go get github.com/your/dependency@latest
```

### 自定义构建输出
编辑脚本中的 `go build` 命令:
```bash
go build -o bin/app -ldflags="-s -w" cmd/main.go
```

---

## 📝 故障排除

### 前端

| 问题 | 解决方案 |
|------|---------|
| `npm install` 超时 | 使用淘宝镜像: `npm install -g cnpm` |
| 项目已存在 | 删除旧目录或更换项目名称 |
| Node 版本不兼容 | 使用 nvm 切换到 Node 16+ |

### 后端

| 问题 | 解决方案 |
|------|---------|
| `go get` 失败 | 配置 GOPROXY: `go env -w GOPROXY=https://goproxy.cn` |
| 找不到 Go | 检查 Go 安装: `go version` |
| 构建失败 | 清理模块缓存: `go clean -modcache` |

---

## 📈 后续改进建议

- [ ] `dev_server.sh` - 同时启动前后端开发服务器
- [ ] `build_all.sh` - 完整项目构建脚本
- [ ] `gen_dao.sh` - 自动生成 DAO 层代码
- [ ] `docker_build.sh` - Docker 镜像构建
- [ ] `deploy.sh` - 自动化部署脚本

