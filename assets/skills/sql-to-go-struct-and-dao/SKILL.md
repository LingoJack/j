---
name: sql-to-go-struct-and-dao
description: 用 jen (github.com/LingoJack/model_infrax) 把 MySQL 建表 SQL 生成一整套 Go 代码（PO/entity、查询 DTO、视图 VO、DAO），支持 itea-go 与 gorm 两种框架。触发场景：用户提供 CREATE TABLE 语句要生成 Go model/DAO；用户说"根据这张表生成结构体/DAO/CRUD"；用户想搭建 gorm 项目的数据访问层；`.model_infrax/` 目录存在或被提及。不要用在非 MySQL、非 gorm 体系的场景。
---

# 工作流

## 1. 安装 jen（幂等）

```bash
command -v jen >/dev/null || go install github.com/LingoJack/model_infrax/cmd/jen@latest
```

## 2. 初始化配置

在**项目根目录**执行：

```bash
jen init
```

生成 `.model_infrax/{config.yml, schema.sql}`。如果已存在会询问是否覆盖。

## 3. 写 SQL + 改配置

1. 把 `CREATE TABLE ...` 贴到 `.model_infrax/schema.sql`
2. 打开 `.model_infrax/config.yml`，**必须确认**：
   - `output_path` — 输出目录，相对项目根。默认 `target/jen`，几乎总要改成项目真实代码目录（如 `internal/gen`），不然生成完还要搬
   - `use_framework` — `itea-go`（依赖腾讯内网 `git.woa.com/...` 的 `igorm.BaseDao`）或 `gorm`（纯 gorm）
   - `package` — 5 个子包路径，必须和 `output_path` 下真实目录层级一致，否则跨包 import 会断

字段全量说明与人工处理清单见 [references/config.md](references/config.md)。

## 4. 生成

```bash
jen                # 读 .model_infrax/config.yml
jen -c <path>      # 或指定配置
```

## 5. 产物结构

以表 `t_example` 为例，`<output_path>/` 下得到：

```
model/entity/t_example.go      # PO + Builder + Jsonify
model/query/t_example_dto.go   # 查询 DTO + Builder
model/view/t_example_vo.go     # VO
dao/t_example_dao.go           # CRUD + 事务 + 原生 SQL
tool/*.go                      # 一整套通用工具（整个项目只需一份）
```

调用方式、DTO 字段约定（Fuzzy/List/Start/End/OrderBy/Page）、零值覆盖语义、事务写法见 [references/generated-code.md](references/generated-code.md)。**修改数据时零值语义易错，首次使用务必读这个文件。**

# 生成后必做检查（否则跑不起来）

1. DAO 里 `func (dao *XxxDao) Database() string` 默认返回 `"@database_name"`，改成真实逻辑库名
2. `use_framework: itea-go` 时确认能拉到 `git.woa.com/tencent-cloud-platform/go-module/itea-gorm`；拉不到就切 `gorm`
3. 跑一遍 `go build ./...` 验证 import 路径（`output_path` + `package` 配对是否正确）
4. 若项目已有 `tool/` 工具包，跟生成的 `tool/` 去重
