import { useState, useEffect } from 'react'
import { Link } from 'react-router-dom'
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter'
import { oneLight } from 'react-syntax-highlighter/dist/esm/styles/prism'

type Lang = 'en' | 'zh'

// Documentation tree structure
const docTree = {
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

const i18n = {
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

| Command | Description |
|---------|-------------|
| \`j note <alias> <category>\` | Mark alias category |
| \`j denote <alias> <category>\` | Remove alias category |

Available categories: \`browser\`, \`editor\`, \`vpn\`, \`outer_url\`, \`script\`

> Marked as browser: use \`j <browser> <url>\` to open links or search
> Marked as editor: use \`j <editor> <file>\` to open files

## List & Find

| Command | Description |
|---------|-------------|
| \`j ls\` | List common aliases (path/url/browser/editor etc.) |
| \`j ls all\` | List all aliases in all sections |
| \`j ls <section>\` | List aliases in specified section |
| \`j contain <alias>\` | Find alias in all categories |
| \`j contain <alias> <sections>\` | Find in specified categories (comma-separated) |

## Open

| Command | Description |
|---------|-------------|
| \`j <alias>\` | Open app/file/URL |
| \`j <browser> <url_alias>\` | Open URL with browser |
| \`j <browser> <text>\` | Search text with browser (default Bing, configurable) |
| \`j <editor> <file>\` | Open file with editor |

> **Smart Detection**: CLI executables run in current terminal (supports piping), GUI apps (.app) open via system`
      },
      report: {
        title: 'Daily Reports',
        content: `## Commands

| Command | Description |
|---------|-------------|
| \`j report <content>\` | Write to report (auto date prefix) |
| \`j reportctl new [date]\` | Start new week (week number +1) |
| \`j reportctl sync [date]\` | Sync week number and date |
| \`j reportctl push [msg]\` | Push to remote git repo |
| \`j reportctl pull\` | Pull from remote git repo |
| \`j reportctl set-url [url]\` | Set/view git repo URL |
| \`j reportctl open\` | Open report file in TUI editor |
| \`j check [N]\` | View recent N lines (default 10) |
| \`j search <N/all> <kw>\` | Search keyword in reports |
| \`j search <N/all> <kw> -f\` | Fuzzy search (case insensitive) |

> Default path: \`~/.jdata/report/week_report.md\`
> Custom path: \`j change report week_report <path>\`
> Configure remote: \`j reportctl set-url <repo_url>\``
      },
      todo: {
        title: 'Todo Management',
        content: `## Commands

| Command | Description |
|---------|-------------|
| \`j todo\` | Enter TUI manager (fullscreen interactive) |
| \`j td\` | Same as above (alias) |
| \`j todo add Buy milk\` | Quick add a todo |
| \`j todo list\` | Output todo list (Markdown rendered) |
| \`j todo list --done\` | Show only completed todos |
| \`j todo list --undone\` | Show only uncompleted todos |

## TUI Keyboard Shortcuts

| Key | Action |
|-----|--------|
| \`n\` / \`↓\` / \`j\` | Move down |
| \`N\` / \`↑\` / \`k\` | Move up |
| \`Space\` / \`Enter\` | Toggle completion \`[x]\` / \`[ ]\` |
| \`a\` | Add new todo |
| \`e\` | Edit selected todo |
| \`d\` | Delete todo (requires confirmation) |
| \`y\` | Copy selected todo to clipboard |
| \`f\` | Filter toggle (all / uncompleted / completed) |
| \`J\` / \`K\` | Reorder (move down / up) |
| \`s\` | Manual save |
| \`Alt+↑\` / \`Alt+↓\` | Scroll preview area |
| \`?\` | Show full help |
| \`q\` | Quit (unsaved changes need save or \`q!\`) |
| \`q!\` | Force quit (discard unsaved changes) |

## Report Integration

When marking a todo as complete, you'll be prompted to write to daily report:

| Action | Result |
|--------|--------|
| \`Space\` / \`Enter\` to complete | Shows confirmation: \`Write to report: "content..."? (Enter/y to write, others skip)\` |
| \`Enter\` / \`y\` / \`Y\` | Write to report + auto save todo |
| Any other key | Mark complete, don't write to report |

> Report format matches \`j report\`: \`- 【YYYY/MM/DD】 content\`
> Data path: \`~/.jdata/report/todo.json\``
      },
      script: {
        title: 'Script System',
        content: `## Commands

| Command | Description |
|---------|-------------|
| \`j concat <name> "<content>"\` | Create script and register as alias (saved to \`~/.jdata/scripts/\`) |
| \`j concat <name>\` | Open TUI editor to modify existing script |
| \`j <script> [args...]\` | Execute script in current terminal |
| \`j <script> -w [args...]\` | Execute in **new terminal window** |
| \`j time countdown <duration>\` | Start countdown (supports 30s / 5m / 1h) |

> \`-w\` or \`--new-window\` flag runs script in new terminal window for background tasks

## Environment Variable Injection

All registered alias paths are automatically injected as environment variables when executing scripts. Naming rule: \`J_<ALIAS_UPPERCASE>\` (\`-\` becomes \`_\`):

\`\`\`bash
#!/bin/bash
# Registered: chrome → /Applications/Google Chrome.app
# Registered: vscode → /Applications/Visual Studio Code.app
# Registered: my-tool → /usr/local/bin/my-tool

open -a "$J_CHROME" https://example.com
"$J_VSCODE" ./src
"$J_MY_TOOL" --version
\`\`\`

> Covers sections: \`path\`, \`inner_url\`, \`outer_url\`, \`script\`
> New window execution (\`-w\`) also supports environment variables
> When paths contain spaces, must use double quotes: \`"$J_CHROME"\` not \`$J_CHROME\``
      },
      aiChat: {
        title: 'AI Chat',
        content: `## Commands

| Command | Description |
|---------|-------------|
| \`j chat\` / \`j ai\` | Enter TUI chat interface (fullscreen) |
| \`j chat Hello\` | Enter chat and send first message |

## Configuration

First run of \`j chat\` will auto-enter config UI if not configured. Press **Ctrl+E** anytime to re-edit.

Config path: \`~/.jdata/agent/data/agent_config.json\`

\`\`\`json
{
  "providers": [
    {
      "name": "GPT-4o",
      "api_base": "https://api.openai.com/v1",
      "api_key": "sk-your-api-key",
      "model": "gpt-4o"
    }
  ],
  "active_index": 0,
  "system_prompt": "You are a helpful assistant.",
  "stream_mode": true,
  "max_history_messages": 20,
  "theme": "dark",
  "tools_enabled": true
}
\`\`\`

> Supports multiple providers, switch with \`Ctrl+T\` in chat

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| \`Enter\` | Send message |
| \`↑\` / \`↓\` | Scroll chat history |
| \`PageUp\` / \`PageDown\` | Fast scroll (10 lines) |
| \`←\` / \`→\` | Move input cursor |
| \`Home\` / \`End\` | Jump to line start/end |
| \`Ctrl+T\` | Switch model provider |
| \`Ctrl+L\` | Archive current conversation |
| \`Ctrl+R\` | Restore archived conversation |
| \`Ctrl+Y\` | Copy last AI reply |
| \`Ctrl+B\` | Enter message browse mode |
| \`Ctrl+S\` | Toggle stream/blocking output |
| \`Ctrl+E\` | Open config UI |
| \`?\` | Show help |
| \`Esc\` / \`Ctrl+C\` | Exit chat |

## Features

- **Markdown Rendering**: Headers, bold, italic, inline code, code blocks (syntax highlighting), lists, tables, blockquotes
- **Code Highlighting**: Rust, Python, JavaScript/TypeScript, Go, Java, Bash/Shell, C/C++, SQL, Ruby, etc.
- **Stream/Blocking Output**: Default streaming, toggle with \`Ctrl+S\`
- **Persistence**: Auto-save to \`~/.jdata/agent/data/chat_session.json\`
- **Multi-model Support**: Configure multiple LLM providers (OpenAI, DeepSeek, etc.)
- **Tool Calling**: Supports Function Calling, AI can execute shell commands and read files
- **Context Compact**: Three-layer conversation compression mechanism`
      },
      agentMode: {
        title: 'Agent Mode',
        content: `Agent mode enables AI with autonomous execution capabilities, automatically calling tools to complete complex tasks.

## Core Capabilities

- **Multi-step Reasoning** — Autonomously plan task steps, execute progressively
- **Tool Calling** — Automatically select and execute Bash/Read/Write/Edit/Grep etc.
- **Context Management** — Auto-compress conversation history, maintain long conversation coherence
- **Background Tasks** — Support background execution of long commands without blocking

## Workflow

\`\`\`
User: "Help me analyze this project's code structure"
  ↓
Agent: 1. Execute Bash(ls -la) to view directory
       2. Execute Glob(**/*.rs) to find source files
       3. Execute Read to read key files
       4. Summarize analysis results
  ↓
Return complete analysis report
\`\`\`

## Configuration (in \`agent_config.json\`)

| Field | Default | Description |
|-------|---------|-------------|
| \`tools_enabled\` | \`true\` | Enable tool calling |
| \`max_tool_rounds\` | \`50\` | Max tool call rounds per conversation |
| \`compact.enabled\` | \`true\` | Enable conversation compression |
| \`compact.token_threshold\` | \`204800\` | Token threshold to trigger compression |
| \`compact.keep_recent\` | \`10\` | Keep recent N tool results |

## Permission Control

Before executing sensitive operations (Bash/Write/Edit), Agent requests confirmation. Configure auto-execution permissions via \`.jcli\` file:

\`\`\`yaml
permissions:
  allow:
    - "Read"
    - "Glob"
    - "Grep"
    - "Bash(cargo build:*)"
\`\`\``
      },
      tools: {
        title: 'AI Tools',
        content: `## Built-in Tools

| Tool | Description | Needs Confirm |
|------|-------------|---------------|
| \`Bash\` | Execute shell commands | Yes |
| \`Read\` | Read local files (supports line range) | |
| \`Write\` | Write file (auto-create directories) | Yes |
| \`Edit\` | Edit file (exact string replacement) | Yes |
| \`Glob\` | Pattern match search filenames | |
| \`Grep\` | Regex search file contents | |
| \`Ask\` | Ask user structured choice questions | |
| \`WebFetch\` | Fetch webpage and convert to Markdown/text | |
| \`WebSearch\` | Search web using Exa Search API | |
| \`Browser\` | Browser automation (CDP + Lite fallback) | |
| \`BackgroundRun\` | Execute shell command in background | Yes |
| \`CheckBackground\` | Query background task status and result | |
| \`LoadSkill\` | Load specified skill to context | |
| \`Compact\` | Trigger conversation compression | |
| \`TaskCreate\` | Create task | |
| \`TaskList\` | List all tasks | |
| \`TaskGet\` | Get task details | |
| \`TaskUpdate\` | Update task status/dependencies | |
| \`RegisterHook\` | Register/manage session-level hooks | Yes |

## WebFetch Parameters

| Parameter | Description |
|-----------|-------------|
| \`url\` | Target URL (required) |
| \`extract_mode\` | Output format: \`markdown\` (default) or \`text\` |
| \`max_chars\` | Max return characters (default 50000) |
| \`authorization\` | Authorization request header |
| \`headers\` | Custom request headers |

## WebSearch Parameters

| Parameter | Description |
|-----------|-------------|
| \`query\` | Search keywords (required) |
| \`count\` | Number of results (default 5, max 10) |
| \`country\` | Search country code (default CN) |
| \`search_lang\` | Search language code (e.g. zh-hans, en) |
| \`freshness\` | Time range: \`pd\`(24h) \`pw\`(week) \`pm\`(month) \`py\`(year) |

## Tool Confirmation

| Key | Action |
|-----|--------|
| \`Y\` / \`Enter\` | Execute tool |
| \`N\` / \`Esc\` | Reject execution |

> \`Bash\` tool has built-in dangerous command filtering (e.g. \`rm -rf /\`), but still recommend checking command content before execution`
      },
      skills: {
        title: 'Skill System',
        content: `Create skill directories under \`~/.jdata/agent/skills/\`, AI loads skills on demand via \`load_skill\` tool.

System prompt only includes skill name and description summary. AI calls \`load_skill\` to load full instructions when needed.

## Template Placeholders

| Placeholder | Replacement |
|-------------|-------------|
| \`{{.current_dir}}\` | Absolute path of current working directory |
| \`{{.skills}}\` | Summary list of all skills (name + description) |
| \`{{.skill_dir}}\` | Absolute path of skills directory |
| \`{{.tools}}\` | Summary list of all tools |
| \`{{.style}}\` | Response style config content |
| \`{{.memory}}\` | Memory content |
| \`{{.soul}}\` | Soul/personality setting |

## Create Skill

\`\`\`bash
mkdir -p ~/.jdata/agent/skills/my-skill
cat > ~/.jdata/agent/skills/my-skill/SKILL.md << 'EOF'
---
name: my-skill
description: Skill description
argument-hint: "[argument description]"
---

Instruction body, $ARGUMENTS will be replaced with argument...
EOF
\`\`\`

## Usage

| Action | Description |
|--------|-------------|
| Type \`@\` | Show skill selection list (supports filtering) |
| \`↑↓\` select + \`Tab/Enter\` | Autocomplete skill name |
| \`@skill argument\` + send | AI recognizes from skill summary then calls \`load_skill\` |
| Enable \`tools_enabled\` | AI can autonomously decide to load skills based on summary |

> Skill directory supports \`references/\` subdirectory for reference files, automatically appended to context`
      },
      hooks: {
        title: 'Hook System',
        content: `Hooks allow injecting custom scripts at key operation points, supporting three-level configuration:

## Three-Level Hooks

1. **User Level**: \`~/.jdata/agent/hooks.yaml\` — Global effect
2. **Project Level**: \`hooks\` field in \`.jcli\` file — Effective in project directory
3. **Session Level**: Dynamically registered by AI via \`register_hook\` tool — Current session only

**Execution Order**: User → Project → Session, chain execution. Former output affects latter input, any \`abort\` immediately stops.

## Available Events

| Event | Trigger | Operable Data |
|-------|---------|---------------|
| \`pre_send_message\` | Before user sends message | user_input, messages |
| \`post_send_message\` | After user sends message | user_input, messages |
| \`pre_llm_request\` | Before LLM API request | messages, system_prompt, model |
| \`post_llm_response\` | After LLM response | assistant_output, messages |
| \`pre_tool_execution\` | Before tool execution | tool_name, tool_arguments |
| \`post_tool_execution\` | After tool execution | tool_name, tool_result |
| \`session_start\` | Session start | messages |
| \`session_end\` | Session end | messages |

## User Level Config (\`~/.jdata/agent/hooks.yaml\`)

\`\`\`yaml
pre_send_message:
  - command: "python3 ~/.jdata/agent/hooks/inject_time.py"
    timeout: 5
pre_llm_request:
  - command: "~/.jdata/agent/hooks/add_context.sh"
session_start:
  - command: "echo '{\"inject_messages\": [{\"role\": \"user\", \"content\": \"Current user: jack\"}]}'"
\`\`\`

## Script Protocol

- **Execution**: \`sh -c "<command>"\`, working directory is user's current directory
- **Environment Variables**: \`JCLI_HOOK_EVENT\` (event name), \`JCLI_CWD\` (current directory)
- **stdin**: HookContext JSON
- **stdout**: HookResult JSON (can be empty/empty JSON to indicate no modification)
- **exit 0**: Success; non-zero exit: treated as abort
- **Timeout**: Default 10 seconds, kill child process after timeout`
      },
      browser: {
        title: 'Browser Automation',
        content: `jcli supports two browser automation modes:

## Lite Mode (Default)

Lightweight HTTP-based browser simulation, no Chrome installation required.

**Features:**
- Tab management
- Page interactive element recognition (snapshot)
- Link/form extraction
- Content retrieval

## CDP Mode

Uses real Chrome/Chromium browser via Chrome DevTools Protocol.

**Features:**
- Screenshots
- Click & Type
- Press keys
- Execute JavaScript
- Full DOM access

**Requirements:**
- Build with: \`cargo build --release --features browser_cdp\`
- Local Chrome or Chromium installation

## Browser Tool Actions

| Action | Description | Required Parameters |
|--------|-------------|---------------------|
| \`start\` | Start browser | None |
| \`stop\` | Stop browser | None |
| \`status\` | View browser status | None |
| \`tabs\` | List open tabs | None |
| \`open\` | Open URL to new tab | \`url\` |
| \`navigate\` | Navigate tab to new URL | \`url\`, \`tab_id\` (optional) |
| \`screenshot\` | Screenshot (requires CDP) | \`tab_id\` (optional), \`full_page\` (optional) |
| \`snapshot\` | Get page interactive elements | \`tab_id\` (optional) |
| \`content\` | Get page text content | \`tab_id\` (optional) |
| \`close\` | Close tab | \`tab_id\` |
| \`click\` | Click element (requires CDP) | \`selector\` |
| \`type\` | Type text (requires CDP) | \`selector\`, \`text\` |
| \`press\` | Press key (requires CDP) | \`key\` |
| \`evaluate\` | Execute JavaScript (requires CDP) | \`script\` |

## FAQ

**Q: How to compile with CDP?**
A: \`cargo build --release --features browser_cdp\` or \`cargo install j-cli --features browser_cdp\`

**Q: What extra dependencies needed?**
A: **CDP mode** requires Chrome/Chromium installed; **Lite mode** has no extra dependencies.

**Q: Will browser close on exit?**
A: Yes. Chrome process terminates automatically on normal exit.`
      },
      remote: {
        title: 'Remote Control',
        content: `\`j ai --remote\` enables remote control. Terminal displays QR code, scan with phone to connect:

## Features

- Mobile browser access, no app installation needed
- Real-time conversation sync
- Send messages, receive replies
- Use within same LAN, safe and reliable

## Usage

| Key | Action |
|-----|--------|
| \`Ctrl+R\` | Start remote control server, show QR code |
| Scan QR code | Open Sprite Remote web client |
| Connected | Terminal shows "Connected", ready for remote operation |

## Security

- **Token Verification**: Random UUID token generated each startup
- **LAN Restriction**: Only accessible within same network
- **Connection Status**: Terminal displays connect/disconnect status in real-time`
      },
      permissions: {
        title: 'Permission Configuration',
        content: `Create \`.jcli\` file in project root (YAML format) to control auto-execution permissions for tools in \`j chat\`. Program searches upward from current directory.

## Configuration Example

\`\`\`yaml
permissions:
  # Allow all (skip all tool confirmations)
  # allow_all: true

  allow:
    # Bash command prefix matching (:* means any argument suffix)
    - "Bash(cargo build:*)"
    - "Bash(cargo test:*)"
    - "Bash(cargo fmt:*)"
    - "Bash(git status:*)"
    - "Bash(ls:*)"

    # Tool level: allow all calls to this tool
    - "Read"
    - "Glob"
    - "Grep"

    # File write restricted to specific directory
    - "Write(path:/Users/jack/projects/*)"
    - "Edit(path:/Users/jack/projects/*)"

    # WebFetch restricted domain
    - "WebFetch(domain:docs.rs)"

  deny:
    # Blacklist (takes priority over allow)
    - "Bash(rm -rf:*)"
    - "Bash(sudo:*)"
\`\`\`

## Matching Rules

| Rule Format | Description | Example |
|-------------|-------------|---------|
| \`*\` | Match all tools all calls | \`"*"\` |
| \`ToolName\` | Match all calls to this tool | \`"Read"\`, \`"Grep"\` |
| \`Bash(cmd:*)\` | Bash command prefix matching | \`"Bash(cargo build:*)"\` |
| \`Write(path:dir/*)\` | File path prefix matching | \`"Write(path:/home/user/*)"\` |
| \`WebFetch(domain:x)\` | URL domain matching | \`"WebFetch(domain:docs.rs)"\` |

- No \`.jcli\` file: Default behavior (tools needing confirmation show confirmation dialog)
- \`deny\` takes priority over \`allow\`
- \`allow_all: true\` or \`"*"\` in allow: All tools skip confirmation`
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
# 标准版（Lite 浏览器模式，无额外依赖）
cargo install j-cli

# 完整版（CDP 浏览器模式，需本地已安装 Chrome/Chromium）
cargo install j-cli --features browser_cdp
\`\`\`

## 从源码编译

\`\`\`bash
git clone https://github.com/LingoJack/j.git
cd j && cargo install --path .

# 启用完整浏览器自动化
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
# 使用安装脚本卸载（推荐）
curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh -s -- --uninstall

# 或通过 cargo 卸载
cargo uninstall j-cli

# 或手动删除
sudo rm /usr/local/bin/j  # 一键安装方式
rm ~/.cargo/bin/j          # cargo 安装方式

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

# 注册 URL 别名（自动识别为 inner_url）
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
j chrome github           # 用 Chrome 打开 github 对应的 URL
j chrome "rust lang"      # 用 Chrome 搜索 "rust lang"
j vscode ./src            # 用 VSCode 打开 src 目录
\`\`\`

## 写日报 & 查看

\`\`\`bash
j report "完成功能开发"    # 写入今日日报
j check                   # 查看最近 10 行
j check 20                # 查看最近 20 行
\`\`\`

## 待办

\`\`\`bash
j todo add 买牛奶         # 快速添加待办
j todo                    # 进入 TUI 管理
\`\`\`

## AI 对话

\`\`\`bash
j chat                    # 进入 TUI 对话界面
j chat 你好               # 快速提问
\`\`\`

## 进入交互模式

\`\`\`bash
j                         # 进入交互模式（带 Tab 补全 + 历史建议）
\`\`\``
      },
      dataDirectory: {
        title: '数据目录',
        content: `所有数据统一存储在 \`~/.jdata/\` 下（可通过环境变量 \`J_DATA_PATH\` 自定义）：

\`\`\`
~/.jdata/
├── config.yaml          # 主配置文件（别名、分类、设置等）
├── agent/               # AI Agent 相关数据
│   ├── data/            # Agent 数据目录
│   │   ├── agent_config.json   # Agent 配置（模型、API 等）
│   │   ├── chat_history.json   # 对话历史
│   │   ├── archives/           # 归档对话
│   │   ├── system_prompt.md    # 系统提示词
│   │   ├── memory.md           # 记忆文件
│   │   ├── soul.md             # 灵魂文件
│   │   └── style.md            # 回复风格配置
│   ├── logs/            # Agent 日志
│   │   ├── info.log
│   │   └── error.log
│   └── skills/          # 技能目录
├── bin/                 # 内置工具
│   └── md_render        # Markdown 渲染器
├── report/              # 日报目录
│   ├── week_report.md   # 周报文件
│   ├── settings.json    # 日报配置（周数、日期）
│   ├── todo.json        # 待办数据
│   └── .git/            # git 仓库
├── scripts/             # j concat 创建的脚本
\`\`\`

## 配置文件结构 (\`config.yaml\`)

| Section | 说明 | 示例 |
|---------|------|------|
| \`path\` | 本地应用/文件路径 | \`chrome: /Applications/Google Chrome.app\` |
| \`inner_url\` | URL 链接 | \`github: https://github.com\` |
| \`outer_url\` | 需 VPN 的外网 URL | \`docs: https://internal.example.com\` |
| \`browser\` | 浏览器列表 | \`chrome: chrome\` |
| \`editor\` | 编辑器列表 | \`vscode: vscode\` |
| \`vpn\` | VPN 应用 | |
| \`script\` | 已注册的脚本 | \`deploy: ~/.jdata/scripts/deploy.sh\` |
| \`report\` | 日报系统配置 | \`git_repo: https://github.com/xxx/report\` |
| \`setting\` | 全局设置 | \`search-engine: bing\` |
| \`log\` | 日志设置 | \`mode: concise\` |`
      },
      alias: {
        title: '别名管理',
        content: `## 命令

| 命令 | 说明 |
|------|------|
| \`j set <alias> <path>\` | 设置别名（路径自动归类到 path，URL 归类到 inner_url） |
| \`j rm <alias>\` | 删除别名（同时清理关联的分类标记） |
| \`j rename <alias> <new>\` | 重命名别名（同步更新所有分类引用） |
| \`j mf <alias> <new_path>\` | 修改别名指向的路径 |

## 分类标记

| 命令 | 说明 |
|------|------|
| \`j note <alias> <category>\` | 标记别名分类 |
| \`j denote <alias> <category>\` | 解除别名分类 |

可用分类: \`browser\`, \`editor\`, \`vpn\`, \`outer_url\`, \`script\`

> 标记为 browser 后可以用 \`j <browser> <url>\` 打开链接或搜索
> 标记为 editor 后可以用 \`j <editor> <file>\` 打开文件

## 列表 & 查找

| 命令 | 说明 |
|------|------|
| \`j ls\` | 列出常用别名（path/url/browser/editor 等） |
| \`j ls all\` | 列出所有 section 下的别名 |
| \`j ls <section>\` | 列出指定 section（如 \`j ls path\`） |
| \`j contain <alias>\` | 在所有分类中查找别名 |
| \`j contain <alias> <sections>\` | 在指定分类中查找（逗号分隔） |

## 打开

| 命令 | 说明 |
|------|------|
| \`j <alias>\` | 打开应用/文件/URL |
| \`j <browser> <url_alias>\` | 用浏览器打开 URL |
| \`j <browser> <text>\` | 用浏览器搜索（默认 Bing，可配置） |
| \`j <editor> <file>\` | 用编辑器打开文件 |

> **智能识别**：CLI 可执行文件在当前终端执行（支持管道），GUI 应用(.app)用系统打开`
      },
      report: {
        title: '日报系统',
        content: `## 命令

| 命令 | 说明 |
|------|------|
| \`j report <content>\` | 写入日报（自动追加日期前缀） |
| \`j reportctl new [date]\` | 开启新的一周（周数+1） |
| \`j reportctl sync [date]\` | 同步周数和日期 |
| \`j reportctl push [msg]\` | 推送周报到远程 git 仓库 |
| \`j reportctl pull\` | 从远程 git 仓库拉取周报 |
| \`j reportctl set-url [url]\` | 设置/查看 git 仓库地址 |
| \`j reportctl open\` | 用内置 TUI 编辑器打开日报文件全文编辑 |
| \`j check [N]\` | 查看日报最近 N 行（默认 10） |
| \`j search <N/all> <kw>\` | 在日报中搜索关键字 |
| \`j search <N/all> <kw> -f\` | 模糊搜索（大小写不敏感） |

> 日报默认路径: \`~/.jdata/report/week_report.md\`
> 自定义路径: \`j change report week_report <path>\`
> 配置远程仓库: \`j reportctl set-url <repo_url>\``
      },
      todo: {
        title: '待办管理',
        content: `## 命令

| 命令 | 说明 |
|------|------|
| \`j todo\` | 进入 TUI 待办管理界面（全屏交互） |
| \`j td\` | 同上（别名） |
| \`j todo add 买牛奶\` | 快速添加一条待办 |
| \`j todo list\` | 输出待办列表（Markdown 渲染） |
| \`j todo list --done\` | 仅显示已完成的待办 |
| \`j todo list --undone\` | 仅显示未完成的待办 |

## TUI 界面快捷键

| 按键 | 功能 |
|------|------|
| \`n\` / \`↓\` / \`j\` | 向下移动 |
| \`N\` / \`↑\` / \`k\` | 向上移动 |
| \`空格\` / \`回车\` | 切换完成状态 \`[x]\` / \`[ ]\` |
| \`a\` | 添加新待办 |
| \`e\` | 编辑选中待办 |
| \`d\` | 删除待办（需确认） |
| \`y\` | 复制选中待办到系统剪切板 |
| \`f\` | 过滤切换（全部 / 未完成 / 已完成） |
| \`J\` / \`K\` | 调整待办顺序（下移 / 上移） |
| \`s\` | 手动保存 |
| \`Alt+↑\` / \`Alt+↓\` | 预览区滚动 |
| \`?\` | 查看完整帮助 |
| \`q\` | 退出（有未保存修改时需先保存或用 \`q!\` 强制退出） |
| \`q!\` | 强制退出（丢弃未保存的修改） |

## 完成时写入日报联动

标记完成时自动询问是否写入日报：

| 操作 | 效果 |
|------|------|
| \`空格\` / \`回车\` 标记完成 | 底部显示确认提示：\`写入日报: "内容..."？ (Enter/y 写入, 其他跳过)\` |
| \`Enter\` / \`y\` / \`Y\` | 写入日报 + 自动保存 todo |
| 其他任意键 | 标记完成，不写入日报 |

> 写入日报的格式与 \`j report\` 命令一致：\`- 【YYYY/MM/DD】 内容\`
> 数据存储路径: \`~/.jdata/report/todo.json\``
      },
      script: {
        title: '脚本系统',
        content: `## 命令

| 命令 | 说明 |
|------|------|
| \`j concat <name> "<content>"\` | 创建脚本并注册为别名（保存到 \`~/.jdata/scripts/\`） |
| \`j concat <name>\` | 脚本已存在时打开 TUI 编辑器修改脚本内容 |
| \`j <script> [args...]\` | 在当前终端执行脚本 |
| \`j <script> -w [args...]\` | 在**新终端窗口**中执行脚本 |
| \`j time countdown <duration>\` | 启动倒计时（支持 30s / 5m / 1h） |

> \`-w\` 或 \`--new-window\` 标志可让脚本在新终端窗口中执行，用于需要后台运行的场景

## 脚本环境变量注入

执行脚本时，所有已注册的别名路径会自动注入为环境变量，命名规则为 \`J_<别名大写>\`（\`-\` 转为 \`_\`）：

\`\`\`bash
#!/bin/bash
# 已注册: chrome → /Applications/Google Chrome.app
# 已注册: vscode → /Applications/Visual Studio Code.app
# 已注册: my-tool → /usr/local/bin/my-tool

open -a "$J_CHROME" https://example.com
"$J_VSCODE" ./src
"$J_MY_TOOL" --version
\`\`\`

> 覆盖 section: \`path\`、\`inner_url\`、\`outer_url\`、\`script\`
> 新窗口执行（\`-w\`）同样支持环境变量注入
> 路径含空格时，脚本中必须用双引号包裹变量：\`"$J_CHROME"\` 而非 \`$J_CHROME\``
      },
      aiChat: {
        title: 'AI 对话',
        content: `## 命令

| 命令 | 说明 |
|------|------|
| \`j chat\` / \`j ai\` | 进入 TUI 对话界面（全屏交互） |
| \`j chat 你好\` | 进入对话并发送首条消息 |

## 配置

首次运行 \`j chat\` 时，若尚未配置模型提供方，会自动进入内置配置界面完成初始配置。已有配置后，也可随时在对话界面中按 **Ctrl+E** 重新编辑。

配置文件路径: \`~/.jdata/agent/data/agent_config.json\`

\`\`\`json
{
  "providers": [
    {
      "name": "GPT-4o",
      "api_base": "https://api.openai.com/v1",
      "api_key": "sk-your-api-key",
      "model": "gpt-4o"
    }
  ],
  "active_index": 0,
  "system_prompt": "你是一个有用的助手。",
  "stream_mode": true,
  "max_history_messages": 20,
  "theme": "dark",
  "tools_enabled": true
}
\`\`\`

> 支持配置多个模型提供方，可在对话中通过 \`Ctrl+T\` 切换

## 对话界面快捷键

| 按键 | 功能 |
|------|------|
| \`Enter\` | 发送消息 |
| \`↑\` / \`↓\` | 滚动对话记录 |
| \`PageUp\` / \`PageDown\` | 快速滚动（10行） |
| \`←\` / \`→\` | 移动输入光标 |
| \`Home\` / \`End\` | 跳到输入行首/行尾 |
| \`Ctrl+T\` | 切换模型提供方 |
| \`Ctrl+L\` | 归档当前对话 |
| \`Ctrl+R\` | 还原归档对话 |
| \`Ctrl+Y\` | 复制最后一条 AI 回复 |
| \`Ctrl+B\` | 进入消息浏览模式 |
| \`Ctrl+S\` | 切换流式/整体输出 |
| \`Ctrl+E\` | 打开配置界面 |
| \`?\` | 显示帮助 |
| \`Esc\` / \`Ctrl+C\` | 退出对话 |

## 功能特性

- **Markdown 渲染**：AI 回复支持标题、加粗、斜体、行内代码、代码块（语法高亮）、列表、表格、引用块
- **代码高亮**：支持 Rust、Python、JavaScript/TypeScript、Go、Java、Bash/Shell、C/C++、SQL、Ruby 等语言
- **流式/整体输出**：默认流式逐字输出，可通过 \`Ctrl+S\` 切换
- **对话持久化**：对话自动保存到 \`~/.jdata/agent/data/chat_session.json\`
- **多模型支持**：可配置多个 LLM 提供方（OpenAI、DeepSeek 等），运行时切换
- **工具调用**：支持 Function Calling，AI 可执行 shell 命令和读取文件
- **Context Compact**：三层对话压缩机制`
      },
      agentMode: {
        title: 'Agent 模式',
        content: `Agent 模式让 AI 具备自主执行能力，可自动调用工具完成复杂任务：

## 核心能力

- **多步推理** — 自主规划任务步骤，逐步执行直至完成
- **工具调用** — 自动选择并执行 Bash/Read/Write/Edit/Grep 等工具
- **上下文管理** — 自动压缩对话历史，保持长对话连贯
- **后台任务** — 支持后台执行长时间命令，不阻塞对话

## 工作流程

\`\`\`
用户: "帮我分析这个项目的代码结构"
  ↓
Agent: 1. 执行 Bash(ls -la) 查看目录
       2. 执行 Glob(**/*.rs) 找到源文件
       3. 执行 Read 读取关键文件
       4. 汇总分析结果
  ↓
返回完整分析报告
\`\`\`

## 配置项（在 \`agent_config.json\` 中）

| 字段 | 默认值 | 说明 |
|------|--------|------|
| \`tools_enabled\` | \`true\` | 启用工具调用 |
| \`max_tool_rounds\` | \`50\` | 单次对话最大工具调用轮数 |
| \`compact.enabled\` | \`true\` | 启用对话压缩 |
| \`compact.token_threshold\` | \`204800\` | 触发压缩的 token 阈值 |
| \`compact.keep_recent\` | \`10\` | 保留最近 N 个工具结果 |

## 权限控制

Agent 执行敏感操作（Bash/Write/Edit）前会请求确认，可通过 \`.jcli\` 文件配置自动执行权限：

\`\`\`yaml
permissions:
  allow:
    - "Read"
    - "Glob"
    - "Grep"
    - "Bash(cargo build:*)"
\`\`\``
      },
      tools: {
        title: 'AI 工具',
        content: `## 内置工具

| 工具名 | 功能 | 需确认 |
|--------|------|--------|
| \`Bash\` | 执行 shell 命令 | Yes |
| \`Read\` | 读取本地文件（支持行号范围） | |
| \`Write\` | 写入文件（自动创建目录） | Yes |
| \`Edit\` | 编辑文件（精确字符串替换） | Yes |
| \`Glob\` | 按模式匹配搜索文件名 | |
| \`Grep\` | 正则搜索文件内容 | |
| \`Ask\` | 向用户提结构化选择题 | |
| \`WebFetch\` | 获取网页内容并转为 Markdown/纯文本 | |
| \`WebSearch\` | 使用 Exa Search API 搜索网络 | |
| \`Browser\` | 浏览器自动化（CDP + Lite fallback） | |
| \`BackgroundRun\` | 后台执行 shell 命令 | Yes |
| \`CheckBackground\` | 查询后台任务状态和结果 | |
| \`LoadSkill\` | 加载指定技能到上下文 | |
| \`Compact\` | 触发对话压缩 | |
| \`TaskCreate\` | 创建任务 | |
| \`TaskList\` | 列出所有任务 | |
| \`TaskGet\` | 获取任务详情 | |
| \`TaskUpdate\` | 更新任务状态/依赖 | |
| \`RegisterHook\` | 注册/管理 session 级 hook | Yes |

## WebFetch 参数

| 参数 | 说明 |
|------|------|
| \`url\` | 目标 URL（必需） |
| \`extract_mode\` | 输出格式：\`markdown\`（默认）或 \`text\` |
| \`max_chars\` | 最大返回字符数（默认 50000） |
| \`authorization\` | Authorization 请求头 |
| \`headers\` | 自定义请求头 |

## WebSearch 参数

| 参数 | 说明 |
|------|------|
| \`query\` | 搜索关键词（必需） |
| \`count\` | 搜索结果数量（默认 5，最大 10） |
| \`country\` | 搜索国家代码（默认 CN） |
| \`search_lang\` | 搜索语言代码（如 zh-hans、en） |
| \`freshness\` | 时间范围：\`pd\`(24h) \`pw\`(一周) \`pm\`(一月) \`py\`(一年) |

## 工具确认快捷键

| 按键 | 功能 |
|------|------|
| \`Y\` / \`Enter\` | 执行工具 |
| \`N\` / \`Esc\` | 拒绝执行 |

> \`Bash\` 工具内置危险命令过滤（如 \`rm -rf /\`），但仍建议执行前检查命令内容`
      },
      skills: {
        title: 'Skill 技能',
        content: `在 \`~/.jdata/agent/skills/\` 下创建 skill 目录，AI 通过 \`load_skill\` 工具按需加载技能。

系统提示词中仅包含技能的名称和描述摘要，AI 判断需要时调用 \`load_skill\` 加载完整指令。

## 系统提示词模板占位符

| 占位符 | 替换内容 |
|--------|----------|
| \`{{.current_dir}}\` | 当前工作目录的绝对路径 |
| \`{{.skills}}\` | 所有技能的 name + description 摘要列表 |
| \`{{.skill_dir}}\` | 技能目录的绝对路径 |
| \`{{.tools}}\` | 所有工具的 name + description 摘要列表 |
| \`{{.style}}\` | 回复风格配置内容 |
| \`{{.memory}}\` | 记忆内容 |
| \`{{.soul}}\` | 灵魂/人格设定 |

## 创建 Skill

\`\`\`bash
mkdir -p ~/.jdata/agent/skills/my-skill
cat > ~/.jdata/agent/skills/my-skill/SKILL.md << 'EOF'
---
name: my-skill
description: 技能描述
argument-hint: "[参数说明]"
---

指令正文，$ARGUMENTS 会被替换为参数...
EOF
\`\`\`

## 使用方式

| 操作 | 说明 |
|------|------|
| 输入 \`@\` | 弹出技能选择列表（支持过滤） |
| \`↑↓\` 选择 + \`Tab/Enter\` | 补全技能名称 |
| \`@skill 参数\` + 发送 | AI 从 skills 摘要识别后调用 \`load_skill\` |
| 启用 tools_enabled | AI 可根据 skills 摘要自主决定是否加载技能 |

> Skill 目录支持 \`references/\` 子目录存放参考文件，会自动附加到上下文`
      },
      hooks: {
        title: 'Hook 系统',
        content: `Hook 允许在关键操作节点注入自定义脚本，支持三级配置：

## 三级 Hook

1. **用户级**：\`~/.jdata/agent/hooks.yaml\` — 全局生效
2. **项目级**：\`.jcli\` 文件的 \`hooks\` 字段 — 项目目录下生效
3. **Session 级**：通过 \`register_hook\` 工具由 AI 动态注册 — 仅当前会话

**执行顺序**：用户级 → 项目级 → Session 级，链式执行。前者输出影响后者输入，任何 \`abort\` 立即中止。

## 可用事件

| 事件 | 触发时机 | 可操作数据 |
|------|----------|------------|
| \`pre_send_message\` | 用户发送消息前 | user_input, messages |
| \`post_send_message\` | 用户发送消息后 | user_input, messages |
| \`pre_llm_request\` | LLM API 请求前 | messages, system_prompt, model |
| \`post_llm_response\` | LLM 回复完成后 | assistant_output, messages |
| \`pre_tool_execution\` | 工具执行前 | tool_name, tool_arguments |
| \`post_tool_execution\` | 工具执行后 | tool_name, tool_result |
| \`session_start\` | 会话启动时 | messages |
| \`session_end\` | 会话退出时 | messages |

## 用户级配置

\`\`\`yaml
pre_send_message:
  - command: "python3 ~/.jdata/agent/hooks/inject_time.py"
    timeout: 5
pre_llm_request:
  - command: "~/.jdata/agent/hooks/add_context.sh"
session_start:
  - command: "echo '{\"inject_messages\": [{\"role\": \"user\", \"content\": \"当前用户: jack\"}]}'"
\`\`\`

## 脚本协议

- **执行方式**：\`sh -c "<command>"\`，工作目录为用户当前目录
- **环境变量**：\`JCLI_HOOK_EVENT\`（事件名）、\`JCLI_CWD\`（当前目录）
- **stdin**：HookContext JSON
- **stdout**：HookResult JSON（可为空/空 JSON 表示无修改）
- **exit 0**：成功；非零退出：视为 abort
- **超时**：默认 10 秒，超时后 kill 子进程`
      },
      browser: {
        title: '浏览器自动化',
        content: `jcli 支持两种浏览器自动化模式：

## Lite 模式（默认）

轻量级 HTTP 请求 + HTML 解析，无需安装 Chrome。

**功能：**
- 标签页管理
- 页面交互元素识别（snapshot）
- 链接/表单提取
- 内容获取

## CDP 模式

通过 Chrome DevTools Protocol 完整控制浏览器。

**功能：**
- 截图
- 点击与输入
- 按键
- 执行 JavaScript
- 完整 DOM 访问

**要求：**
- 编译方式：\`cargo build --release --features browser_cdp\`
- 本地需安装 Chrome 或 Chromium

## 浏览器工具 action

| action | 说明 | 必需参数 |
|--------|------|----------|
| \`start\` | 启动浏览器 | 无 |
| \`stop\` | 停止浏览器 | 无 |
| \`status\` | 查看浏览器状态 | 无 |
| \`tabs\` | 列出已打开的标签页 | 无 |
| \`open\` | 打开 URL 到新标签页 | \`url\` |
| \`navigate\` | 导航标签页到新 URL | \`url\`，\`tab_id\`（可选） |
| \`screenshot\` | 截图（需 CDP） | \`tab_id\`（可选），\`full_page\`（可选） |
| \`snapshot\` | 获取页面可交互元素列表 | \`tab_id\`（可选） |
| \`content\` | 获取页面文本内容 | \`tab_id\`（可选） |
| \`close\` | 关闭标签页 | \`tab_id\` |
| \`click\` | 点击元素（需 CDP） | \`selector\` |
| \`type\` | 输入文本（需 CDP） | \`selector\`，\`text\` |
| \`press\` | 按键（需 CDP） | \`key\` |
| \`evaluate\` | 执行 JavaScript（需 CDP） | \`script\` |

## FAQ

**Q: 怎么编译带 CDP 的版本？**
A: \`cargo build --release --features browser_cdp\` 或 \`cargo install j-cli --features browser_cdp\`

**Q: 用户需要额外安装什么？**
A: **CDP 模式**需要本地已安装 Chrome 或 Chromium；**Lite 模式**无任何额外依赖。

**Q: 程序退出时浏览器会关闭吗？**
A: 会。正常退出时 Chrome 进程自动终止。`
      },
      remote: {
        title: '远程控制',
        content: `\`j ai --remote\` 启用远程控制，终端会显示二维码，手机扫码即可连接：

## 功能特性

- 手机浏览器访问，无需安装 App
- 实时同步对话内容
- 支持发送消息、接收回复
- 同一局域网内使用，安全可靠

## 使用方式

| 按键 | 功能 |
|------|------|
| \`Ctrl+R\` | 启动远程控制服务器，显示二维码 |
| 手机扫码 | 打开 Sprite Remote 网页客户端 |
| 连接成功 | 终端显示"已连接"，可开始远程操作 |

## 安全机制

- **Token 验证**：每次启动生成随机 UUID token
- **局域网限制**：仅同一网络内可访问
- **连接状态提示**：终端实时显示连接/断开状态`
      },
      permissions: {
        title: '权限配置',
        content: `在项目根目录创建 \`.jcli\` 文件（YAML 格式），可细粒度控制 \`j chat\` 中工具的自动执行权限。程序会从当前目录向上查找 \`.jcli\` 文件。

## 配置示例

\`\`\`yaml
permissions:
  # 完全放开（跳过所有工具确认）
  # allow_all: true

  allow:
    # Bash 命令前缀匹配（:* 表示任意参数后缀）
    - "Bash(cargo build:*)"
    - "Bash(cargo test:*)"
    - "Bash(cargo fmt:*)"
    - "Bash(git status:*)"
    - "Bash(ls:*)"

    # 工具级别：允许该工具所有调用跳过确认
    - "Read"
    - "Glob"
    - "Grep"

    # 文件写入限制到特定目录
    - "Write(path:/Users/jack/projects/*)"
    - "Edit(path:/Users/jack/projects/*)"

    # WebFetch 限制域名
    - "WebFetch(domain:docs.rs)"

  deny:
    # 黑名单（优先于 allow）
    - "Bash(rm -rf:*)"
    - "Bash(sudo:*)"
\`\`\`

## 匹配规则

| 规则格式 | 说明 | 示例 |
|----------|------|------|
| \`*\` | 匹配所有工具所有调用 | \`"*"\` |
| \`ToolName\` | 匹配该工具所有调用 | \`"Read"\`, \`"Grep"\` |
| \`Bash(cmd:*)\` | Bash 命令前缀匹配 | \`"Bash(cargo build:*)"\` |
| \`Write(path:dir/*)\` | 文件路径前缀匹配 | \`"Write(path:/home/user/*)"\` |
| \`WebFetch(domain:x)\` | URL 域名匹配 | \`"WebFetch(domain:docs.rs)"\` |

- 无 \`.jcli\` 文件：保持默认行为（需确认的工具弹确认框）
- \`deny\` 优先于 \`allow\`
- \`allow_all: true\` 或 allow 中包含 \`"*"\`：所有工具跳过确认`
      }
    }
  }
}

