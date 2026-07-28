# Report Lark Sync

这份文档只说明 `reportctl` 的 Lark/飞书同步用法，不覆盖 Git 同步。

## 前置条件

需要先安装 `j`，并确保本机可以访问飞书开放平台。

Lark 同步依赖 `lark-cli`。首次使用 Lark 后端时，`j` 会自动检查：

- 本机是否能执行 `lark-cli`
- `lark-cli doctor` 是否显示用户身份可用
- 是否已经配置目标飞书文档

如果本机没有 `lark-cli`，`j` 会尝试通过 npm 安装：

```bash
npm install -g @larksuite/cli
```

如果尚未登录，`j` 会触发：

```bash
lark-cli auth login --domain docs,drive,markdown,wiki
```

按浏览器里的提示完成飞书授权即可。

## 从 0 开始配置

### 1. 创建或准备一个飞书周报文档

建议使用飞书 wiki 文档链接，例如：

```text
https://bytedance.larkoffice.com/wiki/xxxxxxxxxxxxxxxxxxxx
```

使用 wiki 文档的好处是，`push -f` / `pull -f` 覆盖前可以在当前周报文档下面创建备份子文档。

### 2. 切换 report 同步后端

执行：

```bash
j reportctl use lark
```

首次切换时，`j` 会检查 `lark-cli`、检查登录状态，并要求输入飞书文档 URL。

配置成功后，会保存类似下面的配置：

```yaml
report:
  sync_backend: lark
  lark_doc_url: https://bytedance.larkoffice.com/wiki/...
  lark_doc_token: ...
  lark_doc_type: docx
  lark_doc_title: ...
```

后续 `j reportctl push` 和 `j reportctl pull` 会自动走 Lark 后端。

## 日常写日报/周报

本地内容仍然写入默认周报文件：

```bash
j report "完成 Lark 同步开发"
j check
```

也可以打开 Markdown 编辑器编辑整篇周报：

```bash
j reportctl open
```

## 推送到飞书

执行：

```bash
j reportctl push
```

默认 `push` 是安全的：

- 如果本地内容和飞书内容一致，不写入。
- 如果本地内容只是飞书内容的追加版本，只把新增部分 append 到飞书文档。
- 如果本地内容会覆盖、删除或改写飞书已有内容，拒绝执行。

例如下面这种同一行扩展会被视为改写，不会被当成安全追加：

```text
remote: "- old"
local:  "- old text"
```

## 从飞书拉取

执行：

```bash
j reportctl pull
```

默认 `pull` 也是安全的：

- 如果飞书内容和本地内容一致，不写入。
- 如果飞书内容只是本地内容的追加版本，写回本地周报文件。
- 如果飞书内容会覆盖、删除或改写本地已有内容，拒绝执行。

## 强制覆盖和备份

只有明确传 `-f` 或 `--force` 时，才允许破坏性覆盖。

### 强制 push

```bash
j reportctl push -f
```

执行顺序：

1. 拉取飞书文档当前内容。
2. 在当前周报 wiki 文档下面创建备份子文档。
3. 日志输出备份子文档 URL。
4. 备份成功后，用本地 `week_report.md` 覆盖飞书文档。

如果备份失败，不会继续覆盖飞书文档。

### 强制 pull

```bash
j reportctl pull -f
```

执行顺序：

1. 读取本地 `week_report.md` 当前内容。
2. 在当前周报 wiki 文档下面创建备份子文档。
3. 日志输出备份子文档 URL。
4. 备份成功后，用飞书文档内容覆盖本地 `week_report.md`。

如果备份失败，不会继续覆盖本地文件。

## 备份子文档

备份子文档标题类似：

```text
j report backup before force push rev 7 - 2026-07-22 17:09:03
j report backup before force pull - 2026-07-22 17:09:26
```

日志会输出类似：

```text
Backup child document created: j report backup before force push rev 7 - 2026-07-22 17:09:03 (https://.../docx/...)
```

当前实现要求配置的文档 URL 是 wiki URL，才能把备份创建为当前周报文档的子文档。如果不是 wiki URL，强制覆盖会被拒绝，避免无备份覆盖。

## 常用命令

```bash
# 切换到 Lark 后端
j reportctl use lark

# 推送本地追加内容到飞书
j reportctl push

# 从飞书拉取追加内容到本地
j reportctl pull

# 强制覆盖飞书，覆盖前创建备份子文档
j reportctl push -f

# 强制覆盖本地，覆盖前创建备份子文档
j reportctl pull -f
```

## 排查

如果 Lark 同步失败，可以先检查：

```bash
lark-cli doctor
```

重点看 `user_identity` 是否为 `pass`。

如果文档配置不完整，可以重新执行：

```bash
j reportctl use lark
```

如果默认 `push` 或 `pull` 被拒绝，说明两边内容不是 append-only 关系。确认确实要覆盖后，再使用 `-f`，并检查日志中的备份子文档 URL。
