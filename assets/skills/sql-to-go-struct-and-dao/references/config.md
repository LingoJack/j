# config.yml 完整字段

## 模板

```yaml
generate_config:
  # 解析来源：statement=读 SQL 文件（推荐）；database=直连 MySQL 反查
  generate_mode: statement

  # database 模式参数（statement 模式下忽略）
  # database_name: test_db
  # host: localhost
  # port: 3306
  # username: root
  # password: 123456

  # statement 模式：SQL 文件路径，相对项目根
  sql_file_path: .model_infrax/schema.sql

  # true=生成 SQL 里所有表；false=按 table_names 白名单
  all_tables: true
  table_names:
    - t_example

generate_option:
  # 输出根目录，相对项目根
  output_path: target/jen

  # 把所有 PO 合并到一个文件（表很多时可减文件数）
  all_model_in_one_file: false
  all_model_in_one_file_name: model.go

  # true=只为带索引的字段生成 SelectByXxx/UpdateByXxx，瘦身
  crud_only_idx: false

  # 子包路径（相对 output_path），同时决定 Go package 名
  package:
    po: model/entity
    dto: model/query
    vo: model/view
    dao: dao
    tool: tool

  # 框架：itea-go（腾讯 itea，依赖 igorm.BaseDao + wired 注入）
  #      gorm  （纯 gorm.DB）
  use_framework: itea-go
```

## 关键字段决策

| 字段 | 怎么选 |
|---|---|
| `generate_mode` | 有建表语句就用 `statement`；没有但能连库就用 `database` |
| `output_path` | 直接填项目里想要的最终目录（例 `internal/gen` / `app/data`），别留默认 `target/jen` 然后手动搬 |
| `use_framework` | 项目能访问 `git.woa.com` 的私有仓 → `itea-go`；否则 `gorm` |
| `crud_only_idx` | 字段超过 ~10 个、但只有少数字段会被单独查时开 `true` |
| `all_tables` | SQL 文件里只有要生成的表就 `true`；混了不想生成的就用 `table_names` 白名单 |

## package 字段的陷阱

`package` 里 5 个路径是**相对 output_path**。真实目录是 `<output_path>/<package.xxx>`，**Go import 路径**是 `<module_name>/<output_path>/<package.xxx>`。

例：
- `output_path: internal/gen`
- `package.dao: dao`
- Go module 为 `example.com/proj`
- → DAO 文件落在 `internal/gen/dao/*.go`
- → import 写 `"example.com/proj/internal/gen/dao"`

如果改了 `output_path` 但没同步 `package`，或者反之，`go build` 会报找不到 package。

## 生成后必做人工处理

1. **改 `Database()`**：每个 DAO 都有 `func (dao *XxxDao) Database() string { return "@database_name" }`，改成真实逻辑库名
2. **验证私有依赖**（itea-go 模式）：`go mod tidy` 能否拉到 `git.woa.com/tencent-cloud-platform/go-module/itea-gorm`
3. **tool 去重**：生成的 `tool/` 含 id/hash/ptr/copy/aes/jwt 等约 20 个文件；若项目已有同名工具，择一保留
4. **build 验证**：`go build ./...`，特别看 import 路径对不对

## 命令行

```bash
jen init          # 创建 .model_infrax/
jen               # 用默认配置生成
jen -c <path>     # 用指定配置生成
jen -v            # 版本
jen -h            # 帮助
```
