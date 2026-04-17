# 后端 MySQL 容器与部署验收

## 开发期：只起 MySQL

后端编码/测试时不要跑完整 compose，**只起 mysql 一个容器**，backend 仍在宿主机 `go run`。

```bash
make podman-mysql-up   # 等 healthy 后返回
```

模板 compose 里 mysql 配置：

| 项 | 值 |
|---|---|
| 库名 | `appdb` |
| 普通用户 | `appuser` / `apppassword` |
| root 密码 | `rootpassword` |
| 端口 | `3306` |
| healthcheck | 已内置 |

## ⚠️ DSN 陷阱（首次起后端前必改）

模板 `backend/config/config.yaml` 的默认 DSN 是占位值：

```
user:pass@tcp(localhost:3306)/appdb
```

和 compose 里的 mysql 账号**不一致**，backend 本地跑会直接连接失败。必须改成：

```
appuser:apppassword@tcp(127.0.0.1:3306)/appdb?charset=utf8mb4&parseTime=True&loc=Local
```

要点：
- 用户/密码改成 `appuser:apppassword`（和 compose 对齐）
- host 是 `127.0.0.1`，不是 `mysql` —— 因为 backend 跑在宿主机，不在 compose 网络里
- 末尾的 `charset`、`parseTime`、`loc` 参数都要带，否则 time.Time 解析会出问题

> compose 里 backend 服务的 DSN 由 `docker-compose.yml` 的 env 覆盖，host 用 `mysql`，不影响 `config.yaml`。

## 测试

```bash
make podman-mysql-up
make test-backend
```

## 清理

```bash
make podman-down     # 停容器，保留数据卷
make podman-clean    # 连数据卷一起删
```

## 最终全栈验收（阶段 10）

前后端全部完成后：

```bash
make podman-down     # 先停开发期的 mysql 单容器，避免端口冲突
make podman-up       # podman compose up -d --build
make podman-ps       # 三个服务都应 running，mysql 应 healthy
make podman-logs     # 确认 backend DB 连接成功、路由注册完成（Ctrl+C 退出）
```

### 验收清单

- [ ] `mysql` / `backend` / `frontend` 三个容器都 running，mysql healthy
- [ ] 浏览器访问 `http://localhost:5173` 前端可打开
- [ ] 前端调用后端接口（`http://localhost:8080` 或 nginx 反代）返回正常数据
- [ ] `make podman-down && make podman-up` 重启后行为一致（证明数据卷持久化）

### 停止

```bash
make podman-down     # 保留数据
make podman-clean    # 彻底清理（含 mysql 数据卷）
```

## 命令速查

**只用 podman，不用 docker**。模板 Makefile 里 `docker-up` 基于 `docker compose`，本 skill 一律走 `podman-*` 目标。

| 场景 | 命令 |
|---|---|
| 只起 mysql（开发期） | `make podman-mysql-up` |
| 停所有（保留数据） | `make podman-down` |
| 清所有（含数据卷） | `make podman-clean` |
| 全栈起（验收期） | `make podman-up` |
| 查状态 | `make podman-ps` |
| 跟日志 | `make podman-logs` |
| 后端本地跑 | `make run-backend` |
| 前端本地跑 | `make run-frontend` |
| 前端类型检查 | `npx tsc --noEmit 2>&1` |
| 前端构建检查 | `make check-frontend` |
| 后端测试 | `make test-backend` |
