import type { Language } from '../../types'

export const docTree: Record<Language, Record<string, { title: string; children: Record<string, string> }>> = {
  en: {
    gettingStarted: {
      title: 'Getting Started',
      children: {
        installation: 'Installation',
        quickStart: 'Quick Start',
        dataDirectory: 'Data Directory'
      }
    },
    coreFeatures: {
      title: 'Core Features',
      children: {
        alias: 'Alias Management',
        report: 'Daily Reports',
        todo: 'Todo Management',
        script: 'Script System'
      }
    },
    aiFeatures: {
      title: 'AI Features',
      children: {
        aiChat: 'AI Chat',
        agentMode: 'Agent Mode',
        tools: 'AI Tools',
        skills: 'Skill System',
        hooks: 'Hook System'
      }
    },
    advanced: {
      title: 'Advanced',
      children: {
        browser: 'Browser Automation',
        remote: 'Remote Control',
        permissions: 'Permissions'
      }
    }
  },
  zh: {
    gettingStarted: {
      title: '快速开始',
      children: {
        installation: '安装',
        quickStart: '快速上手',
        dataDirectory: '数据目录'
      }
    },
    coreFeatures: {
      title: '核心功能',
      children: {
        alias: '别名管理',
        report: '日报系统',
        todo: '待办管理',
        script: '脚本系统'
      }
    },
    aiFeatures: {
      title: 'AI 功能',
      children: {
        aiChat: 'AI 对话',
        agentMode: 'Agent 模式',
        tools: 'AI 工具',
        skills: 'Skill 技能',
        hooks: 'Hook 系统'
      }
    },
    advanced: {
      title: '进阶功能',
      children: {
        browser: '浏览器自动化',
        remote: '远程控制',
        permissions: '权限配置'
      }
    }
  }
}

