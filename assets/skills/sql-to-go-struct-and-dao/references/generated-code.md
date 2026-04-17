# 生成代码的使用方法

以表 `t_example` 为例说明。PO = `entity.TExample`，DTO = `query.TExampleDto`，DAO = `dao.TExampleDao`。

## PO（entity）

```go
// 直接 new
po := &entity.TExample{Name: "foo"}

// Builder 链式
po := entity.NewTExampleBuilder().WithName("foo").Build()

// 转 JSON
po.Jsonify()         // 紧凑
po.JsonifyIndent()   // 缩进
```

## DTO 字段命名约定

DAO 自动按 DTO 字段名/后缀拼条件，**零值字段会被忽略**。利用这点就能实现"传参才过滤"。

| DTO 字段形式 | 生成的 SQL | 注意 |
|---|---|---|
| `Id`, `Name`, … 基础字段 | `field = ?` 精确匹配 | — |
| `XxxFuzzy` | `field LIKE '%?%'` | — |
| `XxxList` | `field IN (?)` | 不想过滤传 `nil`；**不能传长度为 0 的 slice，会返回 error** |
| `XxxStart` + `XxxEnd` | `field >= ? AND field < DATE_ADD(?, INTERVAL 1 DAY)` | 闭区间（End 含当天） |
| `OrderBy` | 拼到 `ORDER BY` | 已做字段白名单 + ASC/DESC 校验，非法直接丢弃 |
| `PageOffset` + `PageSize` | `LIMIT ? OFFSET ?` | `Offset = PageOffset * PageSize`；`PageSize<=0` 不分页 |

```go
dto := query.NewTExampleDtoBuilder().
    WithNameFuzzy("foo").
    WithOrderBy("createTime DESC").
    WithPageOffset(0).
    WithPageSize(20).
    Build()
list, err := dao.SelectList(ctx, dto)
```

## DAO 方法速查

### 查询

```go
dao.SelectById(ctx, id)                                        // 主键单查，不存在返回 (nil, err)
dao.SelectByIdList(ctx, ids)                                   // 主键批量；空 ids 返回空 slice 无 err
dao.SelectList(ctx, dto)                                       // 多条件 + 排序 + 分页
dao.SelectCount(ctx, dto)                                      // 计数
dao.SelectListWithAppendConditionFunc(ctx, dto, fn)            // 复杂条件补充
dao.SelectCountWithAppendConditionFunc(ctx, dto, fn)
```

`AppendConditionFunc` 签名：

```go
func(ctx context.Context, db *gorm.DB) *gorm.DB
```

### 插入

```go
dao.Insert(ctx, po)                           // Create，全字段写入
dao.InsertBatch(ctx, poList)                  // 批量 Create，一个事务
dao.InsertOrUpdateNullable(ctx, po)           // Save，按主键存在与否插入或更新，**零值覆盖**
dao.InsertOrUpdateBatchNullable(ctx, poList)
```

### 更新

```go
// 主键更新
dao.UpdateById(ctx, po, id)                                  // Updates(struct)，零值忽略
dao.UpdateByIdWithMap(ctx, id, map[string]any{...})          // Updates(map)，可显式写零值
dao.UpdateByIdWithCondition(ctx, po, id, conditionMap)       // 额外条件（乐观锁）
dao.UpdateByIdWithMapAndCondition(ctx, id, updateMap, condMap)

// 非主键的索引字段同样会生成 UpdateByXxx 一套（若 crud_only_idx=true 只保留索引字段的）
```

### 删除

```go
dao.DeleteById(ctx, id)
```

### 原生 SQL

```go
var one entity.TExample
dao.ExecSql(ctx, &one, "SELECT * FROM t_example WHERE id=?", 1)

var many []*entity.TExample
dao.ExecSql(ctx, &many, "SELECT * FROM t_example WHERE name=?", "foo")

// 聚合时自定义 struct
type Agg struct {
    SkillId string `gorm:"column:skill_id"`
    Count   int64  `gorm:"column:count"`
}
var rows []*Agg
dao.ExecSql(ctx, &rows, "SELECT skill_id, COUNT(*) AS count FROM t_example GROUP BY skill_id")
```

注意：
- `recvPtr` **必须是指针**
- 单行查询若无记录返回 error
- 列别名要和 `gorm:"column:..."` 标签一致

### 事务

```go
// 方式 1：闭包（推荐）
dao.Transaction(ctx, func(txDao *dao.TExampleDao) error {
    if err := txDao.Insert(ctx, po1); err != nil { return err }
    return txDao.Insert(ctx, po2)
})

// 方式 2：嵌入外部 *gorm.DB 事务
db.Transaction(func(tx *gorm.DB) error {
    txDao := dao.WithTx(tx)
    return txDao.Insert(ctx, po)
})
```

## 零值覆盖语义（最易错的地方）

| 方法 | 底层 gorm 调用 | 零值是否写入 DB |
|---|---|---|
| `Insert` / `InsertBatch` | `Create` | 全字段写入 |
| `InsertOrUpdateNullable` | `Save` | **会**，覆盖为零值 |
| `UpdateByXxx`（传 PO） | `Updates(struct)` | **不会**，零值被忽略 |
| `UpdateByXxxWithMap` | `Updates(map)` | **会**，按 map 精确写 |

结论：
- 想把字段改成 `""` / `0` / `nil`：必须走 `*WithMap`
- 想做"部分更新保留其他字段"：用 `UpdateByXxx` 传 PO
- 不要用 `InsertOrUpdateNullable` 做部分更新 —— 它会把 PO 里没填的字段全刷成零值

## OrderBy 白名单

生成的 `isValidOrderBy` 会按 `getValidOrderByFields()` 白名单校验。若外部传入 `OrderBy` 包含非白名单字段，**整串被丢弃**（不报错），即不排序。想扩展字段（例如支持 JOIN 出来的列）需要手动改 `getValidOrderByFields()`。
