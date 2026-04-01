## 命令

| 命令 | 描述 |
|------|------|
| `j concat <名称> [内容]` | 创建/编辑脚本 |
| `j <脚本名> [参数]` | 执行脚本并传递参数 |

## 创建脚本

```bash
# 创建脚本并指定内容
j concat open "open $1"

# 使用 TUI 编辑器创建
j concat deploy

# 在新窗口中创建
j concat build -w
```

## 执行脚本

```bash
# 执行脚本
j open README.md         # README.md 作为 $1 传入
j build                  # 无参数执行

# 在新窗口中执行
j open -w README.md
```

## 环境变量

脚本可以使用以下环境变量：

| 变量 | 描述 |
|------|------|
| `$1`, `$2`, ... | 脚本参数 |
| `$@` | 所有参数 |
| `$J_DATA_PATH` | 数据目录路径 |

## 示例

```bash
# 部署脚本
j concat deploy "git pull && cargo build --release && systemctl restart myapp"

# 备份脚本
j concat backup "cp -r $1 ~/.jdata/backups/$(date +%Y%m%d)"

# 编辑器脚本
j concat edit "code $1"
```