// Inline markdown renderer (handles `code`, **bold**, *italic*)
function renderInlineMarkdown(text: string): React.ReactNode {
  const parts: React.ReactNode[] = []
  let remaining = text
  let key = 0

  // First, handle escaped backticks: \`...\` -> treat as inline code
  remaining = remaining.replace(/\\`([^`]+)`/g, '\x00CODE_START$1CODE_END\x00')

  while (remaining.length > 0) {
    // Escaped backtick code (converted to special marker)
    const escapedCodeMatch = remaining.match(/\x00CODE_START(.+?)CODE_END\x00/)
    if (escapedCodeMatch && escapedCodeMatch.index !== undefined) {
      const before = remaining.slice(0, escapedCodeMatch.index)
      if (before) {
        parts.push(<span key={key++}>{renderInlineMarkdown(before)}</span>)
      }
      parts.push(
        <code key={key++} className="bg-stone-100 text-stone-800 px-1.5 py-0.5 rounded text-xs font-mono">
          {escapedCodeMatch[1]}
        </code>
      )
      remaining = remaining.slice(escapedCodeMatch.index + escapedCodeMatch[0].length)
      continue
    }

    // Inline code `...`
    const codeMatch = remaining.match(/`([^`]+)`/)
    if (codeMatch && codeMatch.index !== undefined) {
      const before = remaining.slice(0, codeMatch.index)
      if (before) {
        parts.push(<span key={key++}>{renderInlineMarkdown(before)}</span>)
      }
      parts.push(
        <code key={key++} className="bg-stone-100 text-stone-800 px-1.5 py-0.5 rounded text-xs font-mono">
          {codeMatch[1]}
        </code>
      )
      remaining = remaining.slice(codeMatch.index + codeMatch[0].length)
      continue
    }

    // Bold **...** (match any characters except line breaks, non-greedy)
    const boldMatch = remaining.match(/\*\*(.+?)\*\*/)
    if (boldMatch && boldMatch.index !== undefined) {
      const before = remaining.slice(0, boldMatch.index)
      if (before) {
        parts.push(<span key={key++}>{renderInlineMarkdown(before)}</span>)
      }
      parts.push(
        <strong key={key++} className="font-semibold text-stone-900">
          {boldMatch[1]}
        </strong>
      )
      remaining = remaining.slice(boldMatch.index + boldMatch[0].length)
      continue
    }

    // Italic *...* (but not **...**)
    const italicMatch = remaining.match(/(?<!\*)\*(?!\*)([^*]+)(?<!\*)\*(?!\*)/)
    if (italicMatch && italicMatch.index !== undefined) {
      const before = remaining.slice(0, italicMatch.index)
      if (before) {
        parts.push(<span key={key++}>{renderInlineMarkdown(before)}</span>)
      }
      parts.push(
        <em key={key++} className="italic">
          {italicMatch[1]}
        </em>
      )
      remaining = remaining.slice(italicMatch.index + italicMatch[0].length)
      continue
    }

    // No more matches, push remaining text
    parts.push(<span key={key++}>{remaining}</span>)
    break
  }

  return parts.length > 0 ? parts : text
}

