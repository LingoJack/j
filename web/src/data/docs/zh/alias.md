## 概述

别名系统，为路径和网址创建简短别名以便快速访问。

## 基本用法

### 添加别名

```bash
j set <alias> <path>    # 添加路径别名
j set <alias> <url>     # 添加网址别名
```

### 执行别名

```bash
j <alias>               # 打开路径或网址
```

### 管理别名

```bash
j rm <alias>            # 删除别名
j rename <old> <new>    # 重命名别名
j mf <alias> <new_path> # 修改别名指向
```

## 别名类型

### 路径别名

```bash
# 添加路径
j set work ~/Projects/work
j set notes ~/Documents/notes

# 打开路径
j work    # 在文件管理器中打开
j notes   # 在文件管理器中打开
```

### 网址别名

```bash
# 添加网址
j set gh https://github.com
j set gh-issues https://github.com/issues

# 打开网址
j gh        # 在浏览器中打开
j gh-issues # 在浏览器中打开
```

## 别名存储

别名单独存放在 `~/.jdata/config.yaml`：

```yaml
path:
  work: /Users/user/Projects/work
  notes: /Users/user/Documents/notes

inner_url:
  gh: https://github.com
  gh-issues: https://github.com/issues
```
