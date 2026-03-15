# Go 后端规范

> 所有 shell 命令通过 `cwd` 参数指定工作目录，不使用 `cd`。

## 项目结构

```
backend/
├── cmd/
│   └── server/
│       └── main.go          # 入口：初始化 DB、注册路由、启动服务
├── internal/
│   ├── config/
│   │   └── config.go        # 配置加载（环境变量 / .env）
│   ├── database/
│   │   └── database.go      # GORM 数据库连接 + 自动迁移
│   ├── middleware/
│   │   ├── auth.go          # JWT 鉴权中间件
│   │   └── cors.go          # CORS 中间件
│   └── <module>/            # 每个业务模块一个目录
│       ├── model.go         # 数据模型（GORM struct）
│       ├── repository.go    # 数据访问层
│       ├── service.go       # 业务逻辑层
│       ├── handler.go       # HTTP Handler
│       ├── router.go        # 模块路由注册
│       └── service_test.go  # 测试
├── pkg/
│   └── response/
│       └── response.go      # 统一响应格式
├── go.mod
├── go.sum
└── .env.example
```

## 初始化命令

```json
{ "command": "mkdir -p backend && cd backend && go mod init <module-name>", "cwd": "PROJECT_DIR" }
```

## 核心依赖

```go
// go.mod 关键依赖
require (
    github.com/gin-gonic/gin v1.9+
    gorm.io/gorm v1.25+
    gorm.io/driver/mysql v1.5+
    github.com/golang-jwt/jwt/v5 v5.2+
    github.com/joho/godotenv v1.5+
)
```

## 统一响应格式

```go
// pkg/response/response.go
type Response struct {
    Code    int         `json:"code"`
    Message string      `json:"message"`
    Data    interface{} `json:"data,omitempty"`
}

func Success(c *gin.Context, data interface{}) {
    c.JSON(200, Response{Code: 0, Message: "ok", Data: data})
}

func Error(c *gin.Context, httpCode int, message string) {
    c.JSON(httpCode, Response{Code: -1, Message: message})
}
```

## 模型规范

```go
// internal/user/model.go
type User struct {
    ID        uint           `gorm:"primarykey" json:"id"`
    CreatedAt time.Time      `json:"created_at"`
    UpdatedAt time.Time      `json:"updated_at"`
    DeletedAt gorm.DeletedAt `gorm:"index" json:"-"`
    Username  string         `gorm:"uniqueIndex;size:64;not null" json:"username"`
    Email     string         `gorm:"uniqueIndex;size:128;not null" json:"email"`
    Password  string         `gorm:"size:256;not null" json:"-"`
}
```

要点：
- 密码字段 `json:"-"` 不暴露
- 使用 GORM 软删除
- 字段加 `size` 约束

## Handler 规范

```go
// internal/user/handler.go
type Handler struct {
    service *Service
}

func NewHandler(service *Service) *Handler {
    return &Handler{service: service}
}

func (h *Handler) Register(c *gin.Context) {
    var req RegisterRequest
    if err := c.ShouldBindJSON(&req); err != nil {
        response.Error(c, 400, "参数错误: "+err.Error())
        return
    }
    user, err := h.service.Register(req)
    if err != nil {
        response.Error(c, 500, err.Error())
        return
    }
    response.Success(c, user)
}
```

要点：
- 请求用 `ShouldBindJSON` 绑定
- 错误统一用 `response.Error`
- Handler 只做参数绑定和响应，业务逻辑在 Service 层

## 路由注册

```go
// internal/user/router.go
func RegisterRoutes(r *gin.RouterGroup, h *Handler) {
    users := r.Group("/users")
    {
        users.POST("/register", h.Register)
        users.POST("/login", h.Login)
    }
    // 需要鉴权的路由
    auth := users.Group("", middleware.AuthRequired())
    {
        auth.GET("/me", h.GetProfile)
        auth.PUT("/me", h.UpdateProfile)
    }
}
```

## main.go 模式

```go
func main() {
    config.Load()
    db := database.Connect()

    r := gin.Default()
    r.Use(middleware.CORS())

    api := r.Group("/api")

    // 每个模块独立注册
    userRepo := user.NewRepository(db)
    userService := user.NewService(userRepo)
    userHandler := user.NewHandler(userService)
    user.RegisterRoutes(api, userHandler)

    // ... 其他模块同理

    r.Run(":" + config.Get("PORT", "8080"))
}
```

## 编译检查

```json
{ "command": "go build ./cmd/server/...", "cwd": "PROJECT_DIR/backend", "timeout": 30 }
```

## 测试执行

```json
{ "command": "go test ./internal/<module>/...", "cwd": "PROJECT_DIR/backend", "timeout": 30 }
```

## 环境变量

```env
# .env.example
PORT=8080
DB_DSN=root:password@tcp(127.0.0.1:3306)/mydb?charset=utf8mb4&parseTime=True&loc=Local
JWT_SECRET=your-secret-key
```