export const docI18n = {
  en: {
    nav: {
      back: '← Back to Home',
      github: 'GitHub',
      menu: 'Menu'
    },
    sections: {
      installation: {
        title: 'Installation',
        content: `## One-click Install (Recommended)

\`\`\`bash
# Install latest version
curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh

# Install specific version
curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh -s -- v1.0.0
\`\`\`

## Install from crates.io

\`\`\`bash
# Standard version (Lite browser mode, no extra dependencies)
cargo install j-cli

# Full version (CDP browser mode, requires Chrome/Chromium)
cargo install j-cli --features browser_cdp
\`\`\`

## Build from Source

\`\`\`bash
git clone https://github.com/LingoJack/j.git
cd j && cargo install --path .

# With full browser automation
cargo install --path . --features browser_cdp
\`\`\`

## Verify Installation

\`\`\`bash
j --version
j --help
\`\`\`

## Update

\`\`\`bash
# Built-in update command (auto-detects installation source)
j update

# Check version only
j update --check

# Manual update via cargo
cargo install j-cli
\`\`\`

## Uninstall

\`\`\`bash
# Using install script (recommended)
curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh -s -- --uninstall

# Or via cargo
cargo uninstall j-cli

# Or manual removal
sudo rm /usr/local/bin/j  # One-click install
rm ~/.cargo/bin/j          # Cargo install

# (Optional) Remove data directory
rm -rf ~/.jdata
\`\`\``
      },
      quickStart: {
        title: 'Quick Start',
        content: `## Register App Aliases

\`\`\`bash
j set chrome "/Applications/Google Chrome.app"
j set vscode "/Applications/Visual Studio Code.app"

# Register URL aliases (auto-detected as inner_url)
j set github https://github.com
\`\`\`

## Mark Categories

\`\`\`bash
j note chrome browser
j note vscode editor
\`\`\`

## Open Apps

\`\`\`bash
j chrome                  # Open Chrome
j chrome github           # Open github URL with Chrome
j chrome "rust lang"      # Search "rust lang" with Chrome
j vscode ./src            # Open src directory with VSCode
\`\`\`

## Daily Reports

\`\`\`bash
j report "Completed feature development"
j check                   # View recent 10 lines
j check 20                # View recent 20 lines
\`\`\`

## Todo Management

\`\`\`bash
j todo add Buy milk       # Quick add
j todo                    # Enter TUI manager
\`\`\`

## AI Chat

\`\`\`bash
j chat                    # Enter TUI chat
j chat Hello              # Quick question
\`\`\`

## Interactive Mode

\`\`\`bash
j                         # Enter interactive mode with Tab completion
\`\`\``
      },
      dataDirectory: {
        title: 'Data Directory',
        content: `All data is stored in \`~/.jdata/\` (customizable via \`J_DATA_PATH\` environment variable):

\`\`\`
~/.jdata/
├── config.yaml          # Main config (aliases, categories, settings)
├── agent/               # AI Agent data
│   ├── data/            # Agent data directory
│   │   ├── agent_config.json   # Agent config (model, API)
│   │   ├── chat_history.json   # Chat history
│   │   ├── archives/           # Archived conversations
│   │   ├── system_prompt.md    # System prompt
│   │   ├── memory.md           # Memory file
│   │   ├── soul.md             # Soul file
│   │   └── style.md            # Response style
│   ├── logs/            # Agent logs
│   │   ├── info.log
│   │   └── error.log
│   └── skills/          # Skills directory
├── bin/                 # Built-in tools
│   └── md_render        # Markdown renderer
├── report/              # Daily reports
│   ├── week_report.md   # Week report file
│   ├── settings.json    # Report settings
│   ├── todo.json        # Todo data
│   └── .git/            # Git repository
├── scripts/             # Scripts created via j concat
\`\`\`

## Config File Structure (\`config.yaml\`)

| Section | Description | Example |
|---------|-------------|---------|
| \`path\` | Local app/file paths | \`chrome: /Applications/Google Chrome.app\` |
| \`inner_url\` | URL links | \`github: https://github.com\` |
| \`outer_url\` | URLs requiring VPN | \`docs: https://internal.example.com\` |
| \`browser\` | Browser list | \`chrome: chrome\` |
| \`editor\` | Editor list | \`vscode: vscode\` |
| \`vpn\` | VPN application | |
| \`script\` | Registered scripts | \`deploy: ~/.jdata/scripts/deploy.sh\` |
| \`report\` | Report system config | \`git_repo: https://github.com/xxx/report\` |
| \`setting\` | Global settings | \`search-engine: bing\` |
| \`log\` | Log settings | \`mode: concise\` |`
      },
      alias: {
        title: 'Alias Management',
        content: `## Commands

| Command | Description |
|---------|-------------|
| \`j set <alias> <path>\` | Set alias (paths → path section, URLs → inner_url) |
| \`j rm <alias>\` | Remove alias (cleans associated category marks) |
| \`j rename <alias> <new>\` | Rename alias (updates all category references) |
| \`j mf <alias> <new_path>\` | Modify alias path |

## Category Marking

\`\`\`bash
j note <alias> <category>   # Mark alias with category
j find <category>           # Find aliases by category
j note chrome browser       # Mark chrome as browser
j note github outer_url     # Mark github as outer_url (auto-connect VPN)
\`\`\`

## Categories

| Category | Description |
|----------|-------------|
| \`browser\` | Web browsers |
| \`editor\` | Code editors |
| \`vpn\` | VPN applications |
| \`script\` | Custom scripts |
| \`inner_url\` | Internal URLs |
| \`outer_url\` | External URLs (auto-connect VPN) |

## Examples

\`\`\`bash
# Set app aliases
j set chrome "/Applications/Google Chrome.app"
j set safari "/Applications/Safari.app"
j set vscode "/Applications/Visual Studio Code.app"

# Set URL aliases
j set github https://github.com
j set google https://google.com

# Set directory aliases
j set proj ~/Projects
j set docs ~/Documents

# Open apps
j chrome                   # Open Chrome
j chrome "rust lang"       # Search with Chrome
j chrome github            # Open github URL with Chrome
j vscode proj              # Open proj directory with VSCode
\`\`\``
      },
      report: {
        title: 'Daily Reports',
        content: `## Commands

| Command | Description |
|---------|-------------|
| \`j report [text]\` | Write to daily report (opens TUI if no text) |
| \`j check [n]\` | View recent n lines (default: 10) |
| \`j search <keyword>\` | Search reports with fuzzy matching |

## Examples

\`\`\`bash
# Quick write
j report "Completed user authentication module"
j report "Meeting with team" "Discussed sprint planning"

# View reports
j check          # View recent 10 lines
j check 20       # View recent 20 lines

# Search
j search authentication
j search "user module" -fuzzy
\`\`\`

## TUI Editor

Running \`j report\` without arguments opens the TUI editor:

- **Multi-line editing**: Write longer entries with proper formatting
- **History suggestions**: Auto-complete from previous entries
- **Tab completion**: Quickly insert common phrases

## Git Integration

\`\`\`bash
# Initialize Git sync
cd ~/.jdata/report
git init
git remote add origin <your-repo>

# Daily workflow
j report "Completed feature"
j reportctl push   # Sync to remote
\`\`\``
      },
      todo: {
        title: 'Todo Management',
        content: `## Commands

| Command | Description |
|---------|-------------|
| \`j todo\` | Open TUI todo manager |
| \`j todo add <text>\` | Quick add todo item |
| \`j todo done <id>\` | Mark todo as done |
| \`j todo list\` | List todos (supports --done/--undone) |

## Examples

\`\`\`bash
# Quick add
j todo add Buy milk
j todo add Review pull request

# List todos
j todo list              # All todos
j todo list --undone     # Only pending
j todo list --done       # Only completed

# Mark as done
j todo done 1
j todo done 1 --report   # Also write to daily report
\`\`\`

## TUI Manager

Running \`j todo\` opens the interactive TUI:

- **Add/Edit/Delete**: Manage todos interactively
- **Priority**: Set priority levels
- **Due dates**: Add deadlines
- **Categories**: Organize by project/context

## Markdown Integration

Todos can be written in daily reports using Markdown:

\`\`\`markdown
- [x] Completed task
- [ ] Pending task
- [ ] Another pending task
\`\`\``
      },
      script: {
        title: 'Script System',
        content: `## Commands

| Command | Description |
|---------|-------------|
| \`j concat <name> [content]\` | Create/edit script |
| \`j <script> [args]\` | Execute script with arguments |

## Creating Scripts

\`\`\`bash
# Create script with content
j concat open "open $1"

# Create script with TUI editor
j concat deploy

# Create in new window
j concat build -w
\`\`\`

## Executing Scripts

\`\`\`bash
# Execute script
j open README.md         # Passes README.md as $1
j build                  # Execute without arguments

# Execute in new window
j open -w README.md
\`\`\`

## Environment Variables

Scripts can use environment variables:

| Variable | Description |
|----------|-------------|
| \`$1\`, \`$2\`, ... | Script arguments |
| \`$@\` | All arguments |
| \`$J_DATA_PATH\` | Data directory path |

## Examples

\`\`\`bash
# Deployment script
j concat deploy "git pull && cargo build --release && systemctl restart myapp"

# Backup script
j concat backup "cp -r $1 ~/.jdata/backups/$(date +%Y%m%d)"

# Open in editor
j concat edit "code $1"
\`\`\``
      },
      aiChat: {
        title: 'AI Chat',
        content: `## Starting AI Chat

\`\`\`bash
j chat              # Open TUI chat
j chat "Hello"      # Quick question
\`\`\`

## Features

- **Multi-model support**: OpenAI, Claude, Gemini, Ollama
- **Streaming output**: Real-time responses
- **Tool calling**: AI can use tools
- **Context management**: Include files and URLs

## Context Reference

\`\`\`bash
# Include local files
@file:src/main.rs Explain this code

# Include directories
@dir:src/ Analyze this codebase

# Include URLs
@url:https://example.com Summarize this page
\`\`\`

## Commands

| Command | Description |
|---------|-------------|
| \`/help\` | Show available commands |
| \`/compact\` | Compress conversation context |
| \`/clear\` | Clear conversation history |
| \`/model\` | Switch AI model |
| \`/export\` | Export conversation |

## Web Search

Enable web search to let AI fetch latest information:

\`\`\`bash
What are the new features in React 19?
\`\`\``
      },
      agentMode: {
        title: 'Agent Mode',
        content: `## Overview

Agent mode enables autonomous multi-step reasoning with tool calling.

## Activation

\`\`\`bash
j agent
\`\`\`

## Features

- **Autonomous reasoning**: AI plans and executes multi-step tasks
- **Tool integration**: Uses available tools automatically
- **Task management**: Breaks down complex requests

## Example Tasks

\`\`\`bash
# Code analysis
Analyze the codebase and suggest improvements

# File operations
Find all TODO comments in the code and create a summary

# Research
Research the best practices for React state management and create a report
\`\`\`

## Tool Permissions

Configure which tools the agent can use:

\`\`\`yaml
# ~/.jdata/agent/data/agent_config.yaml
permissions:
  allow:
    - Read
    - Grep
    - Glob
    - WebFetch
  deny:
    - Bash
    - Write
\`\`\``
      },
      tools: {
        title: 'AI Tools',
        content: `## Available Tools

| Tool | Description |
|------|-------------|
| \`Read\` | Read file contents |
| \`Write\` | Write to files |
| \`Edit\` | Edit files with string replacement |
| \`Glob\` | Find files by pattern |
| \`Grep\` | Search file contents |
| \`Bash\` | Execute shell commands |
| \`WebFetch\` | Fetch web page content |
| \`WebSearch\` | Search the web |
| \`Ask\` | Ask user for input |

## Permission Configuration

\`\`\`yaml
# ~/.jdata/agent/data/agent_config.yaml
tools:
  - name: Read
    permission: allow
  - name: Bash
    permission: ask  # Require user confirmation
  - name: Write
    permission: deny
\`\`\`

## Context References

| Reference | Description |
|-----------|-------------|
| \`@file:path\` | Include file content |
| \`@dir:path\` | Include directory structure |
| \`@url:url\` | Include web page content |
| \`@grep:pattern\` | Include search results |
\`\`\``
      },
      skills: {
        title: 'Skill System',
        content: `## Overview

Skills are specialized prompts that extend AI capabilities.

## Skill Structure

\`\`\`
~/.jdata/agent/skills/<skill_name>/
├── skill.md         # Skill definition
├── assets/          # Supporting files
└── examples/        # Example usage
\`\`\`

## Creating Skills

\`\`\`markdown
# skill.md
---
name: code-review
description: Review code for best practices
trigger: code review
---

You are a code reviewer. Analyze code for:
- Code quality
- Performance issues
- Security vulnerabilities
- Best practices
\`\`\`

## Using Skills

\`\`\`bash
# In AI chat
> code review this file @file:src/main.rs
\`\`\`

## Built-in Skills

- \`code-review\`: Code analysis
- \`test-gen\`: Generate tests
- \`doc-gen\`: Generate documentation
- \`refactor\`: Refactoring suggestions
\`\`\``
      },
      hooks: {
        title: 'Hook System',
        content: `## Overview

Hooks allow custom scripts to run at specific events.

## Hook Events

| Event | When it runs |
|-------|--------------|
| \`pre_send_message\` | Before sending message to AI |
| \`post_llm_response\` | After receiving AI response |
| \`pre_tool_execution\` | Before tool execution |
| \`post_tool_execution\` | After tool execution |
| \`session_start\` | When session starts |
| \`session_end\` | When session ends |

## Registering Hooks

\`\`\`bash
# Register a hook
j hook register pre_send_message "echo 'Sending message...'"

# List hooks
j hook list

# Remove hook
j hook remove pre_send_message 0
\`\`\`

## Hook Scripts

Hook scripts receive JSON via stdin:

\`\`\`json
{
  "event": "pre_send_message",
  "data": {
    "message": "user message"
  }
}
\`\`\`

Scripts should output JSON to stdout to modify the data.
\`\`\``
      },
      browser: {
        title: 'Browser Automation',
        content: `## Modes

| Mode | Description |
|------|-------------|
| **Lite** | Lightweight HTTP control (default) |
| **CDP** | Full browser automation via Chrome DevTools Protocol |

## Lite Mode

\`\`\`bash
# Start lite mode
j browser lite

# Open URL
j browser open https://example.com

# Take screenshot
j browser screenshot
\`\`\`

## CDP Mode

\`\`\`bash
# Start with CDP (requires Chrome/Chromium)
j browser cdp

# Navigate
j browser goto https://example.com

# Click element
j browser click "#submit-button"

# Type text
j browser type "#search" "query"

# Take screenshot
j browser screenshot
\`\`\`

## Features

- Screenshot capture
- Element interaction
- Page navigation
- Script injection
- Cookie management
\`\`\``
      },
      remote: {
        title: 'Remote Control',
        content: `## Overview

Control AI chat from mobile devices via WebSocket.

## Setup

\`\`\`bash
# Start remote control server
j remote start

# Show connection QR code
j remote qr
\`\`\`

## Features

- **Mobile control**: Use phone to send messages
- **Voice input**: Dictate messages
- **Push notifications**: Receive responses on mobile
- **Session sync**: Continue conversations across devices

## Security

- WebSocket secure connection
- Token-based authentication
- End-to-end encryption

## Client Apps

- **Web**: Scan QR code to connect
- **iOS**: Shortcuts integration
- **Android**: Tasker integration
\`\`\``
      },
      permissions: {
        title: 'Permissions',
        content: `## Permission Levels

| Level | Description |
|-------|-------------|
| \`allow\` | Always allowed |
| \`ask\` | Ask for confirmation |
| \`deny\` | Always denied |

## Configuration

\`\`\`yaml
# ~/.jdata/agent/data/agent_config.yaml
permissions:
  # Read operations - always allowed
  - tool: Read
    permission: allow
  
  # Write operations - ask for confirmation
  - tool: Write
    permission: ask
  
  # Shell commands - ask for confirmation
  - tool: Bash
    permission: ask
    rules:
      - pattern: "ls *"        # Allow ls commands
        permission: allow
      - pattern: "rm *"        # Always ask for rm
        permission: ask
  
  # Web access - always allowed
  - tool: WebFetch
    permission: allow
  - tool: WebSearch
    permission: allow
\`\`\`

## Fine-grained Rules

\`\`\`yaml
permissions:
  - tool: Bash
    permission: ask
    rules:
      # Allow specific patterns
      - pattern: "git status"
        permission: allow
      - pattern: "cargo build"
        permission: allow
      
      # Deny dangerous patterns
      - pattern: "rm -rf /*"
        permission: deny
\`\`\``
      }
    }
  },
  zh: {
    nav: {
      back: '← 返回首页',
      github: 'GitHub',
      menu: '菜单'
    },
    sections: {
      installation: {
        title: '安装',
        content: `## 一键安装（推荐）

\`\`\`bash
# 安装最新版本
curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh

# 安装指定版本
curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh -s -- v1.0.0
\`\`\`

## 从 crates.io 安装

\`\`\`bash
# 标准版本（Lite 浏览器模式，无额外依赖）
cargo install j-cli

# 完整版本（CDP 浏览器模式，需要 Chrome/Chromium）
cargo install j-cli --features browser_cdp
\`\`\`

## 从源码构建

\`\`\`bash
git clone https://github.com/LingoJack/j.git
cd j && cargo install --path .

# 包含完整浏览器自动化
cargo install --path . --features browser_cdp
\`\`\`

## 验证安装

\`\`\`bash
j --version
j --help
\`\`\`

## 更新

\`\`\`bash
# 内置更新命令（自动检测安装来源）
j update

# 仅检查版本
j update --check

# 通过 cargo 手动更新
cargo install j-cli
\`\`\`

## 卸载

\`\`\`bash
# 使用安装脚本（推荐）
curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh -s -- --uninstall

# 或通过 cargo
cargo uninstall j-cli

# 或手动删除
sudo rm /usr/local/bin/j  # 一键安装
rm ~/.cargo/bin/j          # Cargo 安装

# （可选）删除数据目录
rm -rf ~/.jdata
\`\`\``
      },
      quickStart: {
        title: '快速上手',
        content: `## 注册应用别名

\`\`\`bash
j set chrome "/Applications/Google Chrome.app"
j set vscode "/Applications/Visual Studio Code.app"

# 注册 URL 别名（自动检测为 inner_url）
j set github https://github.com
\`\`\`

## 标记分类

\`\`\`bash
j note chrome browser
j note vscode editor
\`\`\`

## 打开应用

\`\`\`bash
j chrome                  # 打开 Chrome
j chrome github           # 用 Chrome 打开 github URL
j chrome "rust lang"      # 用 Chrome 搜索 "rust lang"
j vscode ./src            # 用 VSCode 打开 src 目录
\`\`\`

## 日报系统

\`\`\`bash
j report "完成功能开发"
j check                   # 查看最近 10 行
j check 20                # 查看最近 20 行
\`\`\`

## 待办管理

\`\`\`bash
j todo add 买牛奶         # 快速添加
j todo                    # 进入 TUI 管理器
\`\`\`

## AI 对话

\`\`\`bash
j chat                    # 进入 TUI 对话
j chat Hello              # 快速提问
\`\`\`

## 交互模式

\`\`\`bash
j                         # 进入交互模式，支持 Tab 补全
\`\`\``
      },
      dataDirectory: {
        title: '数据目录',
        content: `所有数据存储在 \`~/.jdata/\`（可通过 \`J_DATA_PATH\` 环境变量自定义）：

\`\`\`
~/.jdata/
├── config.yaml          # 主配置（别名、分类、设置）
├── agent/               # AI Agent 数据
│   ├── data/            # Agent 数据目录
│   │   ├── agent_config.json   # Agent 配置（模型、API）
│   │   ├── chat_history.json   # 对话历史
│   │   ├── archives/           # 归档对话
│   │   ├── system_prompt.md    # 系统提示
│   │   ├── memory.md           # 记忆文件
│   │   ├── soul.md             # 灵魂文件
│   │   └── style.md            # 响应风格
│   ├── logs/            # Agent 日志
│   │   ├── info.log
│   │   └── error.log
│   └── skills/          # 技能目录
├── bin/                 # 内置工具
│   └── md_render        # Markdown 渲染器
├── report/              # 日报
│   ├── week_report.md   # 周报文件
│   ├── settings.json    # 日报设置
│   ├── todo.json        # 待办数据
│   └── .git/            # Git 仓库
├── scripts/             # 通过 j concat 创建的脚本
\`\`\`

## 配置文件结构（\`config.yaml\`）

| 区域 | 描述 | 示例 |
|------|------|------|
| \`path\` | 本地应用/文件路径 | \`chrome: /Applications/Google Chrome.app\` |
| \`inner_url\` | URL 链接 | \`github: https://github.com\` |
| \`outer_url\` | 需要 VPN 的 URL | \`docs: https://internal.example.com\` |
| \`browser\` | 浏览器列表 | \`chrome: chrome\` |
| \`editor\` | 编辑器列表 | \`vscode: vscode\` |
| \`vpn\` | VPN 应用 | |
| \`script\` | 注册的脚本 | \`deploy: ~/.jdata/scripts/deploy.sh\` |
| \`report\` | 日报系统配置 | \`git_repo: https://github.com/xxx/report\` |
| \`setting\` | 全局设置 | \`search-engine: bing\` |
| \`log\` | 日志设置 | \`mode: concise\` |`
      },
      alias: {
        title: '别名管理',
        content: `## 命令

| 命令 | 描述 |
|------|------|
| \`j set <别名> <路径>\` | 设置别名（路径 → path 区域，URL → inner_url） |
| \`j rm <别名>\` | 删除别名（清除关联的分类标记） |
| \`j rename <别名> <新名>\` | 重命名别名（更新所有分类引用） |
| \`j mf <别名> <新路径>\` | 修改别名路径 |

## 分类标记

\`\`\`bash
j note <别名> <分类>   # 标记别名分类
j find <分类>           # 按分类查找别名
j note chrome browser   # 将 chrome 标记为浏览器
j note github outer_url # 将 github 标记为外网 URL（自动连接 VPN）
\`\`\`

## 分类

| 分类 | 描述 |
|------|------|
| \`browser\` | 浏览器 |
| \`editor\` | 编辑器 |
| \`vpn\` | VPN 应用 |
| \`script\` | 自定义脚本 |
| \`inner_url\` | 内网 URL |
| \`outer_url\` | 外网 URL（自动连接 VPN） |

## 示例

\`\`\`bash
# 设置应用别名
j set chrome "/Applications/Google Chrome.app"
j set safari "/Applications/Safari.app"
j set vscode "/Applications/Visual Studio Code.app"

# 设置 URL 别名
j set github https://github.com
j set google https://google.com

# 设置目录别名
j set proj ~/Projects
j set docs ~/Documents

# 打开应用
j chrome                   # 打开 Chrome
j chrome "rust 语言"       # 用 Chrome 搜索
j chrome github            # 用 Chrome 打开 github URL
j vscode proj              # 用 VSCode 打开 proj 目录
\`\`\``
      },
      report: {
        title: '日报系统',
        content: `## 命令

| 命令 | 描述 |
|------|------|
| \`j report [文本]\` | 写入日报（无文本时打开 TUI） |
| \`j check [n]\` | 查看最近 n 行（默认：10） |
| \`j search <关键字>\` | 搜索日报，支持模糊匹配 |

## 示例

\`\`\`bash
# 快速写入
j report "完成用户认证模块"
j report "团队会议" "讨论冲刺规划"

# 查看日报
j check          # 查看最近 10 行
j check 20       # 查看最近 20 行

# 搜索
j search 认证
j search "用户模块" -fuzzy
\`\`\`

## TUI 编辑器

不带参数运行 \`j report\` 打开 TUI 编辑器：

- **多行编辑**：支持格式的长内容编辑
- **历史建议**：从历史记录自动补全
- **Tab 补全**：快速插入常用短语

## Git 集成

\`\`\`bash
# 初始化 Git 同步
cd ~/.jdata/report
git init
git remote add origin <your-repo>

# 日常工作流
j report "完成功能开发"
j reportctl push   # 同步到远程
\`\`\``
      },
      todo: {
        title: '待办管理',
        content: `## 命令

| 命令 | 描述 |
|------|------|
| \`j todo\` | 打开 TUI 待办管理器 |
| \`j todo add <文本>\` | 快速添加待办 |
| \`j todo done <id>\` | 标记待办完成 |
| \`j todo list\` | 列出待办（支持 --done/--undone） |

## 示例

\`\`\`bash
# 快速添加
j todo add 买牛奶
j todo add 审查 PR

# 列出待办
j todo list              # 所有待办
j todo list --undone     # 仅待处理
j todo list --done       # 仅已完成

# 标记完成
j todo done 1
j todo done 1 --report   # 同时写入日报
\`\`\`

## TUI 管理器

运行 \`j todo\` 打开交互式 TUI：

- **添加/编辑/删除**：交互式管理待办
- **优先级**：设置优先级
- **截止日期**：添加截止时间
- **分类**：按项目/上下文组织

## Markdown 集成

待办可以用 Markdown 写入日报：

\`\`\`markdown
- [x] 已完成的任务
- [ ] 待处理的任务
- [ ] 另一个待处理任务
\`\`\``
      },
      script: {
        title: '脚本系统',
        content: `## 命令

| 命令 | 描述 |
|------|------|
| \`j concat <名称> [内容]\` | 创建/编辑脚本 |
| \`j <脚本> [参数]\` | 执行脚本并传递参数 |

## 创建脚本

\`\`\`bash
# 带内容创建脚本
j concat open "open $1"

# 用 TUI 编辑器创建脚本
j concat deploy

# 在新窗口创建
j concat build -w
\`\`\`

## 执行脚本

\`\`\`bash
# 执行脚本
j open README.md         # 传递 README.md 作为 $1
j build                  # 无参数执行

# 在新窗口执行
j open -w README.md
\`\`\`

## 环境变量

脚本可以使用环境变量：

| 变量 | 描述 |
|------|------|
| \`$1\`, \`$2\`, ... | 脚本参数 |
| \`$@\` | 所有参数 |
| \`$J_DATA_PATH\` | 数据目录路径 |

## 示例

\`\`\`bash
# 部署脚本
j concat deploy "git pull && cargo build --release && systemctl restart myapp"

# 备份脚本
j concat backup "cp -r $1 ~/.jdata/backups/$(date +%Y%m%d)"

# 打开编辑器
j concat edit "code $1"
\`\`\``
      },
      aiChat: {
        title: 'AI 对话',
        content: `## 启动 AI 对话

\`\`\`bash
j chat              # 打开 TUI 对话
j chat "Hello"      # 快速提问
\`\`\`

## 功能特性

- **多模型支持**：OpenAI、Claude、Gemini、Ollama
- **流式输出**：实时响应
- **工具调用**：AI 可以使用工具
- **上下文管理**：包含文件和 URL

## 上下文引用

\`\`\`bash
# 包含本地文件
@file:src/main.rs 解释这段代码

# 包含目录
@dir:src/ 分析这个代码库

# 包含 URL
@url:https://example.com 总结这个页面
\`\`\`

## 命令

| 命令 | 描述 |
|------|------|
| \`/help\` | 显示可用命令 |
| \`/compact\` | 压缩对话上下文 |
| \`/clear\` | 清除对话历史 |
| \`/model\` | 切换 AI 模型 |
| \`/export\` | 导出对话 |

## 网页搜索

启用网页搜索让 AI 获取最新信息：

\`\`\`bash
React 19 有哪些新功能？
\`\`\``
      },
      agentMode: {
        title: 'Agent 模式',
        content: `## 概述

Agent 模式启用带工具调用的自主多步推理。

## 激活

\`\`\`bash
j ai
\`\`\`

## 功能特性

- **自主推理**：AI 规划并执行多步任务
- **工具集成**：自动使用可用工具
- **任务管理**：分解复杂请求

## 示例任务

\`\`\`bash
# 代码分析
分析代码库并提出改进建议

# 文件操作
找到代码中的所有 TODO 注释并创建摘要

# 研究
研究 React 状态管理的最佳实践并创建报告
\`\`\`

## 工具权限

配置 agent 可以使用的工具：

\`\`\`yaml
# ~/.jdata/agent/data/agent_config.yaml
permissions:
  allow:
    - Read
    - Grep
    - Glob
    - WebFetch
  deny:
    - Bash
    - Write
\`\`\``
      },
      tools: {
        title: 'AI 工具',
        content: `## 可用工具

| 工具 | 描述 |
|------|------|
| \`Read\` | 读取文件内容 |
| \`Write\` | 写入文件 |
| \`Edit\` | 字符串替换编辑文件 |
| \`Glob\` | 按模式查找文件 |
| \`Grep\` | 搜索文件内容 |
| \`Bash\` | 执行 shell 命令 |
| \`WebFetch\` | 获取网页内容 |
| \`WebSearch\` | 搜索网页 |
| \`Ask\` | 询问用户输入 |

## 权限配置

\`\`\`yaml
# ~/.jdata/agent/data/agent_config.yaml
tools:
  - name: Read
    permission: allow
  - name: Bash
    permission: ask  # 需要用户确认
  - name: Write
    permission: deny
\`\`\`

## 上下文引用

| 引用 | 描述 |
|------|------|
| \`@file:path\` | 包含文件内容 |
| \`@dir:path\` | 包含目录结构 |
| \`@url:url\` | 包含网页内容 |
| \`@grep:pattern\` | 包含搜索结果 |
\`\`\``
      },
      skills: {
        title: 'Skill 技能',
        content: `## 概述

技能是扩展 AI 能力的专用提示。

## 技能结构

\`\`\`
~/.jdata/agent/skills/<skill_name>/
├── skill.md         # 技能定义
├── assets/          # 支持文件
└── examples/        # 使用示例
\`\`\`

## 创建技能

\`\`\`markdown
# skill.md
---
name: code-review
description: 代码最佳实践审查
trigger: 代码审查
---

你是一个代码审查者。分析代码的：
- 代码质量
- 性能问题
- 安全漏洞
- 最佳实践
\`\`\`

## 使用技能

\`\`\`bash
# 在 AI 对话中
> 代码审查这个文件 @file:src/main.rs
\`\`\`

## 内置技能

- \`code-review\`：代码分析
- \`test-gen\`：生成测试
- \`doc-gen\`：生成文档
- \`refactor\`：重构建议
\`\`\``
      },
      hooks: {
        title: 'Hook 系统',
        content: `## 概述

Hook 允许在特定事件时运行自定义脚本。

## Hook 事件

| 事件 | 运行时机 |
|------|----------|
| \`pre_send_message\` | 发送消息给 AI 之前 |
| \`post_llm_response\` | 收到 AI 响应之后 |
| \`pre_tool_execution\` | 工具执行之前 |
| \`post_tool_execution\` | 工具执行之后 |
| \`session_start\` | 会话开始时 |
| \`session_end\` | 会话结束时 |

## 注册 Hook

\`\`\`bash
# 注册 hook
j hook register pre_send_message "echo 'Sending message...'"

# 列出 hook
j hook list

# 删除 hook
j hook remove pre_send_message 0
\`\`\`

## Hook 脚本

Hook 脚本通过 stdin 接收 JSON：

\`\`\`json
{
  "event": "pre_send_message",
  "data": {
    "message": "用户消息"
  }
}
\`\`\`

脚本应通过 stdout 输出 JSON 来修改数据。
\`\`\``
      },
      browser: {
        title: '浏览器自动化',
        content: `## 模式

| 模式 | 描述 |
|------|------|
| **Lite** | 轻量级 HTTP 控制（默认） |
| **CDP** | 通过 Chrome DevTools Protocol 的完整浏览器自动化 |

## Lite 模式

\`\`\`bash
# 启动 lite 模式
j browser lite

# 打开 URL
j browser open https://example.com

# 截图
j browser screenshot
\`\`\`

## CDP 模式

\`\`\`bash
# 启动 CDP（需要 Chrome/Chromium）
j browser cdp

# 导航
j browser goto https://example.com

# 点击元素
j browser click "#submit-button"

# 输入文本
j browser type "#search" "查询"

# 截图
j browser screenshot
\`\`\`

## 功能特性

- 截图捕获
- 元素交互
- 页面导航
- 脚本注入
- Cookie 管理
\`\`\``
      },
      remote: {
        title: '远程控制',
        content: `## 概述

通过 WebSocket 从移动设备控制 AI 对话。

## 设置

\`\`\`bash
# 启动远程控制服务器
j remote start

# 显示连接二维码
j remote qr
\`\`\`

## 功能特性

- **移动控制**：使用手机发送消息
- **语音输入**：语音转文字
- **推送通知**：在手机上接收响应
- **会话同步**：跨设备继续对话

## 安全性

- WebSocket 安全连接
- 基于令牌的认证
- 端到端加密

## 客户端应用

- **Web**：扫描二维码连接
- **iOS**：Shortcuts 集成
- **Android**：Tasker 集成
\`\`\``
      },
      permissions: {
        title: '权限配置',
        content: `## 权限级别

| 级别 | 描述 |
|------|------|
| \`allow\` | 始终允许 |
| \`ask\` | 询问确认 |
| \`deny\` | 始终拒绝 |

## 配置

\`\`\`yaml
# ~/.jdata/agent/data/agent_config.yaml
permissions:
  # 读取操作 - 始终允许
  - tool: Read
    permission: allow
  
  # 写入操作 - 询问确认
  - tool: Write
    permission: ask
  
  # Shell 命令 - 询问确认
  - tool: Bash
    permission: ask
    rules:
      - pattern: "ls *"        # 允许 ls 命令
        permission: allow
      - pattern: "rm *"        # rm 始终询问
        permission: ask
  
  # 网络访问 - 始终允许
  - tool: WebFetch
    permission: allow
  - tool: WebSearch
    permission: allow
\`\`\`

## 细粒度规则

\`\`\`yaml
permissions:
  - tool: Bash
    permission: ask
    rules:
      # 允许特定模式
      - pattern: "git status"
        permission: allow
      - pattern: "cargo build"
        permission: allow
      
      # 拒绝危险模式
      - pattern: "rm -rf /*"
        permission: deny
\`\`\``
      }
    }
  }
}
