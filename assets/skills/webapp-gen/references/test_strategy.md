# 测试策略

> 所有 shell 命令通过 `cwd` 参数指定工作目录，不使用 `cd`。

## 后端单元测试

### 测试范围

每个模块至少覆盖：
- Service 层：核心业务逻辑（创建、查询、更新、删除、边界条件）
- Handler 层：HTTP 接口（请求绑定、状态码、响应格式）

Repository 层不单独测试（通过 Service 测试间接覆盖）。

### 测试模式

使用 Go 标准 `testing` 包 + `httptest`，不引入额外测试框架。

```go
// internal/user/service_test.go
func TestRegister_Success(t *testing.T) {
    db := setupTestDB(t)
    repo := NewRepository(db)
    svc := NewService(repo)

    user, err := svc.Register(RegisterRequest{
        Username: "testuser",
        Email:    "test@example.com",
        Password: "password123",
    })

    if err != nil {
        t.Fatalf("expected no error, got %v", err)
    }
    if user.Username != "testuser" {
        t.Errorf("expected username 'testuser', got '%s'", user.Username)
    }
}

func TestRegister_DuplicateEmail(t *testing.T) {
    db := setupTestDB(t)
    repo := NewRepository(db)
    svc := NewService(repo)

    // 先注册一次
    svc.Register(RegisterRequest{Username: "u1", Email: "dup@example.com", Password: "pass"})

    // 重复注册应报错
    _, err := svc.Register(RegisterRequest{Username: "u2", Email: "dup@example.com", Password: "pass"})
    if err == nil {
        t.Fatal("expected duplicate email error")
    }
}
```

### 测试数据库

使用 SQLite 内存数据库做测试（避免依赖 MySQL）：

```go
func setupTestDB(t *testing.T) *gorm.DB {
    db, err := gorm.Open(sqlite.Open(":memory:"), &gorm.Config{})
    if err != nil {
        t.Fatalf("failed to open test db: %v", err)
    }
    db.AutoMigrate(&User{}) // 迁移当前模块的模型
    return db
}
```

需要额外依赖：`gorm.io/driver/sqlite`（仅测试用）。

### 执行

```json
{ "command": "go test ./internal/...", "cwd": "PROJECT_DIR/backend", "timeout": 60 }
```

## 集成测试

### 测试范围

验证模块间调用链路，如：

1. 注册用户 → 登录获取 token
2. 用 token 创建商品
3. 用 token 下单（引用商品 ID）
4. 查询订单确认状态

### 测试模式

在 `backend/tests/` 目录下写集成测试，启动真实 HTTP 服务：

```go
// backend/tests/integration_test.go
func TestOrderFlow(t *testing.T) {
    // 启动测试服务器
    router := setupTestRouter()
    srv := httptest.NewServer(router)
    defer srv.Close()

    // 1. 注册
    resp := post(srv.URL+"/api/users/register", map[string]string{
        "username": "buyer", "email": "buyer@test.com", "password": "pass123",
    })
    assert(t, resp.Code, 0)

    // 2. 登录
    resp = post(srv.URL+"/api/users/login", map[string]string{
        "email": "buyer@test.com", "password": "pass123",
    })
    token := resp.Data.(map[string]interface{})["token"].(string)

    // 3. 创建订单
    resp = postWithAuth(srv.URL+"/api/orders", token, map[string]interface{}{
        "items": []map[string]interface{}{
            {"product_id": 1, "quantity": 2},
        },
    })
    assert(t, resp.Code, 0)
}
```

### 执行

```json
{ "command": "go test ./tests/... -v", "cwd": "PROJECT_DIR/backend", "timeout": 60 }
```

## 自动修复循环

测试失败时不要跳过，按以下流程重试：

```
重复（最多 5 次）：
  1. RunShell: { "command": "go test ./internal/<module>/...", "cwd": "PROJECT_DIR/backend", "timeout": 30 }
  2. 全部通过 → 结束
  3. 有失败 → 读取完整错误输出 → 分析失败原因 → 修复代码 → 回到 1

5 次仍失败 → 用 ask 工具告知用户，展示错误信息，请求指导
```

常见失败模式和修复方向：
- `undefined` / `not found` → 检查 import 路径
- `type mismatch` → 检查结构体字段类型
- `connection refused` → 确认测试 DB 初始化
- `duplicate key` → 测试间数据隔离（每个 test 用独立 DB）
