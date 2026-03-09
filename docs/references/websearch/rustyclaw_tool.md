[思考中]
[读取文件中]
[查找文件中]
[思考中]
让我读取完整的工具列表：
[读取文件中]
[思考中]
现在我已经获取了完整的工具列表，让我为你整理一下：

---

# 🧰 RustyClaw 完整工具清单

根据 `mod.rs` 中的定义，RustyClaw 共有 **50 个工具**，按功能分类如下：

---

## 📁 文件系统工具 (6个)

| 工具名 | 功能 |
|--------|------|
| `read_file` | 读取文件内容，支持 .docx/.doc/.rtf/.odt/.pdf/.html |
| `write_file` | 创建或覆写文件，自动创建父目录 |
| `edit_file` | 搜索替换式编辑文件 |
| `list_directory` | 列出目录内容 |
| `search_files` | 在文件内容中搜索文本（类似 grep） |
| `find_files` | 按文件名查找文件（支持 glob） |

---

## 🌐 网络工具 (2个)

| 工具名 | 功能 |
|--------|------|
| `web_fetch` | 获取 URL 内容并转为 markdown，支持 auth headers |
| `web_search` | 使用 Brave Search API 搜索网络 |

---

## 🖥️ 浏览器工具 (1个)

| 工具名 | 功能 |
|--------|------|
| `browser` | **多合一浏览器控制**：status/start/stop/profiles/tabs/open/focus/close/snapshot/screenshot/navigate/console/pdf/act |

---

## 💾 记忆系统工具 (4个)

| 工具名 | 功能 |
|--------|------|
| `memory_search` | 语义搜索 MEMORY.md 和 memory/*.md 文件 |
| `memory_get` | 读取记忆文件内容 |
| `save_memory` | 保存记忆到 HISTORY.md 和 MEMORY.md |
| `search_history` | 搜索 HISTORY.md 历史记录 |

---

## ⏰ 调度与任务工具 (7个)

| 工具名 | 功能 |
|--------|------|
| `cron` | 管理定时任务 |
| `task_list` | 列出活跃任务 |
| `task_status` | 查看任务状态 |
| `task_foreground` | 将任务切换到前台 |
| `task_background` | 将任务切换到后台 |
| `task_cancel` | 取消任务 |
| `task_pause` / `task_resume` | 暂停/恢复任务 |

---

## 👥 会话与代理工具 (6个)

| 工具名 | 功能 |
|--------|------|
| `sessions_list` | 列出活跃会话 |
| `sessions_spawn` | 生成子代理异步执行任务 |
| `sessions_send` | 向其他会话发送消息 |
| `sessions_history` | 获取会话历史 |
| `session_status` | 查看会话状态 |
| `agents_list` | 列出可用的代理配置 |

---

## 🔐 密钥管理工具 (4个)

| 工具名 | 功能 |
|--------|------|
| `secrets_list` | 列出密钥 |
| `secrets_get` | 获取密钥值 |
| `secrets_store` | 存储密钥 |
| `secrets_set_policy` | 设置密钥访问策略 |

---

## 🔌 MCP 工具 (3个)

| 工具名 | 功能 |
|--------|------|
| `mcp_list` | 列出已连接的 MCP 服务器 |
| `mcp_connect` | 连接 MCP 服务器 |
| `mcp_disconnect` | 断开 MCP 服务器 |

---

## 🎯 Skill 技能工具 (7个)

| 工具名 | 功能 |
|--------|------|
| `skill_list` | 列出已加载的技能 |
| `skill_search` | 在 ClawHub 注册表搜索可安装技能 |
| `skill_install` | 从 ClawHub 安装技能 |
| `skill_info` | 查看技能详情 |
| `skill_enable` | 启用/禁用技能 |
| `skill_link_secret` | 将密钥绑定到技能 |
| `skill_create` | 创建新技能 |

---

## 📡 通讯工具 (3个)

| 工具名 | 功能 |
|--------|------|
| `message` | 发送消息 |
| `tts` | 文字转语音 |
| `image` | 图片分析（视觉模型） |

---

## 🎛️ 其他工具 (7个)

| 工具名 | 功能 |
|--------|------|
| `execute_command` | 执行 shell 命令 |
| `process` | 管理后台执行会话 |
| `gateway` | 配置和测试网关 |
| `nodes` | 发现和控制配对设备 |
| `canvas` | 控制节点画布 UI 展示 |
| `apply_patch` | 应用补丁 |
| `pdf` | PDF 处理 |

---

## 📊 与 JCLI 对比

| 功能 | RustyClaw | JCLI |
|------|-----------|------|
| 文件操作 | ✅ 6个工具 | ✅ 类似 |
| 网页获取 | ✅ `web_fetch` | ✅ `web_fetch` |
| 网页搜索 | ✅ `web_search` | ✅ `web_search` |
| **浏览器自动化** | ✅ `browser` (CDP/Lite) | ❌ 缺失 |
| **记忆系统** | ✅ 4个工具 | ❌ 缺失 |
| **子代理** | ✅ `sessions_spawn` | ❌ 缺失 |
| **定时任务** | ✅ `cron` | ❌ 缺失 |
| **MCP 支持** | ✅ 3个工具 | ✅ 已支持 |
| **Skill 系统** | ✅ 7个工具 | ❌ 缺失 |
| **密钥管理** | ✅ 4个工具 | ❌ 缺失 |

---

## 💡 JCLI 可优先引入的功能

按价值排序：

1. **`browser`** - 浏览器自动化（已讨论过迁移计划）
2. **`sessions_spawn`** - 子代理异步执行
3. **`memory_*`** - 记忆持久化系统
4. **`cron`** - 定时任务调度
5. **`skill_*`** - 可扩展技能系统