// Markdown renderer (simple)
function Markdown({ content }: { content: string }) {
  const lines = content.split('\n')
  const elements: React.JSX.Element[] = []
  let inCodeBlock = false
  let codeContent = ''
  let codeLang = ''
  let inTable = false
  let tableRows: string[][] = []

  lines.forEach((line, index) => {
    // Code blocks
    if (line.startsWith('```')) {
      if (!inCodeBlock) {
        inCodeBlock = true
        codeLang = line.slice(3).trim() || 'text'
        codeContent = ''
      } else {
        inCodeBlock = false
        // Map common language names
        const langMap: Record<string, string> = {
          'bash': 'bash',
          'shell': 'bash',
          'sh': 'bash',
          'zsh': 'bash',
          'typescript': 'typescript',
          'ts': 'typescript',
          'javascript': 'javascript',
          'js': 'javascript',
          'python': 'python',
          'py': 'python',
          'rust': 'rust',
          'rs': 'rust',
          'go': 'go',
          'golang': 'go',
          'java': 'java',
          'c': 'c',
          'cpp': 'cpp',
          'c++': 'cpp',
          'csharp': 'csharp',
          'c#': 'csharp',
          'ruby': 'ruby',
          'rb': 'ruby',
          'sql': 'sql',
          'json': 'json',
          'yaml': 'yaml',
          'yml': 'yaml',
          'toml': 'toml',
          'markdown': 'markdown',
          'md': 'markdown',
          'html': 'html',
          'css': 'css',
          'scss': 'scss',
        }
        const lang = langMap[codeLang.toLowerCase()] || codeLang || 'text'
        
        elements.push(
          <div key={index} className="relative group my-4">
            <SyntaxHighlighter
              language={lang}
              style={oneLight}
              customStyle={{
                margin: 0,
                borderRadius: '0.5rem',
                fontSize: '0.875rem',
                backgroundColor: '#faf9f6',
                border: '1px solid #e7e5e4',
              }}
              codeTagProps={{
                style: {
                  fontFamily: 'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace',
                }
              }}
            >
              {codeContent}
            </SyntaxHighlighter>
          </div>
        )
      }
      return
    }

    if (inCodeBlock) {
      codeContent += (codeContent ? '\n' : '') + line
      return
    }

    // Tables
    if (line.startsWith('|')) {
      if (!inTable) {
        inTable = true
        tableRows = []
      }
      // Don't filter empty cells - just trim whitespace
      const cells = line.split('|').slice(1, -1).map(c => c.trim())
      if (!line.includes('---')) {
        tableRows.push(cells)
      }
      return
    } else if (inTable) {
      inTable = false
      // Find the maximum column count to ensure all rows have the same number of columns
      const maxCols = Math.max(...tableRows.map(row => row.length))
      elements.push(
        <div key={`table-${index}`} className="overflow-x-auto my-4">
          <table className="min-w-full border-collapse">
            <thead>
              <tr>
                {tableRows[0]?.map((cell, i) => (
                  <th key={i} className="border border-stone-200 px-4 py-2 text-left bg-stone-50 text-sm font-medium">
                    {renderInlineMarkdown(cell)}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {tableRows.slice(1).map((row, i) => (
                <tr key={i}>
                  {Array.from({ length: maxCols }).map((_, j) => (
                    <td key={j} className="border border-stone-200 px-4 py-2 text-sm">
                      {renderInlineMarkdown(row[j] || '')}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )
    }

    // Blockquotes
    if (line.startsWith('> ')) {
      elements.push(
        <blockquote key={index} className="border-l-4 border-stone-300 pl-4 py-1 my-3 text-stone-600 text-sm italic">
          {renderInlineMarkdown(line.slice(2))}
        </blockquote>
      )
      return
    }

    // Headings
    if (line.startsWith('## ')) {
      elements.push(<h2 key={index} className="text-2xl font-light text-stone-900 mt-8 mb-4">{renderInlineMarkdown(line.slice(3))}</h2>)
      return
    }
    if (line.startsWith('### ')) {
      elements.push(<h3 key={index} className="text-lg font-medium text-stone-900 mt-6 mb-3">{renderInlineMarkdown(line.slice(4))}</h3>)
      return
    }

    // Lists
    if (line.startsWith('- ') || line.startsWith('* ')) {
      elements.push(
        <li key={index} className="text-stone-600 text-sm ml-4 mb-1 list-disc">
          {renderInlineMarkdown(line.slice(2))}
        </li>
      )
      return
    }

    // Numbered lists
    const numMatch = line.match(/^(\d+)\.\s/)
    if (numMatch) {
      elements.push(
        <li key={index} className="text-stone-600 text-sm ml-4 mb-1 list-decimal">
          {renderInlineMarkdown(line.slice(numMatch[0].length))}
        </li>
      )
      return
    }

    // Paragraphs
    if (line.trim()) {
      elements.push(
        <p key={index} className="text-stone-600 text-sm leading-relaxed mb-3">
          {renderInlineMarkdown(line)}
        </p>
      )
    }
  })

  return <>{elements}</>
}

// Sidebar component
function Sidebar({ 
  tree, 
  activeSection, 
  onNavigate,
  isOpen,
  onClose
}: { 
  tree: typeof docTree.en
  activeSection: string
  onNavigate: (section: string) => void
  isOpen: boolean
  onClose: () => void
}) {
  return (
    <>
      {/* Mobile overlay */}
      {isOpen && (
        <div 
          className="fixed inset-0 bg-black/20 z-40 lg:hidden"
          onClick={onClose}
        />
      )}
      
      {/* Sidebar */}
      <aside className={`
        fixed top-[65px] left-0 bottom-0 w-72 bg-[#faf9f6] border-r border-stone-200 
        overflow-y-auto z-50 transition-transform duration-300
        lg:translate-x-0
        ${isOpen ? 'translate-x-0' : '-translate-x-full'}
      `}>
        <nav className="p-6">
          {Object.entries(tree).map(([key, category]) => (
            <div key={key} className="mb-6">
              <h3 className="text-xs font-semibold text-stone-400 uppercase tracking-wider mb-3">
                {category.title}
              </h3>
              <ul className="space-y-1">
                {Object.entries(category.children).map(([childKey, childTitle]) => (
                  <li key={childKey}>
                    <button
                      onClick={() => {
                        onNavigate(childKey)
                        onClose()
                      }}
                      className={`
                        w-full text-left px-3 py-2 rounded-lg text-sm transition-colors
                        ${activeSection === childKey 
                          ? 'bg-stone-200 text-stone-900 font-medium' 
                          : 'text-stone-600 hover:bg-stone-100'
                        }
                      `}
                    >
                      {childTitle}
                    </button>
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </nav>
      </aside>
    </>
  )
}

export default function Docs() {
  const [lang, setLang] = useState<Lang>('zh')
  const [langMenuOpen, setLangMenuOpen] = useState(false)
  const [sidebarOpen, setSidebarOpen] = useState(false)
  const [activeSection, setActiveSection] = useState('installation')
  const t = i18n[lang]
  const tree = docTree[lang]

  // Scroll to section on navigate
  useEffect(() => {
    const element = document.getElementById(activeSection)
    if (element) {
      element.scrollIntoView({ behavior: 'smooth', block: 'start' })
    }
  }, [activeSection])

  // Render section content
  const renderSection = () => {
    const section = t.sections[activeSection as keyof typeof t.sections]
    if (!section) return null

    return (
      <div id={activeSection} className="py-8">
        <h1 className="text-3xl font-light text-stone-900 mb-6">{section.title}</h1>
        <Markdown content={section.content} />
      </div>
    )
  }

  return (
    <div className="min-h-screen bg-[#faf9f6] text-stone-800">
      {/* Navigation */}
      <nav className="fixed top-0 left-0 right-0 z-50 bg-[#faf9f6]/95 backdrop-blur-sm border-b border-stone-200/50">
        <div className="px-4 sm:px-6 py-4 flex items-center justify-between">
          <div className="flex items-center gap-3">
            {/* Mobile menu button */}
            <button 
              onClick={() => setSidebarOpen(!sidebarOpen)}
              className="lg:hidden p-2 -ml-2 text-stone-500 hover:text-stone-900 transition-colors"
            >
              <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2}>
                {sidebarOpen ? (
                  <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
                ) : (
                  <path strokeLinecap="round" strokeLinejoin="round" d="M4 6h16M4 12h16M4 18h16" />
                )}
              </svg>
            </button>
            
            <Link to="/" className="flex items-center gap-2">
              <span className="text-2xl font-bold text-stone-900">j</span>
              <span className="text-stone-400 text-sm hidden sm:inline">docs</span>
            </Link>
          </div>
          
          <div className="flex items-center gap-3 sm:gap-5">
            {/* Language Switcher */}
            <div className="relative">
              <button 
                onClick={() => setLangMenuOpen(!langMenuOpen)}
                onBlur={() => setTimeout(() => setLangMenuOpen(false), 150)}
                className="text-stone-500 hover:text-stone-900 transition-colors text-sm flex items-center gap-0.5"
              >
                {lang === 'en' ? 'EN' : '中文'}
                <svg className={`w-3 h-3 ml-0.5 transition-transform ${langMenuOpen ? 'rotate-180' : ''}`} fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2}>
                  <path strokeLinecap="round" strokeLinejoin="round" d="M19 9l-7 7-7-7" />
                </svg>
              </button>
              {langMenuOpen && (
                <div className="absolute top-full right-0 mt-1 bg-white rounded shadow-lg py-1 z-50 min-w-[60px]">
                  <button
                    onClick={() => { setLang('en'); setLangMenuOpen(false); }}
                    className={`block w-full text-left px-3 py-1.5 text-sm hover:bg-stone-50 ${lang === 'en' ? 'text-stone-900 font-medium' : 'text-stone-500'}`}
                  >
                    EN
                  </button>
                  <button
                    onClick={() => { setLang('zh'); setLangMenuOpen(false); }}
                    className={`block w-full text-left px-3 py-1.5 text-sm hover:bg-stone-50 ${lang === 'zh' ? 'text-stone-900 font-medium' : 'text-stone-500'}`}
                  >
                    中文
                  </button>
                </div>
              )}
            </div>
            <a 
              href="https://github.com/LingoJack/j" 
              target="_blank" 
              rel="noopener noreferrer"
              className="flex items-center gap-2 text-stone-500 hover:text-stone-900 transition-colors"
            >
              <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 24 24">
                <path fillRule="evenodd" clipRule="evenodd" d="M12 2C6.477 2 2 6.477 2 12c0 4.42 2.87 8.17 6.84 9.5.5.08.66-.23.66-.5v-1.69c-2.77.6-3.36-1.34-3.36-1.34-.46-1.16-1.11-1.47-1.11-1.47-.91-.62.07-.6.07-.6 1 .07 1.53 1.03 1.53 1.03.87 1.52 2.34 1.07 2.91.83.09-.65.35-1.09.63-1.34-2.22-.25-4.55-1.11-4.55-4.92 0-1.11.38-2 1.03-2.71-.1-.25-.45-1.29.1-2.64 0 0 .84-.27 2.75 1.02.79-.22 1.65-.33 2.5-.33.85 0 1.71.11 2.5.33 1.91-1.29 2.75-1.02 2.75-1.02.55 1.35.2 2.39.1 2.64.65.71 1.03 1.6 1.03 2.71 0 3.82-2.34 4.66-4.57 4.91.36.31.69.92.69 1.85v2.74c0 .27.16.59.67.5C19.14 20.16 22 16.42 22 12A10 10 0 0012 2z"/>
              </svg>
              <span className="text-sm hidden sm:inline">{t.nav.github}</span>
            </a>
          </div>
        </div>
      </nav>

      {/* Sidebar */}
      <Sidebar 
        tree={tree}
        activeSection={activeSection}
        onNavigate={setActiveSection}
        isOpen={sidebarOpen}
        onClose={() => setSidebarOpen(false)}
      />

      {/* Main Content */}
      <main className="lg:ml-72 pt-[65px]">
        <div className="max-w-3xl mx-auto px-6 pb-16">
          {renderSection()}
        </div>
      </main>

      {/* Footer */}
      <footer className="lg:ml-72 border-t border-stone-200 py-8 px-6 bg-[#faf9f6]">
        <div className="max-w-3xl mx-auto flex items-center justify-between text-sm">
          <Link to="/" className="text-stone-500 hover:text-stone-900 transition-colors">
            {t.nav.back}
          </Link>
          <div className="flex items-center gap-6">
            <a 
              href="https://github.com/LingoJack/j" 
              target="_blank" 
              rel="noopener noreferrer"
              className="text-stone-400 hover:text-stone-900 transition-colors"
            >
              GitHub
            </a>
            <a 
              href="https://crates.io/crates/j-cli" 
              target="_blank" 
              rel="noopener noreferrer"
              className="text-stone-400 hover:text-stone-900 transition-colors"
            >
              crates.io
            </a>
          </div>
        </div>
      </footer>
    </div>
  )
}
