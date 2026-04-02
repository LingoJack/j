import{n as e,r as t}from"./rolldown-runtime-Dw2cE7zH.js";import{r as n,t as r}from"./react-vendor-CTSggWdF.js";import{t as i}from"./index-BjrEp65J.js";import{n as a,t as o}from"./syntax-highlight-DDfxEX0b.js";import{n as s,t as c}from"./LanguageSwitcher-BoZx07nq.js";var l=t(n(),1),u=r();function d({tree:e,activeSection:t,onNavigate:n,isOpen:r,onClose:i}){return(0,u.jsxs)(u.Fragment,{children:[r&&(0,u.jsx)(`div`,{className:`fixed inset-0 bg-black/20 z-40 lg:hidden`,onClick:i}),(0,u.jsx)(`aside`,{className:`
        fixed top-[65px] left-0 bottom-0 w-72 bg-[#faf9f6] border-r border-stone-200 
        overflow-y-auto z-50 transition-transform duration-300
        lg:translate-x-0
        ${r?`translate-x-0`:`-translate-x-full`}
      `,children:(0,u.jsx)(`nav`,{className:`p-6`,children:Object.entries(e).map(([e,r])=>(0,u.jsxs)(`div`,{className:`mb-6`,children:[(0,u.jsx)(`h3`,{className:`text-xs font-semibold text-stone-400 uppercase tracking-wider mb-3`,children:r.title}),(0,u.jsx)(`ul`,{className:`space-y-1`,children:Object.entries(r.children).map(([e,r])=>(0,u.jsx)(`li`,{children:(0,u.jsx)(`button`,{onClick:()=>{n(e),i()},className:`
                        w-full text-left px-3 py-2 rounded-lg text-sm transition-colors
                        ${t===e?`bg-stone-200 text-stone-900 font-medium`:`text-stone-600 hover:bg-stone-100`}
                      `,children:r})},e))})]},e))})})]})}var f={bash:`bash`,shell:`bash`,sh:`bash`,zsh:`bash`,typescript:`typescript`,ts:`typescript`,javascript:`javascript`,js:`javascript`,python:`python`,py:`python`,rust:`rust`,rs:`rust`,go:`go`,golang:`go`,java:`java`,c:`c`,cpp:`cpp`,"c++":`cpp`,csharp:`csharp`,"c#":`csharp`,ruby:`ruby`,rb:`ruby`,sql:`sql`,json:`json`,yaml:`yaml`,yml:`yaml`,toml:`toml`,markdown:`markdown`,md:`markdown`,html:`html`,css:`css`,scss:`scss`};function p(e,t){let n=[],r=e,i=0;for(;r.length>0;){let e=r.match(/`([^`]+)`/);if(e&&e.index!==void 0){let a=r.slice(0,e.index);a&&n.push((0,u.jsx)(`span`,{children:a},`${t}-txt-${i++}`)),n.push((0,u.jsx)(`code`,{className:`bg-stone-100 text-stone-700 px-1.5 py-0.5 rounded text-xs font-mono`,children:e[1]},`${t}-code-${i++}`)),r=r.slice(e.index+e[0].length);continue}let a=r.match(/\*\*([^*]+)\*\*/);if(a&&a.index!==void 0){let e=r.slice(0,a.index);e&&n.push((0,u.jsx)(`span`,{children:e},`${t}-txt-${i++}`)),n.push((0,u.jsx)(`strong`,{className:`font-medium text-stone-900`,children:a[1]},`${t}-bold-${i++}`)),r=r.slice(a.index+a[0].length);continue}let o=r.match(/\*([^*]+)\*/);if(o&&o.index!==void 0){let e=r.slice(0,o.index);e&&n.push((0,u.jsx)(`span`,{children:e},`${t}-txt-${i++}`)),n.push((0,u.jsx)(`em`,{className:`italic`,children:o[1]},`${t}-italic-${i++}`)),r=r.slice(o.index+o[0].length);continue}n.push((0,u.jsx)(`span`,{children:r},`${t}-txt-${i++}`));break}return n.length>0?n:e}function m({content:e}){return(0,u.jsx)(u.Fragment,{children:(0,l.useMemo)(()=>{let t=e.split(`
`),n=[],r=!1,i=``,c=``,l=!1,d=[],m=0,h=()=>{if(d.length>0){let e=Math.max(...d.map(e=>e.length)),t=`table-${m++}`;n.push((0,u.jsx)(`div`,{className:`overflow-x-auto my-4`,children:(0,u.jsxs)(`table`,{className:`min-w-full border-collapse`,children:[(0,u.jsx)(`thead`,{children:(0,u.jsx)(`tr`,{children:d[0]?.map((e,n)=>(0,u.jsx)(`th`,{className:`border border-stone-200 px-4 py-2 text-left bg-stone-50 text-sm font-medium`,children:p(e,`${t}-h${n}`)},`th-${n}`))})}),(0,u.jsx)(`tbody`,{children:d.slice(1).map((n,r)=>(0,u.jsx)(`tr`,{children:Array.from({length:e}).map((e,i)=>(0,u.jsx)(`td`,{className:`border border-stone-200 px-4 py-2 text-sm`,children:p(n[i]||``,`${t}-r${r}c${i}`)},`td-${i}`))},`tr-${r}`))})]})},t)),d=[]}};return t.forEach(e=>{let t=`line-${m++}`;if(e.startsWith("```")){if(!r)h(),r=!0,c=e.slice(3).trim()||`text`,i=``;else{r=!1;let e=f[c.toLowerCase()]||c||`text`;n.push((0,u.jsxs)(`div`,{className:`relative group my-4`,children:[(0,u.jsx)(a,{language:e,style:o,customStyle:{margin:0,borderRadius:`0.5rem`,fontSize:`0.875rem`,backgroundColor:`#faf9f6`,border:`1px solid #e7e5e4`},codeTagProps:{style:{fontFamily:`ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace`}},children:i}),(0,u.jsx)(s,{text:i})]},`code-${m++}`))}return}if(r){i+=(i?`
`:``)+e;return}if(e.startsWith(`|`)){l||(l=!0,d=[]);let t=e.split(`|`).slice(1,-1).map(e=>e.trim());e.includes(`---`)||d.push(t);return}else l&&(l=!1,h());if(e.startsWith(`> `)){n.push((0,u.jsx)(`blockquote`,{className:`border-l-4 border-stone-300 pl-4 py-1 my-3 text-stone-600 text-sm italic`,children:p(e.slice(2),`${t}-q`)},t));return}if(e.startsWith(`## `)){n.push((0,u.jsx)(`h2`,{className:`text-2xl font-light text-stone-900 mt-8 mb-4`,children:p(e.slice(3),`${t}-h2`)},t));return}if(e.startsWith(`### `)){n.push((0,u.jsx)(`h3`,{className:`text-lg font-medium text-stone-900 mt-6 mb-3`,children:p(e.slice(4),`${t}-h3`)},t));return}if(e.startsWith(`- `)||e.startsWith(`* `)){n.push((0,u.jsx)(`li`,{className:`text-stone-600 text-sm ml-4 mb-1 list-disc`,children:p(e.slice(2),`${t}-li`)},t));return}let g=e.match(/^(\d+)\.\s/);if(g){n.push((0,u.jsx)(`li`,{className:`text-stone-600 text-sm ml-4 mb-1 list-decimal`,children:p(e.slice(g[0].length),`${t}-nli`)},t));return}e.trim()&&n.push((0,u.jsx)(`p`,{className:`text-stone-600 text-sm leading-relaxed mb-3`,children:p(e,`${t}-p`)},t))}),l&&h(),n},[e])})}var h=e({default:()=>g}),g=`## Overview

Agent mode is an enhanced AI chat mode with autonomous multi-step reasoning and tool usage.

## Start

\`\`\`bash
j chat              # Enter TUI chat
\`\`\`

In the conversation, AI automatically uses tools to execute multi-step operations as needed.

## Features

- **Autonomous Reasoning**: AI plans and executes multi-step tasks
- **Tool Integration**: Automatically uses available tools (Read, Write, Bash, etc.)
- **Task Management**: Task and Todo tools manage complex tasks

## Example Tasks

\`\`\`
Analyze the codebase and suggest improvements

Find all TODO comments in the code and generate a summary

Research React state management best practices and generate a report
\`\`\`

## Tool Permission Configuration

Configure which tools the AI can use:

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
\`\`\`
`,ee=e({default:()=>te}),te=`## Start AI Chat

\`\`\`bash
j chat              # Enter TUI chat interface
j chat "Hello"      # Quick question and print response
j chat -c           # Continue last session
j chat --session <id>  # Resume specific session
\`\`\`

## Remote Control

\`\`\`bash
j chat --remote     # Enable remote control (scan QR with phone)
j chat --remote --port 9390  # Specify port
\`\`\`

## Shortcuts

| Shortcut | Action |
|----------|--------|
| \`Enter\` | Send message |
| \`Esc\` | Cancel response/Exit |
| \`Ctrl+T\` | Switch model |
| \`Ctrl+L\` | Archive conversation |
| \`Ctrl+Y\` | Copy last AI reply |
| \`Ctrl+B\` | Message browse mode |
| \`Ctrl+E\` | Open config panel |
| \`F1\` or \`?\` | Show help |

## Context References

Type \`@\` in input to trigger completion:

\`\`\`
@skill:<name>       # Reference skill
@command:<name>     # Reference custom command
@file:<path>        # Reference file content (supports images)
\`\`\`

## Multi-Model Support

Supports OpenAI, Claude, Gemini, Ollama and more. Use \`Ctrl+E\` to open config panel.
`,ne=e({default:()=>re}),re=`## Commands

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
\`\`\`
`,_=e({default:()=>v}),v=`## Overview

Browser is a tool in AI chat for web browsing, interaction, and content extraction.

## Modes

| Mode | Description |
|------|-------------|
| **Lite** | Lightweight HTTP control (default, no browser needed) |
| **CDP** | Full browser automation via Chrome DevTools Protocol (requires \`browser_cdp\` feature) |

## Using in AI Chat

\`\`\`
Open https://example.com and summarize the content

Take a screenshot of the current page

Click the submit button
\`\`\`

## Lite Mode

Default mode using HTTP requests to fetch web content:
- Get page text
- Extract page structure
- Get interactive element list

## CDP Mode

Full browser automation when \`browser_cdp\` feature is enabled:
- Screenshot capture
- Element click and input
- Page navigation
- Script injection
- Cookie management

## Build with CDP

\`\`\`bash
cargo build --features browser_cdp
\`\`\`
`,y=e({default:()=>b}),b="All data is stored in `~/.jdata/` (customizable via `J_DATA_PATH` environment variable):\n\n```\n~/.jdata/\n├── config.yaml          # Main config (aliases, categories, settings)\n├── agent/               # AI Agent data\n│   ├── data/            # Agent data directory\n│   │   ├── agent_config.json   # Agent config (model, API)\n│   │   ├── chat_history.json   # Chat history\n│   │   ├── archives/           # Archived conversations\n│   │   ├── system_prompt.md    # System prompt\n│   │   ├── memory.md           # Memory file\n│   │   ├── soul.md             # Soul file\n│   │   └── style.md            # Response style\n│   ├── logs/            # Agent logs\n│   │   ├── info.log\n│   │   └── error.log\n│   └── skills/          # Skills directory\n├── bin/                 # Built-in tools\n│   └── md_render        # Markdown renderer\n├── report/              # Daily reports\n│   ├── week_report.md   # Week report file\n│   ├── settings.json    # Report settings\n│   ├── todo.json        # Todo data\n│   └── .git/            # Git repository\n├── scripts/             # Scripts created via j concat\n```\n\n## Config File Structure (`config.yaml`)\n\n| Section | Description | Example |\n|---------|-------------|---------|\n| `path` | Local app/file paths | `chrome: /Applications/Google Chrome.app` |\n| `inner_url` | URL links | `github: https://github.com` |\n| `outer_url` | URLs requiring VPN | `docs: https://internal.example.com` |\n| `browser` | Browser list | `chrome: chrome` |\n| `editor` | Editor list | `vscode: vscode` |\n| `vpn` | VPN application | |\n| `script` | Registered scripts | `deploy: ~/.jdata/scripts/deploy.sh` |\n| `report` | Report system config | `git_repo: https://github.com/xxx/report` |\n| `setting` | Global settings | `search-engine: bing` |\n| `log` | Log settings | `mode: concise` |\n",x=e({default:()=>S}),S=`## Overview

Hooks allow running custom scripts on specific events, managed via the \`RegisterHook\` tool or config files.

## Hook Events

| Event | When Triggered | Readable Fields | Writable Fields |
|-------|----------------|-----------------|-----------------|
| \`pre_send_message\` | Before sending user message | user_input, messages | user_input, abort |
| \`post_send_message\` | After sending user message | user_input, messages | Notification only |
| \`pre_llm_request\` | Before LLM request | messages, system_prompt, model | messages, system_prompt, inject_messages, abort |
| \`post_llm_response\` | After LLM response | assistant_output, messages | assistant_output |
| \`pre_tool_execution\` | Before tool execution | tool_name, tool_arguments | tool_arguments, abort |
| \`post_tool_execution\` | After tool execution | tool_name, tool_result | tool_result |
| \`session_start\` | Session starts | messages | Notification only |
| \`session_end\` | Session ends | messages | Notification only |

## Using RegisterHook Tool

Manage session-level hooks via tool in AI chat:

\`\`\`
# View protocol documentation
RegisterHook action="help"

# List registered hooks
RegisterHook action="list"

# Register a hook
RegisterHook event="pre_send_message" command="echo '{\\"user_input\\": \\"[modified]\\"}'" timeout=10

# Remove a hook
RegisterHook action="remove" event="pre_send_message" index=0
\`\`\`

## Configuration Files

Manage persistent hooks via YAML files:

\`\`\`yaml
# User level: ~/.jdata/agent/hooks.yaml
# Project level: .jcli/hooks.yaml

pre_send_message:
  - command: "echo '{\\"user_input\\": \\"[timestamp] \\" + $(cat | jq -r .user_input)}'"
    timeout: 5

pre_tool_execution:
  - command: |
      input=$(cat)
      tool=$(echo "$input" | jq -r .tool_name)
      if [ "$tool" = "Bash" ]; then
        echo '{"abort": true}'
      else
        echo '{}'
      fi
    timeout: 10
\`\`\`

## Script Protocol

### Execution Environment

| Item | Description |
|------|-------------|
| Execution | \`sh -c "<command>"\` |
| Working Directory | User's current directory |
| Environment Variables | \`JCLI_HOOK_EVENT\` (event name), \`JCLI_CWD\` (current directory) |

### stdin/stdout

| Item | Description |
|------|-------------|
| stdin | HookContext JSON |
| stdout | HookResult JSON (empty or \`{}\` means no modification) |
| exit 0 | Success |
| exit non-zero | Treated as abort |

### stdin HookContext Example

\`\`\`json
{
  "event": "pre_send_message",
  "cwd": "/path/to/project",
  "user_input": "User input text",
  "messages": [{"role": "user", "content": "..."}],
  "system_prompt": "System prompt",
  "model": "gpt-4o",
  "assistant_output": "AI response text",
  "tool_name": "Bash",
  "tool_arguments": "{\\"command\\": \\"ls\\"}",
  "tool_result": "Tool execution result"
}
\`\`\`

### stdout HookResult Example

\`\`\`json
{
  "user_input": "Modified user message",
  "assistant_output": "Modified AI response",
  "messages": [{"role": "user", "content": "..."}],
  "system_prompt": "Modified prompt",
  "tool_arguments": "Modified tool arguments",
  "tool_result": "Modified tool result",
  "inject_messages": [{"role": "user", "content": "Injected message"}],
  "abort": false
}
\`\`\`

## Script Examples

### Add Timestamp to User Message

\`\`\`bash
#!/bin/bash
input=$(cat)
msg=$(echo "$input" | jq -r .user_input)
echo "{\\"user_input\\": \\"[$(date '+%H:%M')] $msg\\"}"
\`\`\`

### Block Dangerous Commands

\`\`\`bash
#!/bin/bash
input=$(cat)
tool=$(echo "$input" | jq -r .tool_name)
args=$(echo "$input" | jq -r .tool_arguments)

if [ "$tool" = "Bash" ] && echo "$args" | grep -q "rm -rf"; then
  echo '{"abort": true}'
else
  echo '{}'
fi
\`\`\`

### Notification-only Hook

\`\`\`bash
#!/bin/bash
cat > /dev/null  # Must read stdin to avoid SIGPIPE
\`\`\`

## Three-Level Hook Priority

Hooks exist at three levels, executed in order: User → Project → Session

| Level | Config Location | Lifecycle |
|-------|-----------------|-----------|
| User | \`~/.jdata/agent/hooks.yaml\` | Persistent |
| Project | \`.jcli/hooks.yaml\` | Persistent within project |
| Session | RegisterHook tool | Current session only |

During chain execution, the previous hook's output updates the context for the next hook. Any \`abort\` immediately stops the entire chain.

## Notes

- Create script files with Write/Bash tools first, then register with RegisterHook
- Scripts must read from stdin (at least \`cat > /dev/null\`) to avoid SIGPIPE
- Default timeout is 10 seconds; scripts are killed on timeout
- Only session-level hooks can be managed via tool; user/project levels require manual config editing
`,C=e({default:()=>w}),w=`## One-click Install (Recommended)

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
\`\`\`
`,T=e({default:()=>E}),E=`## Permission Levels

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
\`\`\`
`,D=e({default:()=>O}),O=`## Register App Aliases

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
\`\`\`
`,k=e({default:()=>A}),A=`## Overview

Control AI chat from mobile devices via WebSocket, started with \`j chat --remote\`.

## Start

\`\`\`bash
j chat --remote              # Start remote control (default port 9390)
j chat --remote --port 9390  # Specify port
\`\`\`

A QR code will be displayed for phone scanning.

## Features

- **Mobile Control**: Send messages from your phone
- **Real-time Sync**: Continue conversations across devices
- **QR Connection**: No need to type addresses manually

## Client

- **Web**: Scan QR code to connect
`,j=e({default:()=>M}),M=`## Overview

Daily/weekly report system with quick logging, week management, and Git sync.

## Basic Commands

\`\`\`bash
j report <content>        # Quick write to daily report
j report                  # Open TUI editor (prefilled with history + date prefix)
j check [n]               # View last n lines (default 3)
j check open              # Open TUI editor to edit full content
j search <n|all> <keyword> [-f]  # Search reports (-f for fuzzy match)
\`\`\`

## Week Management (reportctl)

\`\`\`bash
j reportctl new [date]      # Start a new week
j reportctl sync [date]     # Sync week number and date
j reportctl set-url <url>   # Set Git repository URL
j reportctl push [message]  # Push to remote repository
j reportctl pull            # Pull from remote repository
j reportctl open            # Open TUI editor to edit full content
\`\`\`

## Report Format

\`\`\`markdown
# Week1[2024-01-01 - 2024-01-07]
- 【01-01】 Project initialization completed
- 【01-02】 Core features implemented
- 【01-03】 Code review and optimization
\`\`\`

## Git Sync

Set up remote repository for automatic sync:

\`\`\`bash
# Initial setup
j reportctl set-url https://github.com/user/reports.git

# Push to remote
j reportctl push "Update report"

# Pull from remote
j reportctl pull
\`\`\`

## Configuration Files

Report settings are stored in two locations:

| File | Description |
|------|-------------|
| \`~/.jdata/config.yaml\` | Main config (report_file_path, git_repo) |
| \`<report_dir>/settings.json\` | Week metadata (week_num, last_day) |

## Auto Week Switch

When current date exceeds \`last_day\`, writing to report automatically:
1. Generates new week title \`# WeekN[start_date - end_date]\`
2. Updates week_num and last_day
`,N=e({default:()=>P}),P=`## Commands

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
\`\`\`
`,F=e({default:()=>I}),I=`## Overview

Skills are specialized prompt modules that extend AI capabilities, loaded via the \`LoadSkill\` tool.

## Skill Structure

\`\`\`
~/.jdata/agent/skills/<skill_name>/
├── SKILL.md          # Skill definition (required)
├── references/       # Reference documents
└── scripts/          # Script files
\`\`\`

## Creating a Skill

\`\`\`markdown
# SKILL.md
---
name: code-review
description: Code review best practices
argument-hint: file path  # optional, hints the argument user passes
---

You are a code reviewer. Analyze code for:
- Code quality
- Performance issues
- Security vulnerabilities
- Best practices
\`\`\`

## Using Skills

AI loads skills via the \`LoadSkill\` tool:

\`\`\`
Load the code-review skill
\`\`\`

## Skill Sources

- **User level**: \`~/.jdata/agent/skills/\`
- **Project level**: \`.jcli/skills/\` (project level overrides user level when names conflict)
`,L=e({default:()=>R}),R=`## Commands

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
\`\`\`
`,z=e({default:()=>B}),B="## Available Tools\n\n| Tool | Description |\n|------|-------------|\n| `Read` | Read file contents |\n| `Write` | Write to files |\n| `Edit` | Edit files with string replacement |\n| `Glob` | Find files by pattern |\n| `Grep` | Search file contents |\n| `Bash` | Execute shell commands |\n| `WebFetch` | Fetch web page content |\n| `WebSearch` | Search the web |\n| `Ask` | Ask user for input |\n| `TaskOutput` | Get background task output |\n| `Task` | Manage tasks |\n| `TodoWrite` | Write todo items |\n| `TodoRead` | Read todo items |\n| `Compact` | Compress conversation context |\n| `RegisterHook` | Register hooks |\n| `ComputerUse` | macOS desktop control |\n| `EnterPlanMode` | Enter plan mode |\n| `ExitPlanMode` | Exit plan mode |\n| `LoadSkill` | Load skills |\n\n## Permission Configuration\n\n```yaml\n# ~/.jdata/agent/data/agent_config.yaml\ntools:\n  - name: Read\n    permission: allow\n  - name: Bash\n    permission: ask  # Require user confirmation\n  - name: Write\n    permission: deny\n```\n\n## Context References\n\n| Reference | Description |\n|-----------|-------------|\n| `@file:path` | Include file content |\n| `@dir:path` | Include directory structure |\n| `@url:url` | Include web page content |\n| `@grep:pattern` | Include search results |\n",V=e({default:()=>H}),H=`## 概述

Agent 模式是 AI 对话的增强模式，支持自主多步推理和工具调用。

## 启动

\`\`\`bash
j chat              # 进入 TUI 对话
\`\`\`

在对话中，AI 会根据任务需要自动使用工具执行多步操作。

## 功能特性

- **自主推理**：AI 规划并执行多步任务
- **工具集成**：自动使用可用工具（Read、Write、Bash 等）
- **任务管理**：Task 和 Todo 工具管理复杂任务

## 示例任务

\`\`\`
分析代码库并提出改进建议

查找代码中的所有 TODO 注释并生成摘要

研究 React 状态管理的最佳实践并生成报告
\`\`\`

## 工具权限配置

配置 AI 可以使用的工具：

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
\`\`\`
`,U=e({default:()=>W}),W=`## 启动 AI 对话

\`\`\`bash
j chat              # 进入 TUI 对话界面
j chat "你好"       # 快速提问并打印回复
j chat -c           # 延续上一个会话
j chat --session <id>  # 恢复指定会话
\`\`\`

## 远程控制

\`\`\`bash
j chat --remote     # 启用远程控制（手机扫码）
j chat --remote --port 9390  # 指定端口
\`\`\`

## 快捷键

| 快捷键 | 功能 |
|--------|------|
| \`Enter\` | 发送消息 |
| \`Esc\` | 取消响应/退出 |
| \`Ctrl+T\` | 切换模型 |
| \`Ctrl+L\` | 归档对话 |
| \`Ctrl+Y\` | 复制最后一条 AI 回复 |
| \`Ctrl+B\` | 消息浏览模式 |
| \`Ctrl+E\` | 打开配置界面 |
| \`F1\` 或 \`?\` | 显示帮助 |

## 上下文引用

输入框中以 \`@\` 触发补全：

\`\`\`
@skill:<name>       # 引用技能
@command:<name>     # 引用自定义命令
@file:<path>        # 引用文件内容（支持图片）
\`\`\`

## 多模型支持

支持 OpenAI、Claude、Gemini、Ollama 等模型，通过 \`Ctrl+E\` 打开配置界面管理。
`,G=e({default:()=>K}),K=`## 命令

| 命令 | 描述 |
|------|------|
| \`j set <别名> <路径>\` | 设置别名（路径 → path 配置，URL → inner_url） |
| \`j rm <别名>\` | 删除别名（同时清理关联的分类标记） |
| \`j rename <别名> <新别名>\` | 重命名别名（更新所有分类引用） |
| \`j mf <别名> <新路径>\` | 修改别名路径 |

## 分类标记

\`\`\`bash
j note <别名> <分类>   # 为别名标记分类
j find <分类>         # 按分类查找别名
j note chrome browser # 将 chrome 标记为浏览器
j note github outer_url # 将 github 标记为外网（自动连接 VPN）
\`\`\`

## 分类说明

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
j chrome "rust lang"       # 用 Chrome 搜索
j chrome github            # 用 Chrome 打开 github
j vscode proj              # 用 VSCode 打开 proj 目录
\`\`\`
`,q=e({default:()=>J}),J=`## 概述

Browser 是 AI 对话中的工具，支持网页浏览、交互和内容提取。

## 模式

| 模式 | 描述 |
|------|------|
| **Lite** | 轻量级 HTTP 控制（默认，无需浏览器） |
| **CDP** | 通过 Chrome DevTools Protocol 实现完整浏览器自动化（需 \`browser_cdp\` feature） |

## 在 AI 对话中使用

\`\`\`
打开 https://example.com 并总结内容

截取当前页面的截图

点击提交按钮
\`\`\`

## Lite 模式

默认模式，使用 HTTP 请求获取网页内容：
- 获取页面文本
- 提取页面结构
- 获取交互元素列表

## CDP 模式

启用 \`browser_cdp\` feature 后支持完整浏览器自动化：
- 截图捕获
- 元素点击和输入
- 页面导航
- 脚本注入
- Cookie 管理

## 编译启用 CDP

\`\`\`bash
cargo build --features browser_cdp
\`\`\`
`,Y=e({default:()=>X}),X="所有数据存储在 `~/.jdata/` 目录（可通过 `J_DATA_PATH` 环境变量自定义）：\n\n```\n~/.jdata/\n├── config.yaml          # 主配置（别名、分类、设置）\n├── agent/               # AI Agent 数据\n│   ├── data/            # Agent 数据目录\n│   │   ├── agent_config.json   # Agent 配置（模型、API）\n│   │   ├── chat_history.json   # 对话历史\n│   │   ├── archives/           # 归档对话\n│   │   ├── system_prompt.md    # 系统提示词\n│   │   ├── memory.md           # 记忆文件\n│   │   ├── soul.md             # 灵魂文件\n│   │   └── style.md            # 响应风格\n│   ├── logs/            # Agent 日志\n│   │   ├── info.log\n│   │   └── error.log\n│   └── skills/          # 技能目录\n├── bin/                 # 内置工具\n│   └── md_render        # Markdown 渲染器\n├── report/              # 日报数据\n│   ├── week_report.md   # 周报文件\n│   ├── settings.json    # 报告设置\n│   ├── todo.json        # 待办数据\n│   └── .git/            # Git 仓库\n├── scripts/             # 通过 j concat 创建的脚本\n```\n\n## 配置文件结构（`config.yaml`）\n\n| 配置项 | 描述 | 示例 |\n|--------|------|------|\n| `path` | 本地应用/文件路径 | `chrome: /Applications/Google Chrome.app` |\n| `inner_url` | URL 链接 | `github: https://github.com` |\n| `outer_url` | 需要 VPN 的 URL | `docs: https://internal.example.com` |\n| `browser` | 浏览器列表 | `chrome: chrome` |\n| `editor` | 编辑器列表 | `vscode: vscode` |\n| `vpn` | VPN 应用 | |\n| `script` | 注册脚本 | `deploy: ~/.jdata/scripts/deploy.sh` |\n| `report` | 日报系统配置 | `git_repo: https://github.com/xxx/report` |\n| `setting` | 全局设置 | `search-engine: bing` |\n| `log` | 日志设置 | `mode: concise` |\n",ie=e({default:()=>ae}),ae=`## 概述

Hook 允许在特定事件时运行自定义脚本，通过 \`RegisterHook\` 工具或配置文件管理。

## Hook 事件

| 事件 | 触发时机 | 可读字段 | 可写字段 |
|------|----------|----------|----------|
| \`pre_send_message\` | 用户消息发送前 | user_input, messages | user_input, abort |
| \`post_send_message\` | 用户消息发送后 | user_input, messages | 仅通知，返回值忽略 |
| \`pre_llm_request\` | LLM 请求前 | messages, system_prompt, model | messages, system_prompt, inject_messages, abort |
| \`post_llm_response\` | LLM 回复后 | assistant_output, messages | assistant_output |
| \`pre_tool_execution\` | 工具执行前 | tool_name, tool_arguments | tool_arguments, abort |
| \`post_tool_execution\` | 工具执行后 | tool_name, tool_result | tool_result |
| \`session_start\` | 会话开始 | messages | 仅通知 |
| \`session_end\` | 会话结束 | messages | 仅通知 |

## 使用 RegisterHook 工具

在 AI 对话中通过工具管理 session 级 hook：

\`\`\`
# 查看协议文档
RegisterHook action="help"

# 列出已注册的 hook
RegisterHook action="list"

# 注册 hook
RegisterHook event="pre_send_message" command="echo '{\\"user_input\\": \\"[modified]\\"}'" timeout=10

# 移除 hook
RegisterHook action="remove" event="pre_send_message" index=0
\`\`\`

## 配置文件

通过 YAML 文件管理持久化 hook：

\`\`\`yaml
# 用户级: ~/.jdata/agent/hooks.yaml
# 项目级: .jcli/hooks.yaml

pre_send_message:
  - command: "echo '{\\"user_input\\": \\"[timestamp] \\" + $(cat | jq -r .user_input)}'"
    timeout: 5

pre_tool_execution:
  - command: |
      input=$(cat)
      tool=$(echo "$input" | jq -r .tool_name)
      if [ "$tool" = "Bash" ]; then
        echo '{"abort": true}'
      else
        echo '{}'
      fi
    timeout: 10
\`\`\`

## 脚本协议

### 执行环境

| 项目 | 说明 |
|------|------|
| 执行方式 | \`sh -c "<command>"\` |
| 工作目录 | 用户当前目录 |
| 环境变量 | \`JCLI_HOOK_EVENT\`（事件名）、\`JCLI_CWD\`（当前目录） |

### stdin/stdout

| 项目 | 说明 |
|------|------|
| stdin | HookContext JSON |
| stdout | HookResult JSON（空或 \`{}\` 表示无修改） |
| exit 0 | 成功 |
| exit 非 0 | 视为 abort |

### stdin HookContext 示例

\`\`\`json
{
  "event": "pre_send_message",
  "cwd": "/path/to/project",
  "user_input": "用户输入文本",
  "messages": [{"role": "user", "content": "..."}],
  "system_prompt": "系统提示词",
  "model": "gpt-4o",
  "assistant_output": "AI 回复文本",
  "tool_name": "Bash",
  "tool_arguments": "{\\"command\\": \\"ls\\"}",
  "tool_result": "工具执行结果"
}
\`\`\`

### stdout HookResult 示例

\`\`\`json
{
  "user_input": "修改后的用户消息",
  "assistant_output": "修改后的 AI 回复",
  "messages": [{"role": "user", "content": "..."}],
  "system_prompt": "修改后的提示词",
  "tool_arguments": "修改后的工具参数",
  "tool_result": "修改后的工具结果",
  "inject_messages": [{"role": "user", "content": "注入消息"}],
  "abort": false
}
\`\`\`

## 脚本示例

### 给用户消息加时间戳

\`\`\`bash
#!/bin/bash
input=$(cat)
msg=$(echo "$input" | jq -r .user_input)
echo "{\\"user_input\\": \\"[$(date '+%H:%M')] $msg\\"}"
\`\`\`

### 拦截危险命令

\`\`\`bash
#!/bin/bash
input=$(cat)
tool=$(echo "$input" | jq -r .tool_name)
args=$(echo "$input" | jq -r .tool_arguments)

if [ "$tool" = "Bash" ] && echo "$args" | grep -q "rm -rf"; then
  echo '{"abort": true}'
else
  echo '{}'
fi
\`\`\`

### 纯通知 hook

\`\`\`bash
#!/bin/bash
cat > /dev/null  # 必须读取 stdin，否则可能 SIGPIPE
\`\`\`

## 三级 Hook 优先级

Hook 分三个级别，执行顺序：用户级 → 项目级 → Session 级

| 级别 | 配置位置 | 生命周期 |
|------|----------|----------|
| 用户级 | \`~/.jdata/agent/hooks.yaml\` | 持久化 |
| 项目级 | \`.jcli/hooks.yaml\` | 项目内持久化 |
| Session 级 | RegisterHook 工具 | 仅当前会话 |

链式执行时，前一个 hook 的输出会更新到 context 中，成为下一个 hook 的输入。任何 \`abort\` 立即中止整条链。

## 注意事项

- 先用 Write/Bash 工具创建脚本文件，再用 RegisterHook 注册
- 脚本必须从 stdin 读取（至少 \`cat > /dev/null\`），否则可能 SIGPIPE
- timeout 默认 10 秒，超时后脚本被 kill
- 只有 session 级 hook 可通过工具管理；用户级/项目级需手动编辑配置文件
`,oe=e({default:()=>se}),se=`## 一键安装（推荐）

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

# 完整版（CDP 浏览器模式，需要 Chrome/Chromium）
cargo install j-cli --features browser_cdp
\`\`\`

## 从源码构建

\`\`\`bash
git clone https://github.com/LingoJack/j.git
cd j && cargo install --path .

# 包含完整浏览器自动化功能
cargo install --path . --features browser_cdp
\`\`\`

## 验证安装

\`\`\`bash
j --version
j --help
\`\`\`

## 更新

\`\`\`bash
# 使用内置更新命令（自动检测安装来源）
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
\`\`\`
`,ce=e({default:()=>le}),le=`## 权限级别

| 级别 | 描述 |
|------|------|
| \`allow\` | 始终允许 |
| \`ask\` | 请求确认 |
| \`deny\` | 始终拒绝 |

## 配置

\`\`\`yaml
# ~/.jdata/agent/data/agent_config.yaml
permissions:
  # 读取操作 - 始终允许
  - tool: Read
    permission: allow
  
  # 写入操作 - 请求确认
  - tool: Write
    permission: ask
  
  # Shell 命令 - 请求确认
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
\`\`\`
`,ue=e({default:()=>de}),de=`## 注册应用别名

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
j chrome github           # 用 Chrome 打开 github URL
j chrome "rust lang"      # 用 Chrome 搜索 "rust lang"
j vscode ./src            # 用 VSCode 打开 src 目录
\`\`\`

## 日报

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
j chat 你好               # 快速提问
\`\`\`

## 交互模式

\`\`\`bash
j                         # 进入交互模式，支持 Tab 补全
\`\`\`
`,fe=e({default:()=>pe}),pe=`## 概述

通过 WebSocket 从移动设备控制 AI 对话，使用 \`j chat --remote\` 启动。

## 启动

\`\`\`bash
j chat --remote              # 启动远程控制（默认端口 9390）
j chat --remote --port 9390  # 指定端口
\`\`\`

启动后会显示二维码，手机扫码即可连接。

## 功能特性

- **移动控制**：使用手机发送消息
- **实时同步**：跨设备继续对话
- **扫码连接**：无需手动输入地址

## 客户端

- **Web**：扫描二维码连接
`,me=e({default:()=>he}),he=`## 概述

日报/周报系统，支持快速记录、周报管理和 Git 同步。

## 基本命令

\`\`\`bash
j report <内容>         # 快速写入日报
j report                # 打开 TUI 编辑器（预填历史+日期前缀）
j check [n]             # 查看最近 n 行（默认 3 行）
j check open            # 打开 TUI 编辑器编辑全文
j search <n|all> <关键词> [-f]  # 搜索周报（-f 模糊匹配）
\`\`\`

## 周报管理 (reportctl)

\`\`\`bash
j reportctl new [日期]      # 开启新的一周
j reportctl sync [日期]     # 同步周数和日期
j reportctl set-url <url>   # 设置 Git 仓库地址
j reportctl push [message]  # 推送到远程仓库
j reportctl pull            # 从远程仓库拉取
j reportctl open            # 打开 TUI 编辑器编辑全文
\`\`\`

## 日报格式

\`\`\`markdown
# Week1[2024-01-01 - 2024-01-07]
- 【01-01】 完成项目初始化
- 【01-02】 实现核心功能
- 【01-03】 代码审查和优化
\`\`\`

## Git 同步

设置远程仓库后可自动同步：

\`\`\`bash
# 首次设置
j reportctl set-url https://github.com/user/reports.git

# 推送到远程
j reportctl push "更新周报"

# 从远程拉取
j reportctl pull
\`\`\`

## 配置文件

日报配置存储在两个位置：

| 文件 | 描述 |
|------|------|
| \`~/.jdata/config.yaml\` | 主配置（report_file_path、git_repo） |
| \`<report_dir>/settings.json\` | 周报元数据（week_num、last_day） |

## 自动周切换

当当前日期超过 \`last_day\` 时，写入日报会自动：
1. 生成新周标题 \`# WeekN[开始日期 - 结束日期]\`
2. 更新 week_num 和 last_day
`,ge=e({default:()=>_e}),_e=`## 命令

| 命令 | 描述 |
|------|------|
| \`j concat <名称> [内容]\` | 创建/编辑脚本 |
| \`j <脚本名> [参数]\` | 执行脚本并传递参数 |

## 创建脚本

\`\`\`bash
# 创建脚本并指定内容
j concat open "open $1"

# 使用 TUI 编辑器创建
j concat deploy

# 在新窗口中创建
j concat build -w
\`\`\`

## 执行脚本

\`\`\`bash
# 执行脚本
j open README.md         # README.md 作为 $1 传入
j build                  # 无参数执行

# 在新窗口中执行
j open -w README.md
\`\`\`

## 环境变量

脚本可以使用以下环境变量：

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

# 编辑器脚本
j concat edit "code $1"
\`\`\`
`,ve=e({default:()=>ye}),ye=`## 概述

Skill 是扩展 AI 能力的专用提示词模块，通过 \`LoadSkill\` 工具加载。

## Skill 结构

\`\`\`
~/.jdata/agent/skills/<skill_name>/
├── SKILL.md          # Skill 定义（必需）
├── references/       # 参考文档
└── scripts/          # 脚本文件
\`\`\`

## 创建 Skill

\`\`\`markdown
# SKILL.md
---
name: code-review
description: 代码审查最佳实践
argument-hint: 文件路径  # 可选，提示用户传入的参数
---

你是一个代码审查者。分析代码的：
- 代码质量
- 性能问题
- 安全漏洞
- 最佳实践
\`\`\`

## 使用 Skill

AI 通过 \`LoadSkill\` 工具加载 skill：

\`\`\`
加载 code-review skill
\`\`\`

## Skill 来源

- **用户级**：\`~/.jdata/agent/skills/\`
- **项目级**：\`.jcli/skills/\`（同名时项目级覆盖用户级）
`,be=e({default:()=>xe}),xe=`## 命令

| 命令 | 描述 |
|------|------|
| \`j todo\` | 打开 TUI 待办管理器 |
| \`j todo add <内容>\` | 快速添加待办 |
| \`j todo done <id>\` | 标记待办完成 |
| \`j todo list\` | 列出待办（支持 --done/--undone） |

## 示例

\`\`\`bash
# 快速添加
j todo add 买牛奶
j todo add 审查 PR

# 列出待办
j todo list              # 所有待办
j todo list --undone     # 仅未完成
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

待办可以在日报中使用 Markdown 格式：

\`\`\`markdown
- [x] 已完成的任务
- [ ] 待处理的任务
- [ ] 另一个待处理任务
\`\`\`
`,Se=e({default:()=>Ce}),Ce="## 可用工具\n\n| 工具 | 描述 |\n|------|------|\n| `Read` | 读取文件内容 |\n| `Write` | 写入文件 |\n| `Edit` | 字符串替换编辑文件 |\n| `Glob` | 按模式查找文件 |\n| `Grep` | 搜索文件内容 |\n| `Bash` | 执行 shell 命令 |\n| `WebFetch` | 获取网页内容 |\n| `WebSearch` | 搜索网络 |\n| `Ask` | 向用户请求输入 |\n| `TaskOutput` | 获取后台任务输出 |\n| `Task` | 管理任务 |\n| `TodoWrite` | 写入待办事项 |\n| `TodoRead` | 读取待办事项 |\n| `Compact` | 压缩对话上下文 |\n| `RegisterHook` | 注册钩子 |\n| `ComputerUse` | macOS 桌面控制 |\n| `EnterPlanMode` | 进入计划模式 |\n| `ExitPlanMode` | 退出计划模式 |\n| `LoadSkill` | 加载技能 |\n\n## 权限配置\n\n```yaml\n# ~/.jdata/agent/data/agent_config.yaml\ntools:\n  - name: Read\n    permission: allow\n  - name: Bash\n    permission: ask  # 需要用户确认\n  - name: Write\n    permission: deny\n```\n\n## 上下文引用\n\n| 引用 | 描述 |\n|------|------|\n| `@file:路径` | 包含文件内容 |\n| `@dir:路径` | 包含目录结构 |\n| `@url:url` | 包含网页内容 |\n| `@grep:模式` | 包含搜索结果 |\n",we={en:{gettingStarted:{title:`Getting Started`,children:{installation:`Installation`,quickStart:`Quick Start`,dataDirectory:`Data Directory`}},coreFeatures:{title:`Core Features`,children:{alias:`Alias Management`,report:`Daily Reports`,todo:`Todo Management`,script:`Script System`}},aiFeatures:{title:`AI Features`,children:{aiChat:`AI Chat`,agentMode:`Agent Mode`,tools:`AI Tools`,skills:`Skill System`,hooks:`Hook System`}},advanced:{title:`Advanced`,children:{browser:`Browser Automation`,remote:`Remote Control`,permissions:`Permissions`}}},zh:{gettingStarted:{title:`快速开始`,children:{installation:`安装`,quickStart:`快速上手`,dataDirectory:`数据目录`}},coreFeatures:{title:`核心功能`,children:{alias:`别名管理`,report:`日报系统`,todo:`待办管理`,script:`脚本系统`}},aiFeatures:{title:`AI 功能`,children:{aiChat:`AI 对话`,agentMode:`Agent 模式`,tools:`AI 工具`,skills:`Skill 技能`,hooks:`Hook 系统`}},advanced:{title:`进阶功能`,children:{browser:`浏览器自动化`,remote:`远程控制`,permissions:`权限配置`}}}},Te={en:{back:`← Back to Home`,github:`GitHub`,menu:`Menu`},zh:{back:`← 返回首页`,github:`GitHub`,menu:`菜单`}},Z={en:{installation:`Installation`,quickStart:`Quick Start`,dataDirectory:`Data Directory`,alias:`Alias Management`,report:`Daily Reports`,todo:`Todo Management`,script:`Script System`,aiChat:`AI Chat`,agentMode:`Agent Mode`,tools:`AI Tools`,skills:`Skill System`,hooks:`Hook System`,browser:`Browser Automation`,remote:`Remote Control`,permissions:`Permissions`},zh:{installation:`安装`,quickStart:`快速上手`,dataDirectory:`数据目录`,alias:`别名管理`,report:`日报系统`,todo:`待办管理`,script:`脚本系统`,aiChat:`AI 对话`,agentMode:`Agent 模式`,tools:`AI 工具`,skills:`Skill 技能`,hooks:`Hook 系统`,browser:`浏览器自动化`,remote:`远程控制`,permissions:`权限配置`}};function Ee(){return[`installation`,`quickStart`,`dataDirectory`,`alias`,`report`,`todo`,`script`,`aiChat`,`agentMode`,`tools`,`skills`,`hooks`,`browser`,`remote`,`permissions`]}var De=Object.assign({"./en/agentMode.md":h,"./en/aiChat.md":ee,"./en/alias.md":ne,"./en/browser.md":_,"./en/dataDirectory.md":y,"./en/hooks.md":x,"./en/installation.md":C,"./en/permissions.md":T,"./en/quickStart.md":D,"./en/remote.md":k,"./en/report.md":j,"./en/script.md":N,"./en/skills.md":F,"./en/todo.md":L,"./en/tools.md":z}),Oe=Object.assign({"./zh/agentMode.md":V,"./zh/aiChat.md":U,"./zh/alias.md":G,"./zh/browser.md":q,"./zh/dataDirectory.md":Y,"./zh/hooks.md":ie,"./zh/installation.md":oe,"./zh/permissions.md":ce,"./zh/quickStart.md":ue,"./zh/remote.md":fe,"./zh/report.md":me,"./zh/script.md":ge,"./zh/skills.md":ve,"./zh/todo.md":be,"./zh/tools.md":Se});function ke(){let e={en:{},zh:{}};for(let[t,n]of Object.entries(De)){let r=t.match(/\.\/en\/(\w+)\.md$/);r&&n?.default&&(e.en[r[1]]=n.default)}for(let[t,n]of Object.entries(Oe)){let r=t.match(/\.\/zh\/(\w+)\.md$/);r&&n?.default&&(e.zh[r[1]]=n.default)}return e}var Q=ke();function Ae(e,t){return Q[e]?.[t]||Q.en[t]||``}function $(e,t){return Z[e]?.[t]||Z.en[t]||t}var je={en:{prev:`Previous`,next:`Next`},zh:{prev:`上一页`,next:`下一页`}};function Me({lang:e,activeSection:t,onNavigate:n}){let r=Ee(),i=Z[e],a=je[e],o=r.indexOf(t),s=o>0?r[o-1]:null,c=o<r.length-1?r[o+1]:null;return(0,u.jsxs)(`div`,{className:`flex items-center justify-between py-8 mt-8 border-t border-stone-200`,children:[(0,u.jsx)(`div`,{className:`flex-1`,children:s&&(0,u.jsxs)(`button`,{onClick:()=>n(s),className:`group flex flex-col items-start text-left hover:bg-stone-100 rounded-lg p-3 -ml-3 transition-colors`,children:[(0,u.jsxs)(`span`,{className:`text-xs text-stone-400 mb-1 flex items-center gap-1`,children:[(0,u.jsx)(`svg`,{className:`w-4 h-4`,fill:`none`,stroke:`currentColor`,viewBox:`0 0 24 24`,strokeWidth:2,children:(0,u.jsx)(`path`,{strokeLinecap:`round`,strokeLinejoin:`round`,d:`M15 19l-7-7 7-7`})}),a.prev]}),(0,u.jsx)(`span`,{className:`text-sm font-medium text-stone-700 group-hover:text-stone-900 transition-colors`,children:i[s]})]})}),(0,u.jsx)(`div`,{className:`flex-1 flex justify-end`,children:c&&(0,u.jsxs)(`button`,{onClick:()=>n(c),className:`group flex flex-col items-end text-right hover:bg-stone-100 rounded-lg p-3 -mr-3 transition-colors`,children:[(0,u.jsxs)(`span`,{className:`text-xs text-stone-400 mb-1 flex items-center gap-1`,children:[a.next,(0,u.jsx)(`svg`,{className:`w-4 h-4`,fill:`none`,stroke:`currentColor`,viewBox:`0 0 24 24`,strokeWidth:2,children:(0,u.jsx)(`path`,{strokeLinecap:`round`,strokeLinejoin:`round`,d:`M9 5l7 7-7 7`})})]}),(0,u.jsx)(`span`,{className:`text-sm font-medium text-stone-700 group-hover:text-stone-900 transition-colors`,children:i[c]})]})})]})}function Ne(){let[e,t]=(0,l.useState)(`zh`),[n,r]=(0,l.useState)(!1),[a,o]=(0,l.useState)(`installation`),s=Te[e],f=we[e];return(0,l.useEffect)(()=>{let e=document.getElementById(a);e&&e.scrollIntoView({behavior:`smooth`,block:`start`})},[a]),(0,u.jsxs)(`div`,{className:`min-h-screen bg-[#faf9f6] text-stone-800`,children:[(0,u.jsx)(`nav`,{className:`fixed top-0 left-0 right-0 z-50 bg-[#faf9f6]/95 backdrop-blur-sm border-b border-stone-200/50`,children:(0,u.jsxs)(`div`,{className:`px-4 sm:px-6 py-4 flex items-center justify-between`,children:[(0,u.jsxs)(`div`,{className:`flex items-center gap-3`,children:[(0,u.jsx)(`button`,{onClick:()=>r(!n),className:`lg:hidden p-2 -ml-2 text-stone-500 hover:text-stone-900 transition-colors`,children:(0,u.jsx)(`svg`,{className:`w-6 h-6`,fill:`none`,stroke:`currentColor`,viewBox:`0 0 24 24`,strokeWidth:2,children:n?(0,u.jsx)(`path`,{strokeLinecap:`round`,strokeLinejoin:`round`,d:`M6 18L18 6M6 6l12 12`}):(0,u.jsx)(`path`,{strokeLinecap:`round`,strokeLinejoin:`round`,d:`M4 6h16M4 12h16M4 18h16`})})}),(0,u.jsxs)(i,{to:`/`,className:`flex items-center gap-2`,children:[(0,u.jsx)(`span`,{className:`text-2xl font-bold text-stone-900`,children:`j`}),(0,u.jsx)(`span`,{className:`text-stone-400 text-sm hidden sm:inline`,children:`docs`})]})]}),(0,u.jsxs)(`div`,{className:`flex items-center gap-3 sm:gap-5`,children:[(0,u.jsx)(c,{lang:e,onChange:t}),(0,u.jsxs)(`a`,{href:`https://github.com/LingoJack/j`,target:`_blank`,rel:`noopener noreferrer`,className:`flex items-center gap-2 text-stone-500 hover:text-stone-900 transition-colors`,children:[(0,u.jsx)(`svg`,{className:`w-5 h-5`,fill:`currentColor`,viewBox:`0 0 24 24`,children:(0,u.jsx)(`path`,{fillRule:`evenodd`,clipRule:`evenodd`,d:`M12 2C6.477 2 2 6.477 2 12c0 4.42 2.87 8.17 6.84 9.5.5.08.66-.23.66-.5v-1.69c-2.77.6-3.36-1.34-3.36-1.34-.46-1.16-1.11-1.47-1.11-1.47-.91-.62.07-.6.07-.6 1 .07 1.53 1.03 1.53 1.03.87 1.52 2.34 1.07 2.91.83.09-.65.35-1.09.63-1.34-2.22-.25-4.55-1.11-4.55-4.92 0-1.11.38-2 1.03-2.71-.1-.25-.45-1.29.1-2.64 0 0 .84-.27 2.75 1.02.79-.22 1.65-.33 2.5-.33.85 0 1.71.11 2.5.33 1.91-1.29 2.75-1.02 2.75-1.02.55 1.35.2 2.39.1 2.64.65.71 1.03 1.6 1.03 2.71 0 3.82-2.34 4.66-4.57 4.91.36.31.69.92.69 1.85v2.74c0 .27.16.59.67.5C19.14 20.16 22 16.42 22 12A10 10 0 0012 2z`})}),(0,u.jsx)(`span`,{className:`text-sm hidden sm:inline`,children:s.github})]})]})]})}),(0,u.jsx)(d,{tree:f,activeSection:a,onNavigate:o,isOpen:n,onClose:()=>r(!1)}),(0,u.jsx)(`main`,{className:`lg:ml-72 pt-[65px]`,children:(0,u.jsxs)(`div`,{className:`max-w-3xl mx-auto px-6 pb-16`,children:[(()=>{let t=Ae(e,a),n=$(e,a);return t?(0,u.jsxs)(`div`,{id:a,className:`py-8`,children:[(0,u.jsx)(`h1`,{className:`text-3xl font-light text-stone-900 mb-6`,children:n}),(0,u.jsx)(m,{content:t})]},`${e}-${a}`):null})(),(0,u.jsx)(Me,{lang:e,activeSection:a,onNavigate:o})]})}),(0,u.jsx)(`footer`,{className:`lg:ml-72 border-t border-stone-200 py-8 px-6 bg-[#faf9f6]`,children:(0,u.jsxs)(`div`,{className:`max-w-3xl mx-auto flex items-center justify-between text-sm`,children:[(0,u.jsx)(i,{to:`/`,className:`text-stone-500 hover:text-stone-900 transition-colors`,children:s.back}),(0,u.jsxs)(`div`,{className:`flex items-center gap-6`,children:[(0,u.jsx)(`a`,{href:`https://github.com/LingoJack/j`,target:`_blank`,rel:`noopener noreferrer`,className:`text-stone-500 hover:text-stone-900 transition-colors`,children:`GitHub`}),(0,u.jsx)(`a`,{href:`https://crates.io/crates/j-cli`,target:`_blank`,rel:`noopener noreferrer`,className:`text-stone-500 hover:text-stone-900 transition-colors`,children:`crates.io`})]})]})})]})}export{Ne as default};