---
name: sql-to-go-struct-and-dao
description: 根据 sql 语句自动生成 go 的结构体和 DAO 层代码，基于 gorm 框架
---

安装 jen (如无)
```bash
go install github.com/LingoJack/model_infrax/cmd/jen@latest
```

在需要的目录执行 `jen init` 初始化项目，会自动创建 `.model_infrax/` 目录及其相关配置
目录结构如：
```bash
➜  j git:(main) ✗ tree .model_infrax 
.model_infrax
├── config.yml
└── schema.sql
```

生成使用方法详情请见
```bash
> jen -h
Model Infrax v2.0.1 — Go 代码生成器 CLI

用法:
  jen init          初始化 .model_infrax 配置目录（已有配置会询问是否覆盖）
  jen               加载 .model_infrax/config.yml 生成代码
  jen -c <path>     指定配置文件路径生成代码
  jen -v            显示版本信息
  jen -h            显示帮助信息

快速上手:
  1. jen init                            初始化配置
  2. 编辑 .model_infrax/schema.sql       编写建表语句
  3. jen                                 生成代码到 target/jen/

更多信息:
  https://github.com/LingoJack/model_infrax
```

配置文件例如：
```yaml
generate_config:
  # 生成模式: database(从数据库解析) 或 statement(从SQL文件解析)
  generate_mode: statement
  
  # database 模式配置
  # database_name: test_db
  # host: localhost
  # port: 3306
  # username: root
  # password: 123456

  # statement 模式配置（相对于项目根目录）
  sql_file_path: .model_infrax/schema.sql
  
  # 通用配置
  all_tables: true
  table_names:
    - t_example

generate_option:
  # 输出路径（相对于项目根目录）
  output_path: target/jen

  # 是否将所有模型放在一个文件中
  all_model_in_one_file: false

  # 所有模型放在一个文件中的文件名
  all_model_in_one_file_name: model.go

  # 只为有索引的字段生成 infrax 方法
  crud_only_idx: false

  # go 的 package 映射
  package:
    po: model/entity
    dto: model/query
    vo: model/view
    dao: dao
    tool: tool

  # 使用框架, 可选: itea-go, gorm
  use_framework: itea-go
```

要生成代码的sql放置于 `.model_infrax/schema.sql`
修改配置中的 `output_path` 输出路径以及要使用的框架 `use_framework` ，或者手动复制生成的代码
生成代码的命令
```bash
jen
```