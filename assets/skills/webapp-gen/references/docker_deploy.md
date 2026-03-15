# 容器化部署规范

> 所有 shell 命令通过 `cwd` 参数指定工作目录，不使用 `cd`。

## 后端 Dockerfile

多阶段构建，最终镜像只包含二进制文件：

```dockerfile
# backend/Dockerfile
FROM golang:1.23-alpine AS builder
WORKDIR /app
COPY go.mod go.sum ./
RUN go mod download
COPY . .
RUN CGO_ENABLED=0 go build -o server ./cmd/server

FROM alpine:3.19
RUN apk add --no-cache ca-certificates tzdata
WORKDIR /app
COPY --from=builder /app/server .
EXPOSE 8080
CMD ["./server"]
```

## 前端 Dockerfile

构建静态文件 + nginx 托管：

```dockerfile
# frontend/Dockerfile
FROM node:20-alpine AS builder
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build

FROM nginx:alpine
COPY --from=builder /app/dist /usr/share/nginx/html
COPY nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 80
```

前端 nginx 配置（处理 SPA 路由 + 反向代理 API）：

```nginx
# frontend/nginx.conf
server {
    listen 80;
    root /usr/share/nginx/html;
    index index.html;

    # SPA 路由：所有非文件请求回退到 index.html
    location / {
        try_files $uri $uri/ /index.html;
    }

    # API 反向代理到后端
    location /api/ {
        proxy_pass http://backend:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

## docker-compose.yaml

```yaml
services:
  mysql:
    image: mysql:8.0
    environment:
      MYSQL_ROOT_PASSWORD: ${MYSQL_ROOT_PASSWORD:-root123}
      MYSQL_DATABASE: ${MYSQL_DATABASE:-myapp}
    ports:
      - "3306:3306"
    volumes:
      - mysql_data:/var/lib/mysql
    healthcheck:
      test: ["CMD", "mysqladmin", "ping", "-h", "localhost"]
      interval: 5s
      timeout: 3s
      retries: 10

  backend:
    build: ./backend
    ports:
      - "8080:8080"
    environment:
      PORT: "8080"
      DB_DSN: "root:${MYSQL_ROOT_PASSWORD:-root123}@tcp(mysql:3306)/${MYSQL_DATABASE:-myapp}?charset=utf8mb4&parseTime=True&loc=Local"
      JWT_SECRET: ${JWT_SECRET:-dev-secret-change-me}
    depends_on:
      mysql:
        condition: service_healthy

  frontend:
    build: ./frontend
    ports:
      - "80:80"
    depends_on:
      - backend

volumes:
  mysql_data:
```

## .env.example

```env
# MySQL
MYSQL_ROOT_PASSWORD=root123
MYSQL_DATABASE=myapp

# Backend
JWT_SECRET=change-me-in-production

# 如需 Podman：DOCKER_HOST=unix:///run/user/1000/podman/podman.sock
```

## 启动与验证

构建并启动：

```json
{ "command": "docker compose up --build -d", "cwd": "PROJECT_DIR", "timeout": 180 }
```

等待并检查健康状态：

```json
{ "command": "sleep 5 && docker compose ps", "cwd": "PROJECT_DIR", "timeout": 15 }
{ "command": "curl -s http://localhost:8080/api/health", "cwd": "PROJECT_DIR", "timeout": 10 }
{ "command": "curl -s -o /dev/null -w '%{http_code}' http://localhost:80", "cwd": "PROJECT_DIR", "timeout": 10 }
```

期望：后端返回 `{"code":0,"message":"ok"}`，前端返回 HTTP 200。

查看日志（排错用）：

```json
{ "command": "docker compose logs --tail=50", "cwd": "PROJECT_DIR", "timeout": 10 }
```

## 停止与清理

```json
{ "command": "docker compose down", "cwd": "PROJECT_DIR" }
{ "command": "docker compose down -v", "cwd": "PROJECT_DIR" }
```
