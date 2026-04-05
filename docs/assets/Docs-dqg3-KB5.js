import{n as e,r as t}from"./rolldown-runtime-Dw2cE7zH.js";import{r as n,t as r}from"./react-vendor-CTSggWdF.js";import{n as i,t as a}from"./index-BRw6Rn9o.js";import{n as o,t as s}from"./syntax-highlight-DDfxEX0b.js";import{n as c,t as l}from"./LanguageSwitcher-BoZx07nq.js";var u=t(n(),1),d=r();function f({tree:e,activeSection:t,onNavigate:n,isOpen:r,onClose:i}){return(0,d.jsxs)(d.Fragment,{children:[r&&(0,d.jsx)(`div`,{className:`fixed inset-0 bg-black/20 z-40 lg:hidden`,onClick:i}),(0,d.jsx)(`aside`,{className:`
        fixed top-[65px] left-0 bottom-0 w-72 bg-[#faf9f6] border-r border-stone-200 
        overflow-y-auto z-50 transition-transform duration-300
        lg:translate-x-0
        ${r?`translate-x-0`:`-translate-x-full`}
      `,children:(0,d.jsx)(`nav`,{className:`p-6`,children:Object.entries(e).map(([e,r])=>(0,d.jsxs)(`div`,{className:`mb-6`,children:[(0,d.jsx)(`h3`,{className:`text-xs font-semibold text-stone-400 uppercase tracking-wider mb-3`,children:r.title}),(0,d.jsx)(`ul`,{className:`space-y-1`,children:Object.entries(r.children).map(([e,r])=>(0,d.jsx)(`li`,{children:(0,d.jsx)(`button`,{onClick:()=>{n(e),i()},className:`
                        w-full text-left px-3 py-2 rounded-lg text-sm transition-colors
                        ${t===e?`bg-stone-200 text-stone-900 font-medium`:`text-stone-600 hover:bg-stone-100`}
                      `,children:r})},e))})]},e))})})]})}var p={bash:`bash`,shell:`bash`,sh:`bash`,zsh:`bash`,typescript:`typescript`,ts:`typescript`,javascript:`javascript`,js:`javascript`,python:`python`,py:`python`,rust:`rust`,rs:`rust`,go:`go`,golang:`go`,java:`java`,c:`c`,cpp:`cpp`,"c++":`cpp`,csharp:`csharp`,"c#":`csharp`,ruby:`ruby`,rb:`ruby`,sql:`sql`,json:`json`,yaml:`yaml`,yml:`yaml`,toml:`toml`,markdown:`markdown`,md:`markdown`,html:`html`,css:`css`,scss:`scss`};function m(e,t){let n=[],r=e,i=0;for(;r.length>0;){let e=r.match(/`([^`]+)`/),a=r.match(/\*\*([^*]+)\*\*/),o=r.match(/\*([^*]+)\*/),s=[];if(e&&e.index!==void 0&&s.push({type:`code`,match:e,index:e.index}),a&&a.index!==void 0&&s.push({type:`bold`,match:a,index:a.index}),o&&o.index!==void 0&&s.push({type:`italic`,match:o,index:o.index}),s.length===0){n.push((0,d.jsx)(`span`,{children:r},`${t}-txt-${i++}`));break}s.sort((e,t)=>e.index-t.index);let c=s[0],l=r.slice(0,c.index);l&&n.push((0,d.jsx)(`span`,{children:l},`${t}-txt-${i++}`)),c.type===`code`?n.push((0,d.jsx)(`code`,{className:`bg-stone-100 text-stone-700 px-1.5 py-0.5 rounded text-xs font-mono`,children:c.match[1]},`${t}-code-${i++}`)):c.type===`bold`?n.push((0,d.jsx)(`strong`,{className:`font-medium text-stone-900`,children:c.match[1]},`${t}-bold-${i++}`)):c.type===`italic`&&n.push((0,d.jsx)(`em`,{className:`italic`,children:c.match[1]},`${t}-italic-${i++}`)),r=r.slice(c.index+c.match[0].length)}return n.length>0?n:e}function h({content:e}){return(0,d.jsx)(d.Fragment,{children:(0,u.useMemo)(()=>{let t=e.split(`
`),n=[],r=!1,i=``,a=``,l=!1,u=[],f=0,h=()=>{if(u.length>0){let e=Math.max(...u.map(e=>e.length)),t=`table-${f++}`;n.push((0,d.jsx)(`div`,{className:`overflow-x-auto my-4`,children:(0,d.jsxs)(`table`,{className:`min-w-full border-collapse`,children:[(0,d.jsx)(`thead`,{children:(0,d.jsx)(`tr`,{children:u[0]?.map((e,n)=>(0,d.jsx)(`th`,{className:`border border-stone-200 px-4 py-2 text-left bg-stone-50 text-sm font-medium`,children:m(e,`${t}-h${n}`)},`th-${n}`))})}),(0,d.jsx)(`tbody`,{children:u.slice(1).map((n,r)=>(0,d.jsx)(`tr`,{children:Array.from({length:e}).map((e,i)=>(0,d.jsx)(`td`,{className:`border border-stone-200 px-4 py-2 text-sm`,children:m(n[i]||``,`${t}-r${r}c${i}`)},`td-${i}`))},`tr-${r}`))})]})},t)),u=[]}};return t.forEach(e=>{let t=`line-${f++}`;if(e.startsWith("```")){if(!r)h(),r=!0,a=e.slice(3).trim()||`text`,i=``;else{r=!1;let e=p[a.toLowerCase()]||a||`text`;n.push((0,d.jsxs)(`div`,{className:`relative group my-4`,children:[(0,d.jsx)(o,{language:e,style:s,customStyle:{margin:0,borderRadius:`0.5rem`,fontSize:`0.875rem`,backgroundColor:`#faf9f6`,border:`1px solid #e7e5e4`},codeTagProps:{style:{fontFamily:`ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace`}},children:i}),(0,d.jsx)(c,{text:i})]},`code-${f++}`))}return}if(r){i+=(i?`
`:``)+e;return}if(e.startsWith(`|`)){l||(l=!0,u=[]);let t=e.split(`|`).slice(1,-1).map(e=>e.trim());e.includes(`---`)||u.push(t);return}else l&&(l=!1,h());if(e.startsWith(`> `)){n.push((0,d.jsx)(`blockquote`,{className:`border-l-4 border-stone-300 pl-4 py-1 my-3 text-stone-600 text-sm italic`,children:m(e.slice(2),`${t}-q`)},t));return}if(e.startsWith(`## `)){n.push((0,d.jsx)(`h2`,{className:`text-2xl font-light text-stone-900 mt-8 mb-4`,children:m(e.slice(3),`${t}-h2`)},t));return}if(e.startsWith(`### `)){n.push((0,d.jsx)(`h3`,{className:`text-lg font-medium text-stone-900 mt-6 mb-3`,children:m(e.slice(4),`${t}-h3`)},t));return}if(e.startsWith(`- `)||e.startsWith(`* `)){n.push((0,d.jsx)(`li`,{className:`text-stone-600 text-sm ml-4 mb-1 list-disc`,children:m(e.slice(2),`${t}-li`)},t));return}let g=e.match(/^(\d+)\.\s/);if(g){n.push((0,d.jsx)(`li`,{className:`text-stone-600 text-sm ml-4 mb-1 list-decimal`,children:m(e.slice(g[0].length),`${t}-nli`)},t));return}e.trim()&&n.push((0,d.jsx)(`p`,{className:`text-stone-600 text-sm leading-relaxed mb-3`,children:m(e,`${t}-p`)},t))}),l&&h(),n},[e])})}var g=e({default:()=>ee}),ee=`## Overview

Agent mode is an enhanced AI chat mode with autonomous multi-step reasoning and tool usage.

## Start

\`\`\`bash
j chat              # Enter TUI chat
\`\`\`

In the conversation, AI automatically uses tools to execute multi-step operations as needed.

> **Auto-apply tools**: Create \`.jcli/permissions.yaml\` in project root with \`allow_all: true\` to skip all tool confirmations. See "Tool Permission Configuration" below.

## Features

- **Autonomous Reasoning**: AI plans and executes multi-step tasks
- **Tool Integration**: Automatically uses available tools (Read, Write, Bash, etc.)
- **Task Management**: Task and Todo tools manage complex tasks
- **Plan Mode**: EnterPlanMode allows exploring codebase before making plan
- **Sub-agent**: Agent tool can spawn sub-agents for complex tasks in parallel

## Sub-agent (Agent Tool)

Agent tool allows the main agent to spawn sub-agents for complex tasks:

- **No recursion**: Sub-agent cannot call Agent tool
- **Max rounds**: 30 tool call limit
- **Execution mode**: Foreground (blocking) or background (async)
- **Permission control**: Follows deny/allow rules

\`\`\`
# Example: Spawn sub-agent to search and organize code
Search all files containing 'TODO' and organize by directory
\`\`\`

## Plan Mode

For complex tasks, enter plan mode to explore codebase first:

\`\`\`
# Enter plan mode
Analyze this project's architecture and design a refactoring plan

# AI will:
1. Enter plan mode (read-only tools available)
2. Explore codebase structure
3. Generate detailed plan
4. Submit plan for user approval
5. Execute after approval
\`\`\`

## Example Tasks

\`\`\`
Analyze the codebase and suggest improvements

Find all TODO comments in the code and generate a summary

Research React state management best practices and generate a report
\`\`\`

## Tool Permission Configuration

Configure which tools the AI can use (in \`.jcli/permissions.yaml\`):

\`\`\`yaml
permissions:
  allow_all: false   # Set to true to skip all confirmations
  
  allow:             # Skip confirmation if matched
    - Read
    - Grep
    - Glob
    - WebFetch
  
  deny:              # Takes priority over allow, blocks execution
    - Bash
    - Write
\`\`\`
`,te=e({default:()=>ne}),ne=`## Start AI Chat

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
`,re=e({default:()=>_}),_=`## Commands

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
`,v=e({default:()=>y}),y=`## Overview

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

| Action | Description |
|--------|-------------|
| \`status\` | Check browser status |
| \`open\` | Open URL and fetch page content |
| \`snapshot\` | Get interactive element list |
| \`content\` | Extract body text |
| \`tabs\` | List open tabs |
| \`close\` | Close tab |

**Limitations:** Does not support \`click\`, \`type\`, \`press\`, \`evaluate\`, \`screenshot\`

## CDP Mode

Full browser automation when \`browser_cdp\` feature is enabled:

| Action | Description |
|--------|-------------|
| \`start\` | Launch browser |
| \`stop\` | Stop browser |
| \`open\` | Open new tab |
| \`navigate\` | Navigate to URL |
| \`screenshot\` | Capture screenshot (supports full page) |
| \`snapshot\` | Get page snapshot (with element selectors) |
| \`content\` | Extract body text |
| \`click\` | Click element |
| \`type\` | Type text (supports Unicode) |
| \`press\` | Press key (Enter/Tab/Escape etc.) |
| \`evaluate\` | Execute JavaScript |
| \`tabs\` | List tabs |
| \`close\` | Close tab |

### Typical Workflow

\`\`\`
1. open to open a page
2. snapshot to get interactive elements (returns selectors)
3. Use returned selector (e.g., [data-jref="e3"]) for click/type
4. content to get results
\`\`\`

### Headless Configuration

Configure in \`.jcli/config.yaml\`:

\`\`\`yaml
settings:
  browser_headless: true  # true=no window, false=show window
\`\`\`

Or override via parameter: \`{ "action": "start", "headless": false }\`

## Build with CDP

\`\`\`bash
cargo build --features browser_cdp
\`\`\`
`,b=e({default:()=>x}),x="All data is stored in `~/.jdata/` (customizable via `J_DATA_PATH` environment variable):\n\n```\n~/.jdata/\n├── config.yaml          # Main config (aliases, categories, settings)\n├── history.txt          # Command history\n├── agent/               # AI Agent data\n│   ├── data/            # Agent data directory\n│   │   ├── agent_config.yaml   # Agent config (model, API)\n│   │   ├── sessions/           # Chat sessions storage\n│   │   ├── archives/           # Archived conversations\n│   │   ├── system_prompt.md    # System prompt\n│   │   ├── memory.md           # Memory file\n│   │   └── soul.md             # Soul file\n│   ├── logs/            # Agent logs\n│   │   ├── info.log\n│   │   └── error.log\n│   ├── skills/          # User-level skills directory\n│   ├── commands/        # User-level custom commands\n│   └── hooks.yaml       # User-level hooks config\n├── report/              # Daily reports\n│   ├── week_report.md   # Week report file\n│   ├── settings.json    # Report settings\n│   ├── todo.json        # Todo data\n│   └── .git/            # Git repository\n└── scripts/             # Scripts created via j concat\n```\n\n## Project-level Config\n\nCreate `.jcli/` in project directory for project-level configuration:\n\n```\n.jcli/\n├── config.yaml          # Project-level config\n├── permissions.yaml     # Tool permissions\n├── hooks.yaml           # Project-level hooks\n├── skills/              # Project-level skills (override user-level)\n└── commands/            # Project-level custom commands\n```\n\n## Config File Structure (`config.yaml`)\n\n| Section | Description | Example |\n|---------|-------------|---------|\n| `path` | Local app/file paths | `chrome: /Applications/Google Chrome.app` |\n| `inner_url` | URL links | `github: https://github.com` |\n| `outer_url` | URLs requiring VPN | `docs: https://internal.example.com` |\n| `browser` | Browser list | `chrome: chrome` |\n| `editor` | Editor list | `vscode: vscode` |\n| `vpn` | VPN application | |\n| `script` | Registered scripts | `deploy: ~/.jdata/scripts/deploy.sh` |\n| `report` | Report system config | `git_repo: https://github.com/xxx/report` |\n| `setting` | Global settings | `search-engine: bing` |\n| `log` | Log settings | `mode: concise` |\n\n## Agent Config (`agent_config.yaml`)\n\n| Setting | Description | Default |\n|---------|-------------|---------|\n| `providers` | Model provider list | - |\n| `active_index` | Current active provider index | 0 |\n| `system_prompt` | System prompt | - |\n| `stream_mode` | Stream output | true |\n| `max_history_messages` | Max history messages sent to API | 20 |\n| `tools_enabled` | Enable tool calling | false |\n| `max_tool_rounds` | Max tool call rounds | 100 |\n| `tool_confirm_timeout` | Tool confirm timeout seconds | 0 (no timeout) |\n| `disabled_tools` | Disabled tools list | [] |\n| `disabled_skills` | Disabled skills list | [] |\n| `disabled_commands` | Disabled commands list | [] |\n| `auto_restore_session` | Auto restore last session on startup | false |\n",S=e({default:()=>C}),C=`## Overview

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
`,w=e({default:()=>T}),T=`## One-click Install (Recommended)

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
`,E=e({default:()=>D}),D=`## Permission Configuration File

Permissions are configured in \`.jcli/permissions.yaml\` in your project directory:

\`\`\`yaml
permissions:
  # Allow all tools without confirmation
  allow_all: false
  
  # Allow list (skip confirmation if matched)
  allow:
    - Read
    - Grep
    - Glob
    - "Bash(cargo build:*)"
    - "Bash(git status:*)"
  
  # Deny list (takes priority over allow, blocks execution)
  deny:
    - "Bash(rm -rf:*)"
    - "Bash(/.*sudo.*/)"    # Regex match
\`\`\`

## Rule Formats

| Format | Description | Example |
|--------|-------------|---------|
| \`*\` | Match all tools | \`*\` |
| \`ToolName\` | Match all calls to this tool | \`Read\`, \`Grep\` |
| \`ToolName(prefix:*)\` | Prefix match | \`Bash(cargo build:*)\` |
| \`ToolName(path:/dir/*)\` | Path match | \`Write(path:/src/*)\` |
| \`ToolName(domain:example.com)\` | Domain match | \`WebFetch(domain:docs.rs)\` |
| \`ToolName(/regex/)\` | Regex match | \`Bash(/^cargo (build\\|test)/)\` |

## Match Priority

\`\`\`
deny > allow > default requires confirmation
\`\`\`

- \`deny\` list has highest priority, blocks execution if matched
- \`allow\` list skips confirmation if matched
- \`allow_all: true\` skips all confirmations (but deny still takes priority)

## Tool-Specific Rules

### Bash Command Matching

\`\`\`yaml
allow:
  - "Bash(cargo:*)"        # cargo build, cargo test, etc.
  - "Bash(git status:*)"   # git status
  - "Bash(ls:*)"           # ls, ls -la, etc.
  
deny:
  - "Bash(rm -rf:*)"       # Block rm -rf
  - "Bash(/.*sudo.*/)"     # Block all sudo commands
\`\`\`

### File Path Matching (Write/Edit/Read)

\`\`\`yaml
allow:
  - "Write(path:/src/*)"   # Allow writes to /src directory
  - "Edit(path:/lib/*)"    # Allow edits to /lib directory
  
deny:
  - "Write(path:/etc/*)"   # Block writes to /etc
\`\`\`

### URL Domain Matching (WebFetch)

\`\`\`yaml
allow:
  - "WebFetch(domain:docs.rs)"
  - "WebFetch(domain:github.com)"
  - "WebFetch(domain:/.*\\\\.google\\\\.com$/)"  # Regex match all google subdomains
\`\`\`
`,O=e({default:()=>k}),k=`## Register App Aliases

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
`,A=e({default:()=>j}),j=`## Overview

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
`,M=e({default:()=>N}),N=`## Overview

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
`,P=e({default:()=>F}),F=`## Commands

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
`,I=e({default:()=>L}),L=`## Overview

Skills are specialized prompt modules that extend AI capabilities, loaded via the \`LoadSkill\` tool.

## Skill Structure

\`\`\`
~/.jdata/agent/skills/<skill_name>/
├── SKILL.md          # Skill definition (required)
├── references/       # Reference documents (AI reads on demand via Read tool)
└── scripts/          # Script files (AI executes on demand via Bash tool)
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

After loading, AI will:
1. Get the skill's body content as context
2. List file paths in references/ and scripts/ directories
3. Read reference documents on demand via Read tool
4. Execute scripts on demand via Bash tool

## Skill Sources

| Source | Path | Priority |
|--------|------|----------|
| User level | \`~/.jdata/agent/skills/\` | Low |
| Project level | \`.jcli/skills/\` | High (overrides user level) |

## Disabling Skills

Disable specific skills via the TUI configuration interface. Settings are saved in \`~/.jdata/agent/data/agent_config.json\`:

\`\`\`json
{
  "disabled_skills": ["skill-name-1", "skill-name-2"]
}
\`\`\`
`,R=e({default:()=>z}),z=`## Commands

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
`,B=e({default:()=>V}),V='## Available Tools\n\n| Tool | Description |\n|------|-------------|\n| `Read` | Read file contents (supports images: png/jpg/gif/webp/bmp) |\n| `Write` | Write to files (auto-create directories) |\n| `Edit` | Edit files with string replacement (old_string must match uniquely) |\n| `Glob` | Find files by pattern (supports glob like `**/*.rs`) |\n| `Grep` | Regex search file contents (supports context, pagination) |\n| `Bash` | Execute shell commands (supports background execution) |\n| `WebFetch` | Fetch web page content (convert to Markdown or text) |\n| `WebSearch` | Search the web via Exa (requires EXA_API_KEY) |\n| `Ask` | Ask user for structured input (single/multi-select) |\n| `TaskOutput` | Get background task output |\n| `Task` | Manage tasks (create/get/list/update) |\n| `TodoWrite` | Write todo items (only one in_progress allowed) |\n| `TodoRead` | Read todo items list |\n| `Compact` | Compress conversation context (auto-triggered) |\n| `RegisterHook` | Register session-level hooks |\n| `ComputerUse` | macOS desktop control (screenshot, click, type) |\n| `EnterPlanMode` | Enter plan mode (read-only tools) |\n| `ExitPlanMode` | Exit plan mode (submit plan) |\n| `LoadSkill` | Load skills (registered on demand) |\n| `Agent` | Sub-agent for complex tasks (prevents recursion) |\n| `Browser` | Browser tool (Lite/CDP mode) |\n\n## Permission Configuration\n\nPermissions are configured in `.jcli/permissions.yaml` with three rule types:\n\n```yaml\n# .jcli/permissions.yaml\npermissions:\n  # Allow all tools without confirmation\n  allow_all: false\n  \n  # Allow list (skip confirmation if matched, supports regex)\n  allow:\n    - Read\n    - Grep\n    - Glob\n    - "Bash:ls.*"       # Regex match command arguments\n    - "Bash:git status"\n  \n  # Deny list (takes priority over allow, blocks execution)\n  deny:\n    - "Bash:rm -rf.*"   # Block dangerous commands\n    - "Bash:.*sudo.*"   # Block sudo commands\n```\n\n### Rule Matching\n\n- **Simple match**: Tool name (e.g., `Read`, `Bash`)\n- **Regex match**: `ToolName:regex_pattern` (e.g., `Bash:rm.*` matches Bash tool\'s command argument)\n- **Priority**: deny > allow > default requires confirmation\n\n## Context References\n\n| Reference | Description |\n|-----------|-------------|\n| `@file:path` | Include file content (auto-read and inject into context) |\n| `@skill:name` | Load and activate the specified skill |\n',H=e({default:()=>U}),U=`## 概述

Agent 模式是 AI 对话的增强模式，支持自主多步推理和工具调用。

## 启动

\`\`\`bash
j chat              # 进入 TUI 对话
\`\`\`

在对话中，AI 会根据任务需要自动使用工具执行多步操作。

> **自动应用工具**：在项目根目录创建 \`.jcli/permissions.yaml\`，设置 \`allow_all: true\` 可跳过所有工具确认。详见下方"工具权限配置"。

## 功能特性

- **自主推理**：AI 规划并执行多步任务
- **工具集成**：自动使用可用工具（Read、Write、Bash 等）
- **任务管理**：Task 和 Todo 工具管理复杂任务
- **计划模式**：EnterPlanMode 可先探索代码库再制定计划
- **子代理**：Agent 工具可派生子代理并行处理复杂任务

## 子代理（Agent 工具）

Agent 工具允许主代理派生子代理执行复杂任务：

- **防递归**：子代理无法调用 Agent 工具
- **最大轮数**：30 轮工具调用限制
- **执行模式**：前台（阻塞）或后台（异步）
- **权限控制**：遵循 deny/allow 规则

\`\`\`
# 示例：派生子代理搜索并整理代码
搜索所有包含 'TODO' 的文件并按目录分类整理
\`\`\`

## 计划模式

对于复杂任务，可先进入计划模式探索代码库：

\`\`\`
# 进入计划模式
分析这个项目的架构并设计重构方案

# AI 会：
1. 进入计划模式（只读工具可用）
2. 探索代码库结构
3. 生成详细计划
4. 提交计划等待用户确认
5. 用户确认后执行
\`\`\`

## 示例任务

\`\`\`
分析代码库并提出改进建议

查找代码中的所有 TODO 注释并生成摘要

研究 React 状态管理的最佳实践并生成报告
\`\`\`

## 工具权限配置

配置 AI 可以使用的工具（位于 \`.jcli/permissions.yaml\`）：

\`\`\`yaml
permissions:
  allow_all: false   # 设为 true 跳过所有确认
  
  allow:             # 匹配则跳过确认
    - Read
    - Grep
    - Glob
    - WebFetch
  
  deny:              # 优先于 allow，直接拒绝
    - Bash
    - Write
\`\`\`
`,W=e({default:()=>G}),G=`## 启动 AI 对话

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
`,K=e({default:()=>q}),q=`## 命令

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
`,J=e({default:()=>Y}),Y=`## 概述

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

| Action | 描述 |
|--------|------|
| \`status\` | 检查浏览器状态 |
| \`open\` | 打开 URL，获取页面内容 |
| \`snapshot\` | 获取交互元素列表 |
| \`content\` | 提取正文文本 |
| \`tabs\` | 列出已打开的标签页 |
| \`close\` | 关闭标签页 |

**限制**：不支持 \`click\`、\`type\`、\`press\`、\`evaluate\`、\`screenshot\`

## CDP 模式

启用 \`browser_cdp\` feature 后支持完整浏览器自动化：

| Action | 描述 |
|--------|------|
| \`start\` | 启动浏览器 |
| \`stop\` | 停止浏览器 |
| \`open\` | 打开新标签页 |
| \`navigate\` | 导航到 URL |
| \`screenshot\` | 截图（支持全页） |
| \`snapshot\` | 获取页面快照（含元素 selector） |
| \`content\` | 提取正文文本 |
| \`click\` | 点击元素 |
| \`type\` | 输入文本（支持中文） |
| \`press\` | 按键（Enter/Tab/Escape 等） |
| \`evaluate\` | 执行 JavaScript |
| \`tabs\` | 列出标签页 |
| \`close\` | 关闭标签页 |

### 典型工作流

\`\`\`
1. open 打开页面
2. snapshot 获取可交互元素列表（返回 selector）
3. 使用返回的 selector（如 [data-jref="e3"]）进行 click/type
4. content 获取结果
\`\`\`

### headless 配置

在 \`.jcli/config.yaml\` 中配置：

\`\`\`yaml
settings:
  browser_headless: true  # true=无窗口，false=显示窗口
\`\`\`

或通过参数覆盖：\`{ "action": "start", "headless": false }\`

## 编译启用 CDP

\`\`\`bash
cargo build --features browser_cdp
\`\`\`
`,X=e({default:()=>ie}),ie="所有数据存储在 `~/.jdata/` 目录（可通过 `J_DATA_PATH` 环境变量自定义）：\n\n```\n~/.jdata/\n├── config.yaml          # 主配置（别名、分类、设置）\n├── history.txt          # 命令历史\n├── agent/               # AI Agent 数据\n│   ├── data/            # Agent 数据目录\n│   │   ├── agent_config.yaml   # Agent 配置（模型、API）\n│   │   ├── sessions/           # 对话会话存储\n│   │   ├── archives/           # 归档对话\n│   │   ├── system_prompt.md    # 系统提示词\n│   │   ├── memory.md           # 记忆文件\n│   │   └── soul.md             # 灵魂文件\n│   ├── logs/            # Agent 日志\n│   │   ├── info.log\n│   │   └── error.log\n│   ├── skills/          # 用户级技能目录\n│   ├── commands/        # 用户级自定义命令\n│   └── hooks.yaml       # 用户级钩子配置\n├── report/              # 日报数据\n│   ├── week_report.md   # 周报文件\n│   ├── settings.json    # 报告设置\n│   ├── todo.json        # 待办数据\n│   └── .git/            # Git 仓库\n└── scripts/             # 通过 j concat 创建的脚本\n```\n\n## 项目级配置\n\n项目目录下可创建 `.jcli/` 存放项目级配置：\n\n```\n.jcli/\n├── config.yaml          # 项目级配置\n├── permissions.yaml     # 工具权限配置\n├── hooks.yaml           # 项目级钩子\n├── skills/              # 项目级技能（覆盖用户级）\n└── commands/            # 项目级自定义命令\n```\n\n## 配置文件结构（`config.yaml`）\n\n| 配置项 | 描述 | 示例 |\n|--------|------|------|\n| `path` | 本地应用/文件路径 | `chrome: /Applications/Google Chrome.app` |\n| `inner_url` | URL 链接 | `github: https://github.com` |\n| `outer_url` | 需要 VPN 的 URL | `docs: https://internal.example.com` |\n| `browser` | 浏览器列表 | `chrome: chrome` |\n| `editor` | 编辑器列表 | `vscode: vscode` |\n| `vpn` | VPN 应用 | |\n| `script` | 注册脚本 | `deploy: ~/.jdata/scripts/deploy.sh` |\n| `report` | 日报系统配置 | `git_repo: https://github.com/xxx/report` |\n| `setting` | 全局设置 | `search-engine: bing` |\n| `log` | 日志设置 | `mode: concise` |\n\n## Agent 配置（`agent_config.yaml`）\n\n| 配置项 | 描述 | 默认值 |\n|--------|------|--------|\n| `providers` | 模型提供方列表 | - |\n| `active_index` | 当前选中的 provider 索引 | 0 |\n| `system_prompt` | 系统提示词 | - |\n| `stream_mode` | 流式输出 | true |\n| `max_history_messages` | 发送给 API 的历史消息数量限制 | 20 |\n| `tools_enabled` | 启用工具调用 | false |\n| `max_tool_rounds` | 工具调用最大轮数 | 100 |\n| `tool_confirm_timeout` | 工具确认超时秒数 | 0（不超时） |\n| `disabled_tools` | 禁用的工具列表 | [] |\n| `disabled_skills` | 禁用的 skill 列表 | [] |\n| `disabled_commands` | 禁用的 command 列表 | [] |\n| `auto_restore_session` | 启动时自动恢复最近的 session | false |\n",ae=e({default:()=>oe}),oe=`## 概述

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
`,se=e({default:()=>ce}),ce=`## 一键安装（推荐）

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
`,le=e({default:()=>ue}),ue=`## 权限配置文件

权限配置位于项目目录 \`.jcli/permissions.yaml\`：

\`\`\`yaml
permissions:
  # 完全放开（跳过所有工具确认）
  allow_all: false
  
  # 允许列表（匹配则跳过确认）
  allow:
    - Read
    - Grep
    - Glob
    - "Bash(cargo build:*)"
    - "Bash(git status:*)"
  
  # 拒绝列表（优先于 allow，匹配则直接拒绝执行）
  deny:
    - "Bash(rm -rf:*)"
    - "Bash(/.*sudo.*/)"    # 正则匹配
\`\`\`

## 规则格式

| 格式 | 说明 | 示例 |
|------|------|------|
| \`*\` | 匹配所有工具 | \`*\` |
| \`ToolName\` | 匹配该工具所有调用 | \`Read\`, \`Grep\` |
| \`ToolName(prefix:*)\` | 前缀匹配 | \`Bash(cargo build:*)\` |
| \`ToolName(path:/dir/*)\` | 路径匹配 | \`Write(path:/src/*)\` |
| \`ToolName(domain:example.com)\` | 域名匹配 | \`WebFetch(domain:docs.rs)\` |
| \`ToolName(/regex/)\` | 正则匹配 | \`Bash(/^cargo (build\\|test)/)\` |

## 匹配优先级

\`\`\`
deny > allow > 默认需要确认
\`\`\`

- \`deny\` 列表优先级最高，匹配则直接拒绝执行
- \`allow\` 列表匹配则跳过确认
- \`allow_all: true\` 跳过所有确认（但 deny 仍优先）

## 工具特定规则

### Bash 命令匹配

\`\`\`yaml
allow:
  - "Bash(cargo:*)"        # cargo build, cargo test 等
  - "Bash(git status:*)"   # git status
  - "Bash(ls:*)"           # ls, ls -la 等
  
deny:
  - "Bash(rm -rf:*)"       # 阻止 rm -rf
  - "Bash(/.*sudo.*/)"     # 阻止所有 sudo 命令
\`\`\`

### 文件路径匹配（Write/Edit/Read）

\`\`\`yaml
allow:
  - "Write(path:/src/*)"   # 允许写入 /src 目录
  - "Edit(path:/lib/*)"    # 允许编辑 /lib 目录
  
deny:
  - "Write(path:/etc/*)"   # 阻止写入 /etc
\`\`\`

### URL 域名匹配（WebFetch）

\`\`\`yaml
allow:
  - "WebFetch(domain:docs.rs)"
  - "WebFetch(domain:github.com)"
  - "WebFetch(domain:/.*\\\\.google\\\\.com$/)"  # 正则匹配所有 google 子域名
\`\`\`
`,de=e({default:()=>fe}),fe=`## 注册应用别名

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
`,pe=e({default:()=>me}),me=`## 概述

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
`,he=e({default:()=>ge}),ge=`## 概述

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
`,_e=e({default:()=>ve}),ve=`## 命令

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
`,ye=e({default:()=>be}),be=`## 概述

Skill 是扩展 AI 能力的专用提示词模块，通过 \`LoadSkill\` 工具加载。

## Skill 结构

\`\`\`
~/.jdata/agent/skills/<skill_name>/
├── SKILL.md          # Skill 定义（必需）
├── references/       # 参考文档（AI 按需 Read 读取）
└── scripts/          # 脚本文件（AI 按需 Bash 执行）
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

加载后，AI 会：
1. 获取 skill 的 body 内容作为上下文
2. 列出 references/ 和 scripts/ 目录中的文件路径
3. 按需使用 Read 工具读取参考文档
4. 按需使用 Bash 工具执行脚本

## Skill 来源

| 来源 | 路径 | 优先级 |
|------|------|--------|
| 用户级 | \`~/.jdata/agent/skills/\` | 低 |
| 项目级 | \`.jcli/skills/\` | 高（覆盖用户级） |

## 禁用 Skill

在 TUI 配置界面中禁用特定 skill，配置保存在 \`~/.jdata/agent/data/agent_config.json\`：

\`\`\`json
{
  "disabled_skills": ["skill-name-1", "skill-name-2"]
}
\`\`\`
`,xe=e({default:()=>Se}),Se=`## 命令

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
`,Ce=e({default:()=>we}),we='## 可用工具\n\n| 工具 | 描述 |\n|------|------|\n| `Read` | 读取文件内容（支持图片：png/jpg/gif/webp/bmp） |\n| `Write` | 写入文件（自动创建目录） |\n| `Edit` | 字符串替换编辑文件（old_string 必须唯一匹配） |\n| `Glob` | 按模式查找文件（支持 `**/*.rs` 等 glob 模式） |\n| `Grep` | 正则搜索文件内容（支持 context、pagination） |\n| `Bash` | 执行 shell 命令（支持后台执行） |\n| `WebFetch` | 获取网页内容（转 Markdown 或纯文本） |\n| `WebSearch` | Exa 搜索网络（需 EXA_API_KEY） |\n| `Ask` | 向用户请求结构化输入（单选/多选） |\n| `TaskOutput` | 获取后台任务输出 |\n| `Task` | 管理任务（create/get/list/update） |\n| `TodoWrite` | 写入待办事项（仅一个 in_progress） |\n| `TodoRead` | 读取待办事项列表 |\n| `Compact` | 压缩对话上下文（自动触发） |\n| `RegisterHook` | 注册 session 级钩子 |\n| `ComputerUse` | macOS 桌面控制（截图、点击、输入） |\n| `EnterPlanMode` | 进入计划模式（只读工具） |\n| `ExitPlanMode` | 退出计划模式（提交计划） |\n| `LoadSkill` | 加载技能（按需注册） |\n| `Agent` | 子代理执行复杂任务（防止递归） |\n| `Browser` | 浏览器工具（Lite/CDP 模式） |\n\n## 权限配置\n\n权限配置位于 `.jcli/permissions.yaml`，支持三种规则：\n\n```yaml\n# .jcli/permissions.yaml\npermissions:\n  # 完全放开（跳过所有工具确认）\n  allow_all: false\n  \n  # 允许列表（匹配则跳过确认，支持正则）\n  allow:\n    - Read\n    - Grep\n    - Glob\n    - "Bash:ls.*"       # 正则匹配命令参数\n    - "Bash:git status"\n  \n  # 拒绝列表（优先于 allow，匹配则直接拒绝）\n  deny:\n    - "Bash:rm -rf.*"   # 阻止危险命令\n    - "Bash:.*sudo.*"   # 阻止 sudo 命令\n```\n\n### 规则匹配说明\n\n- **简单匹配**：工具名（如 `Read`、`Bash`）\n- **正则匹配**：`工具名:正则表达式`（如 `Bash:rm.*` 匹配 Bash 工具的 command 参数）\n- **优先级**：deny > allow > 默认需要确认\n\n## 上下文引用\n\n| 引用 | 描述 |\n|------|------|\n| `@file:路径` | 包含文件内容（自动读取并注入上下文） |\n| `@skill:名称` | 加载并激活指定 skill |\n',Te={en:{gettingStarted:{title:`Getting Started`,children:{installation:`Installation`,quickStart:`Quick Start`,dataDirectory:`Data Directory`}},coreFeatures:{title:`Core Features`,children:{alias:`Alias Management`,report:`Daily Reports`,todo:`Todo Management`,script:`Script System`}},aiFeatures:{title:`AI Features`,children:{aiChat:`AI Chat`,agentMode:`Agent Mode`,tools:`AI Tools`,skills:`Skill System`,hooks:`Hook System`}},advanced:{title:`Advanced`,children:{browser:`Browser Automation`,remote:`Remote Control`,permissions:`Permissions`}}},zh:{gettingStarted:{title:`快速开始`,children:{installation:`安装`,quickStart:`快速上手`,dataDirectory:`数据目录`}},coreFeatures:{title:`核心功能`,children:{alias:`别名管理`,report:`日报系统`,todo:`待办管理`,script:`脚本系统`}},aiFeatures:{title:`AI 功能`,children:{aiChat:`AI 对话`,agentMode:`Agent 模式`,tools:`AI 工具`,skills:`Skill 技能`,hooks:`Hook 系统`}},advanced:{title:`进阶功能`,children:{browser:`浏览器自动化`,remote:`远程控制`,permissions:`权限配置`}}}},Ee={en:{back:`← Back to Home`,github:`GitHub`,menu:`Menu`},zh:{back:`← 返回首页`,github:`GitHub`,menu:`菜单`}},Z={en:{installation:`Installation`,quickStart:`Quick Start`,dataDirectory:`Data Directory`,alias:`Alias Management`,report:`Daily Reports`,todo:`Todo Management`,script:`Script System`,aiChat:`AI Chat`,agentMode:`Agent Mode`,tools:`AI Tools`,skills:`Skill System`,hooks:`Hook System`,browser:`Browser Automation`,remote:`Remote Control`,permissions:`Permissions`},zh:{installation:`安装`,quickStart:`快速上手`,dataDirectory:`数据目录`,alias:`别名管理`,report:`日报系统`,todo:`待办管理`,script:`脚本系统`,aiChat:`AI 对话`,agentMode:`Agent 模式`,tools:`AI 工具`,skills:`Skill 技能`,hooks:`Hook 系统`,browser:`浏览器自动化`,remote:`远程控制`,permissions:`权限配置`}};function De(){return[`installation`,`quickStart`,`dataDirectory`,`alias`,`report`,`todo`,`script`,`aiChat`,`agentMode`,`tools`,`skills`,`hooks`,`browser`,`remote`,`permissions`]}var Oe=Object.assign({"./en/agentMode.md":g,"./en/aiChat.md":te,"./en/alias.md":re,"./en/browser.md":v,"./en/dataDirectory.md":b,"./en/hooks.md":S,"./en/installation.md":w,"./en/permissions.md":E,"./en/quickStart.md":O,"./en/remote.md":A,"./en/report.md":M,"./en/script.md":P,"./en/skills.md":I,"./en/todo.md":R,"./en/tools.md":B}),Q=Object.assign({"./zh/agentMode.md":H,"./zh/aiChat.md":W,"./zh/alias.md":K,"./zh/browser.md":J,"./zh/dataDirectory.md":X,"./zh/hooks.md":ae,"./zh/installation.md":se,"./zh/permissions.md":le,"./zh/quickStart.md":de,"./zh/remote.md":pe,"./zh/report.md":he,"./zh/script.md":_e,"./zh/skills.md":ye,"./zh/todo.md":xe,"./zh/tools.md":Ce});function ke(){let e={en:{},zh:{}};for(let[t,n]of Object.entries(Oe)){let r=t.match(/\.\/en\/(\w+)\.md$/);r&&n?.default&&(e.en[r[1]]=n.default)}for(let[t,n]of Object.entries(Q)){let r=t.match(/\.\/zh\/(\w+)\.md$/);r&&n?.default&&(e.zh[r[1]]=n.default)}return e}var $=ke();function Ae(e,t){return $[e]?.[t]||$.en[t]||``}function je(e,t){return Z[e]?.[t]||Z.en[t]||t}var Me={en:{prev:`Previous`,next:`Next`},zh:{prev:`上一页`,next:`下一页`}};function Ne({lang:e,activeSection:t,onNavigate:n}){let r=De(),i=Z[e],a=Me[e],o=r.indexOf(t),s=o>0?r[o-1]:null,c=o<r.length-1?r[o+1]:null;return(0,d.jsxs)(`div`,{className:`flex items-center justify-between py-8 mt-8 border-t border-stone-200`,children:[(0,d.jsx)(`div`,{className:`flex-1`,children:s&&(0,d.jsxs)(`button`,{onClick:()=>n(s),className:`group flex flex-col items-start text-left hover:bg-stone-100 rounded-lg p-3 -ml-3 transition-colors`,children:[(0,d.jsxs)(`span`,{className:`text-xs text-stone-400 mb-1 flex items-center gap-1`,children:[(0,d.jsx)(`svg`,{className:`w-4 h-4`,fill:`none`,stroke:`currentColor`,viewBox:`0 0 24 24`,strokeWidth:2,children:(0,d.jsx)(`path`,{strokeLinecap:`round`,strokeLinejoin:`round`,d:`M15 19l-7-7 7-7`})}),a.prev]}),(0,d.jsx)(`span`,{className:`text-sm font-medium text-stone-700 group-hover:text-stone-900 transition-colors`,children:i[s]})]})}),(0,d.jsx)(`div`,{className:`flex-1 flex justify-end`,children:c&&(0,d.jsxs)(`button`,{onClick:()=>n(c),className:`group flex flex-col items-end text-right hover:bg-stone-100 rounded-lg p-3 -mr-3 transition-colors`,children:[(0,d.jsxs)(`span`,{className:`text-xs text-stone-400 mb-1 flex items-center gap-1`,children:[a.next,(0,d.jsx)(`svg`,{className:`w-4 h-4`,fill:`none`,stroke:`currentColor`,viewBox:`0 0 24 24`,strokeWidth:2,children:(0,d.jsx)(`path`,{strokeLinecap:`round`,strokeLinejoin:`round`,d:`M9 5l7 7-7 7`})})]}),(0,d.jsx)(`span`,{className:`text-sm font-medium text-stone-700 group-hover:text-stone-900 transition-colors`,children:i[c]})]})})]})}function Pe(){let[e,t]=i(),[n,r]=(0,u.useState)(`zh`),[o,s]=(0,u.useState)(!1),c=e.get(`section`)||`installation`,p=Ee[n],m=Te[n],g=e=>{t({section:e})};return(0,u.useEffect)(()=>{let e=document.getElementById(c);e&&e.scrollIntoView({behavior:`smooth`,block:`start`})},[c]),(0,d.jsxs)(`div`,{className:`min-h-screen bg-[#faf9f6] text-stone-800`,children:[(0,d.jsx)(`nav`,{className:`fixed top-0 left-0 right-0 z-50 bg-[#faf9f6]/95 backdrop-blur-sm border-b border-stone-200/50`,children:(0,d.jsxs)(`div`,{className:`px-4 sm:px-6 py-4 flex items-center justify-between`,children:[(0,d.jsxs)(`div`,{className:`flex items-center gap-3`,children:[(0,d.jsx)(`button`,{onClick:()=>s(!o),className:`lg:hidden p-2 -ml-2 text-stone-500 hover:text-stone-900 transition-colors`,children:(0,d.jsx)(`svg`,{className:`w-6 h-6`,fill:`none`,stroke:`currentColor`,viewBox:`0 0 24 24`,strokeWidth:2,children:o?(0,d.jsx)(`path`,{strokeLinecap:`round`,strokeLinejoin:`round`,d:`M6 18L18 6M6 6l12 12`}):(0,d.jsx)(`path`,{strokeLinecap:`round`,strokeLinejoin:`round`,d:`M4 6h16M4 12h16M4 18h16`})})}),(0,d.jsxs)(a,{to:`/`,className:`flex items-center gap-2`,children:[(0,d.jsx)(`span`,{className:`text-2xl font-bold text-stone-900`,children:`j`}),(0,d.jsx)(`span`,{className:`text-stone-400 text-sm hidden sm:inline`,children:`docs`})]})]}),(0,d.jsxs)(`div`,{className:`flex items-center gap-3 sm:gap-5`,children:[(0,d.jsx)(l,{lang:n,onChange:r}),(0,d.jsxs)(`a`,{href:`https://github.com/LingoJack/j`,target:`_blank`,rel:`noopener noreferrer`,className:`flex items-center gap-2 text-stone-500 hover:text-stone-900 transition-colors`,children:[(0,d.jsx)(`svg`,{className:`w-5 h-5`,fill:`currentColor`,viewBox:`0 0 24 24`,children:(0,d.jsx)(`path`,{fillRule:`evenodd`,clipRule:`evenodd`,d:`M12 2C6.477 2 2 6.477 2 12c0 4.42 2.87 8.17 6.84 9.5.5.08.66-.23.66-.5v-1.69c-2.77.6-3.36-1.34-3.36-1.34-.46-1.16-1.11-1.47-1.11-1.47-.91-.62.07-.6.07-.6 1 .07 1.53 1.03 1.53 1.03.87 1.52 2.34 1.07 2.91.83.09-.65.35-1.09.63-1.34-2.22-.25-4.55-1.11-4.55-4.92 0-1.11.38-2 1.03-2.71-.1-.25-.45-1.29.1-2.64 0 0 .84-.27 2.75 1.02.79-.22 1.65-.33 2.5-.33.85 0 1.71.11 2.5.33 1.91-1.29 2.75-1.02 2.75-1.02.55 1.35.2 2.39.1 2.64.65.71 1.03 1.6 1.03 2.71 0 3.82-2.34 4.66-4.57 4.91.36.31.69.92.69 1.85v2.74c0 .27.16.59.67.5C19.14 20.16 22 16.42 22 12A10 10 0 0012 2z`})}),(0,d.jsx)(`span`,{className:`text-sm hidden sm:inline`,children:p.github})]})]})]})}),(0,d.jsx)(f,{tree:m,activeSection:c,onNavigate:g,isOpen:o,onClose:()=>s(!1)}),(0,d.jsx)(`main`,{className:`lg:ml-72 pt-[65px]`,children:(0,d.jsxs)(`div`,{className:`max-w-3xl mx-auto px-6 pb-16`,children:[(()=>{let e=Ae(n,c),t=je(n,c);return e?(0,d.jsxs)(`div`,{id:c,className:`py-8`,children:[(0,d.jsx)(`h1`,{className:`text-3xl font-light text-stone-900 mb-6`,children:t}),(0,d.jsx)(h,{content:e})]},`${n}-${c}`):null})(),(0,d.jsx)(Ne,{lang:n,activeSection:c,onNavigate:g})]})}),(0,d.jsx)(`footer`,{className:`lg:ml-72 border-t border-stone-200 py-8 px-6 bg-[#faf9f6]`,children:(0,d.jsxs)(`div`,{className:`max-w-3xl mx-auto flex items-center justify-between text-sm`,children:[(0,d.jsx)(a,{to:`/`,className:`text-stone-500 hover:text-stone-900 transition-colors`,children:p.back}),(0,d.jsxs)(`div`,{className:`flex items-center gap-6`,children:[(0,d.jsx)(`a`,{href:`https://github.com/LingoJack/j`,target:`_blank`,rel:`noopener noreferrer`,className:`text-stone-500 hover:text-stone-900 transition-colors`,children:`GitHub`}),(0,d.jsx)(`a`,{href:`https://crates.io/crates/j-cli`,target:`_blank`,rel:`noopener noreferrer`,className:`text-stone-500 hover:text-stone-900 transition-colors`,children:`crates.io`})]})]})})]})}export{Pe as default};