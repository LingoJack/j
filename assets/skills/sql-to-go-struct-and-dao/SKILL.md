---
name: sql-to-go-struct-and-dao
description: 根据 sql 语句自动生成 go 的结构体和 DAO 层代码，基于 gorm 框架
---

```bash
go install github.com/LingoJack/model_infrax/cmd/jen@latest
```

在需要的目录执行 jen init 初始化项目，会自动创建 .model_infrax/ 文件

详情请见
```bash
jen -h
```