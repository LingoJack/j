# j-cli 配置文件详细说明

## 主配置文件 (config.yaml)

位置：`~/.jdata/config.yaml`

### 配置结构

示例：
```yaml
# 本地应用/文件路径
path:
  chrome: /Applications/Google Chrome.app
  vscode: /Applications/Visual Studio Code.app
  iterm: /Applications/iTerm.app

# URL 链接
inner_url:
  github: https://github.com
  google: https://google.com

# 需 VPN 的外网 URL
outer_url:
  internal-docs: https://internal.example.com

# 浏览器列表（值引用 path 中的 key）
browser:
  chrome: chrome
  safari: safari

# 编辑器列表
editor:
  vscode: vscode
  sublime: sublime

# VPN 应用
vpn: { }

# 已注册的脚本
script:
  deploy: ~/.jdata/scripts/deploy.sh
  backup: ~/.jdata/scripts/backup.sh

# 日报系统配置
report:
  git_repo: https://github.com/xxx/report

# 全局设置
setting:
  search-engine: bing  # 搜索引擎：bing/google/duckduckgo

# 日志设置
log:
  mode: concise  # verbose/concise
```

### Section 说明

| Section | 类型 | 说明 |
|---------|------|------|
| `path` | 键值对 | 本地应用或文件路径 |
| `inner_url` | 键值对 | 可直接访问的 URL |
| `outer_url` | 键值对 | 需要 VPN 的外网 URL |
| `browser` | 键值对 | 浏览器列表，值引用 path 中的 key |
| `editor` | 键值对 | 编辑器列表，值引用 path 中的 key |
| `vpn` | 键值对 | VPN 应用配置 |
| `script` | 键值对 | 已注册的脚本路径 |
| `report` | 对象 | 日报系统配置 |
| `setting` | 对象 | 全局设置 |
| `log` | 对象 | 日志配置 |

## AI 对话配置 (agent_config.json)

位置：`~/.jdata/agent/data/agent_config.json`

### 配置结构

```json
{
  "providers": [
    {
      "name": "GPT-4o",
      "api_base": "https://api.openai.com/v1",
      "api_key": "sk-your-api-key",
      "model": "gpt-4o"
    },
    {
      "name": "DeepSeek",
      "api_base": "https://api.deepseek.com/v1",
      "api_key": "sk-your-api-key",
      "model": "deepseek-chat"
    }
  ],
  "active_index": 0,
  "system_prompt": "你是一个有用的助手。",
  "stream_mode": true,
  "max_history_messages": 20,
  "theme": "dark"
}
```

### 字段说明

| 字段 | 类型 | 说明 |
|------|------|------|
| `providers` | 数组 | 模型提供方列表 |
| `active_index` | 数字 | 当前活跃的 provider 索引 |
| `system_prompt` | 字符串 | 系统提示词 |
| `stream_mode` | 布尔 | 是否流式输出 |
| `max_history_messages` | 数字 | 最大历史消息数 |
| `theme` | 字符串 | 主题风格 |

### Provider 字段说明

| 字段 | 类型 | 说明 |
|------|------|------|
| `name` | 字符串 | 显示名称 |
| `api_base` | 字符串 | API 基础地址 |
| `api_key` | 字符串 | API 密钥 |
| `model` | 字符串 | 模型名称 |

### 主题列表

| 主题 | 说明 |
|------|------|
| `dark` | 深色主题（默认） |
| `light` | 浅色主题 |
| `dracula` | Dracula 配色 |
| `gruvbox` | Gruvbox 配色 |
| `monokai` | Monokai 配色 |
| `nord` | Nord 配色 |

## 日报配置 (settings.json)

位置：`~/.jdata/report/settings.json`

### 配置结构

```json
{
  "week_number": 10,
  "start_date": "2026-03-01"
}
```

### 字段说明

| 字段 | 类型 | 说明 |
|------|------|------|
| `week_number` | 数字 | 当前周数 |
| `start_date` | 字符串 | 本周开始日期 |

## 待办数据 (todo.json)

位置：`~/.jdata/todo/todo.json`

### 数据结构

```json
{
  "items": [
    {
      "content": "完成功能开发",
      "completed": false,
      "created_at": "2026-03-01T10:00:00Z"
    },
    {
      "content": "写周报",
      "completed": true,
      "created_at": "2026-03-01T09:00:00Z",
      "completed_at": "2026-03-01T11:00:00Z"
    }
  ]
}
```

### 字段说明

| 字段 | 类型 | 说明 |
|------|------|------|
| `content` | 字符串 | 待办内容 |
| `completed` | 布尔 | 是否完成 |
| `created_at` | 字符串 | 创建时间 |
| `completed_at` | 字符串 | 完成时间（可选）|

## 环境变量

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `J_DATA_PATH` | 数据存储目录 | `~/.jdata` |

## Shell 补全配置

### zsh
```bash
# 添加到 ~/.zshrc
eval "$(j completion zsh)"
```

### bash
```bash
# 添加到 ~/.bashrc
eval "$(j completion bash)"
```

## 自定义配置路径

```bash
# 修改日报文件路径
j change report week_report /custom/path/week_report.md

# 通过环境变量修改数据目录
export J_DATA_PATH=/custom/data/path
```
