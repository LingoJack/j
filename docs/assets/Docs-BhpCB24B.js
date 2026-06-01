import{n as e,r as t}from"./rolldown-runtime-Dw2cE7zH.js";import{r as n,t as r}from"./react-vendor-CTSggWdF.js";import{n as i,t as a}from"./index-DDvf-pcx.js";import{n as o,t as s}from"./syntax-highlight-DDfxEX0b.js";import{n as c,t as l}from"./LanguageSwitcher-BoZx07nq.js";var u=t(n(),1),d=r();function f({tree:e,activeSection:t,onNavigate:n,isOpen:r,onClose:i}){return(0,d.jsxs)(d.Fragment,{children:[r&&(0,d.jsx)(`div`,{className:`fixed inset-0 bg-black/20 z-40 lg:hidden`,onClick:i}),(0,d.jsx)(`aside`,{className:`
        fixed top-[65px] left-0 bottom-0 w-64 bg-[#faf9f6] border-r border-stone-200/70
        overflow-y-auto z-50 transition-transform duration-300
        lg:translate-x-0 scrollbar-thin
        ${r?`translate-x-0`:`-translate-x-full`}
      `,children:(0,d.jsx)(`nav`,{className:`p-4`,children:Object.entries(e).map(([e,r],a,o)=>(0,d.jsxs)(`div`,{className:`mb-6 ${a<o.length-1?`pb-6 border-b border-stone-200/50`:``}`,children:[(0,d.jsxs)(`h3`,{className:`flex items-center gap-2 text-xs font-semibold text-stone-400 uppercase tracking-wider mb-3 px-3`,children:[(0,d.jsx)(`span`,{className:`w-1 h-4 bg-stone-300 rounded-full flex-shrink-0`}),r.title]}),(0,d.jsx)(`ul`,{className:`space-y-0.5`,children:Object.entries(r.children).map(([e,r])=>(0,d.jsx)(`li`,{children:(0,d.jsx)(`button`,{onClick:()=>{n(e),i()},className:`
                        relative w-full text-left px-3 py-2 rounded-lg text-sm transition-all duration-200
                        ${t===e?`text-stone-900 font-medium bg-stone-100 before:absolute before:left-0 before:top-1/2 before:-translate-y-1/2 before:w-0.5 before:h-5 before:bg-stone-900 before:rounded-full`:`text-stone-500 hover:text-stone-700 hover:bg-stone-50`}
                      `,children:r})},e))})]},e))})})]})}var p={bash:`bash`,shell:`bash`,sh:`bash`,zsh:`bash`,powershell:`powershell`,ps1:`powershell`,typescript:`typescript`,ts:`typescript`,javascript:`javascript`,js:`javascript`,python:`python`,py:`python`,rust:`rust`,rs:`rust`,go:`go`,golang:`go`,java:`java`,c:`c`,cpp:`cpp`,"c++":`cpp`,csharp:`csharp`,"c#":`csharp`,ruby:`ruby`,rb:`ruby`,sql:`sql`,json:`json`,yaml:`yaml`,yml:`yaml`,toml:`toml`,markdown:`markdown`,md:`markdown`,html:`html`,css:`css`,scss:`scss`};function m(e){return e.toLowerCase().replace(/[^\w\u4e00-\u9fa5]+/g,`-`).replace(/^-+|-+$/g,``).slice(0,50)}function h(e,t){let n=[],r=e,i=0;for(;r.length>0;){let e=r.match(/`([^`]+)`/),a=r.match(/\*\*([^*]+)\*\*/),o=r.match(/\*([^*]+)\*/),s=r.match(/~~([^~]+)~~/),c=[];if(e&&e.index!==void 0&&c.push({type:`code`,match:e,index:e.index}),a&&a.index!==void 0&&c.push({type:`bold`,match:a,index:a.index}),o&&o.index!==void 0&&c.push({type:`italic`,match:o,index:o.index}),s&&s.index!==void 0&&c.push({type:`strike`,match:s,index:s.index}),c.length===0){n.push((0,d.jsx)(`span`,{children:r},`${t}-txt-${i++}`));break}c.sort((e,t)=>e.index-t.index);let l=c[0],u=r.slice(0,l.index);u&&n.push((0,d.jsx)(`span`,{children:u},`${t}-txt-${i++}`)),l.type===`code`?n.push((0,d.jsx)(`code`,{className:`bg-stone-100 text-stone-700 px-1.5 py-0.5 rounded text-xs font-mono`,children:l.match[1]},`${t}-code-${i++}`)):l.type===`bold`?n.push((0,d.jsx)(`strong`,{className:`font-medium text-stone-900`,children:l.match[1]},`${t}-bold-${i++}`)):l.type===`italic`?n.push((0,d.jsx)(`em`,{className:`italic`,children:l.match[1]},`${t}-italic-${i++}`)):l.type===`strike`&&n.push((0,d.jsx)(`del`,{className:`line-through text-stone-400`,children:l.match[1]},`${t}-strike-${i++}`)),r=r.slice(l.index+l.match[0].length)}return n.length>0?n:e}function g({unixCode:e,windowsCode:t,blockKey:n}){let[r,i]=(0,u.useState)(`unix`),a=r===`unix`?e:t,l=p[a.lang.toLowerCase()]||a.lang||`text`,f=(0,u.useCallback)(e=>i(e),[]);return(0,d.jsxs)(`div`,{className:`my-4`,children:[(0,d.jsxs)(`div`,{className:`flex gap-2 mb-3`,children:[(0,d.jsx)(`button`,{onClick:()=>f(`unix`),className:`px-2.5 py-1 text-xs font-medium rounded-lg transition-colors ${r===`unix`?`bg-stone-900 text-white`:`bg-stone-100 text-stone-500 hover:bg-stone-200`}`,children:`macOS / Linux`}),(0,d.jsx)(`button`,{onClick:()=>f(`windows`),className:`px-2.5 py-1 text-xs font-medium rounded-lg transition-colors ${r===`windows`?`bg-stone-900 text-white`:`bg-stone-100 text-stone-500 hover:bg-stone-200`}`,children:`Windows`})]}),(0,d.jsxs)(`div`,{className:`relative group`,children:[(0,d.jsx)(o,{language:l,style:s,customStyle:{margin:0,borderRadius:`0.5rem`,fontSize:`0.875rem`,backgroundColor:`#faf9f6`,border:`1px solid #e7e5e4`},codeTagProps:{style:{fontFamily:`ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace`}},children:a.content}),(0,d.jsx)(c,{text:a.content})]})]},n)}var _=/^###\s+(macOS\s*\/\s*Linux|macOS|Linux)\s*$/,v=/^###\s+Windows\s*$/;function ee(e){let t=[],n=0,r=[],i=()=>{r.length>0&&(t.push({type:`raw`,lines:r}),r=[])};for(;n<e.length;){if(_.test(e[n])){let r=y(e,n+1);if(r){let a=r.endIdx+1;for(;a<e.length&&e[a].trim()===``;)a++;if(a<e.length&&v.test(e[a])){let o=y(e,a+1);if(o){i(),t.push({type:`platform-pair`,unixCode:{lang:r.lang,content:r.content},windowsCode:{lang:o.lang,content:o.content}}),n=o.endIdx+1;continue}}}}r.push(e[n]),n++}return i(),t}function y(e,t){let n=t;for(;n<e.length&&e[n].trim()===``;)n++;if(n>=e.length||!e[n].startsWith("```"))return null;let r=e[n].slice(3).trim()||`text`,i=``;for(n++;n<e.length&&!e[n].startsWith("```");)i+=(i?`
`:``)+e[n],n++;return n>=e.length?null:{lang:r,content:i,endIdx:n}}function te({content:e}){return(0,d.jsx)(d.Fragment,{children:(0,u.useMemo)(()=>{let t=ee(e.split(`
`)),n=[],r=0,i=new Set,a=e=>{if(e.length>0){let t=Math.max(...e.map(e=>e.length)),i=`table-${r++}`;n.push((0,d.jsx)(`div`,{className:`overflow-x-auto my-4`,children:(0,d.jsxs)(`table`,{className:`min-w-full border-collapse`,children:[(0,d.jsx)(`thead`,{children:(0,d.jsx)(`tr`,{children:e[0]?.map((e,t)=>(0,d.jsx)(`th`,{className:`border border-stone-200 px-4 py-2 text-left bg-stone-50 text-sm font-medium`,children:h(e,`${i}-h${t}`)},`th-${t}`))})}),(0,d.jsx)(`tbody`,{children:e.slice(1).map((e,n)=>(0,d.jsx)(`tr`,{children:Array.from({length:t}).map((t,r)=>(0,d.jsx)(`td`,{className:`border border-stone-200 px-4 py-2 text-sm`,children:h(e[r]||``,`${i}-r${n}c${r}`)},`td-${r}`))},`tr-${n}`))})]})},i))}};function l(e){let t=!1,l=``,u=``,f=!1,g=[],_=()=>{f&&(f=!1,a(g),g=[])};e.forEach(e=>{let a=`line-${r++}`;if(e.startsWith("```")){if(!t)_(),t=!0,u=e.slice(3).trim()||`text`,l=``;else{t=!1;let e=p[u.toLowerCase()]||u||`text`;n.push((0,d.jsxs)(`div`,{className:`relative group my-4`,children:[(0,d.jsx)(o,{language:e,style:s,customStyle:{margin:0,borderRadius:`0.5rem`,fontSize:`0.875rem`,backgroundColor:`#faf9f6`,border:`1px solid #e7e5e4`},codeTagProps:{style:{fontFamily:`ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace`}},children:l}),(0,d.jsx)(c,{text:l})]},`code-${r++}`))}return}if(t){l+=(l?`
`:``)+e;return}if(e.startsWith(`|`)){f||(f=!0,g=[]);let t=e.split(`|`).slice(1,-1).map(e=>e.trim());e.includes(`---`)||g.push(t);return}else f&&_();if(e.startsWith(`> `)){n.push((0,d.jsx)(`blockquote`,{className:`border-l-4 border-stone-300 pl-4 py-1 my-3 text-stone-600 text-sm italic`,children:h(e.slice(2),`${a}-q`)},a));return}if(e.startsWith(`## `)){let t=e.slice(3).trim(),r=m(t.replace(/\*\*([^*]+)\*\*/g,`$1`).replace(/`([^`]+)`/g,`$1`)),o=1;for(;i.has(r);)r=`${m(t.replace(/\*\*([^*]+)\*\*/g,`$1`).replace(/`([^`]+)`/g,`$1`))}-${o}`,o++;i.add(r),n.push((0,d.jsx)(`h2`,{id:r,className:`text-2xl font-light text-stone-900 mt-12 mb-5`,children:h(t,`${a}-h2`)},a));return}if(e.startsWith(`### `)){let t=e.slice(4).trim(),r=m(t.replace(/\*\*([^*]+)\*\*/g,`$1`).replace(/`([^`]+)`/g,`$1`)),o=1;for(;i.has(r);)r=`${m(t.replace(/\*\*([^*]+)\*\*/g,`$1`).replace(/`([^`]+)`/g,`$1`))}-${o}`,o++;i.add(r),n.push((0,d.jsx)(`h3`,{id:r,className:`text-lg font-medium text-stone-900 mt-8 mb-4`,children:h(t,`${a}-h3`)},a));return}if(e.startsWith(`#### `)){let t=e.slice(5).trim(),r=m(t.replace(/\*\*([^*]+)\*\*/g,`$1`).replace(/`([^`]+)`/g,`$1`)),o=1;for(;i.has(r);)r=`${m(t.replace(/\*\*([^*]+)\*\*/g,`$1`).replace(/`([^`]+)`/g,`$1`))}-${o}`,o++;i.add(r),n.push((0,d.jsx)(`h4`,{id:r,className:`text-base font-semibold text-stone-800 mt-6 mb-3`,children:h(t,`${a}-h4`)},a));return}if(e.startsWith(`- `)||e.startsWith(`* `)){n.push((0,d.jsx)(`li`,{className:`text-stone-600 text-sm ml-4 mb-1 list-disc`,children:h(e.slice(2),`${a}-li`)},a));return}let v=e.match(/^(\d+)\.\s/);if(v){n.push((0,d.jsx)(`li`,{className:`text-stone-600 text-sm ml-4 mb-1 list-decimal`,children:h(e.slice(v[0].length),`${a}-nli`)},a));return}e.trim()&&n.push((0,d.jsx)(`p`,{className:`text-stone-600 text-sm leading-relaxed mb-3`,children:h(e,`${a}-p`)},a))}),f&&_()}for(let e of t)if(e.type===`platform-pair`){let t=`platform-${r++}`;n.push((0,d.jsx)(g,{blockKey:t,unixCode:e.unixCode,windowsCode:e.windowsCode},t))}else l(e.lines);return n},[e])})}var ne=e({default:()=>re}),re=`## Overview

AI chat system with multi-model support, context references, and autonomous agent execution.

## Start Chat

\`\`\`bash
j chat              # Enter TUI chat interface
j chat "Hello"      # Quick question with response printed
j chat -c           # Continue previous session
j chat --session <id>  # Restore specific session
\`\`\`

## Shortcuts

| Shortcut | Action |
|----------|--------|
| \`Enter\` | Send message |
| \`Esc\` | Cancel response/Exit |
| \`Ctrl+Y\` | Copy last AI reply |
| \`Ctrl+B\` | Message browse mode |
| \`Ctrl+G\` | Open log windows |
| \`Ctrl+O\` | Toggle tool details |
| \`Ctrl+E\` | Open config panel |
| \`F1\` or \`?\` | Show help |

## Slash Commands

Type \`/\` in the input box to trigger slash commands:

| Command | Action |
|---------|--------|
| \`/copy\` | Copy last AI reply |
| \`/log\` | Open log windows |
| \`/browse\` | Browse message history |
| \`/config\` | Open config panel |
| \`/model\` | Switch model |
| \`/archive\` | Archive current conversation |

## Context References

Type \`@\` in the input box to trigger completion:

\`\`\`
@skill:<name>       # Reference a skill
@command:<name>     # Reference a custom command
@file:<path>        # Reference file content (supports images)
\`\`\`

## Agent Capabilities

AI chat has built-in Agent capabilities for autonomous multi-step task execution:

- **Autonomous Reasoning**: AI plans and executes multi-step tasks
- **Tool Integration**: Automatically uses available tools (Read, Write, Bash/PowerShell, etc.)
- **Task Management**: Task and Todo tools manage complex tasks
- **Plan Mode**: Explore codebase before making a plan

### Available Tools

| Tool | macOS / Linux | Windows | Description |
|------|:---:|:---:|-------------|
| Read | ✅ | ✅ | Read files |
| Write | ✅ | ✅ | Write files |
| Edit | ✅ | ✅ | Edit files |
| Glob | ✅ | ✅ | File name search |
| Grep | ✅ | ✅ | Content search |
| Bash | ✅ | ❌ | Shell command execution |
| PowerShell | ❌ | ✅ | PowerShell command execution |
| WebFetch | ✅ | ✅ | Web page fetching |
| WebSearch | ✅ | ✅ | Web search |
| ComputerUse | ✅ | ❌ | Screen capture & interaction |
| Browser | ✅ | ✅ | Browser automation |

### Plan Mode

For complex tasks, enter plan mode to explore the codebase first:

\`\`\`
Analyze the project architecture and design a refactoring plan

# AI will:
1. Enter plan mode (read-only tools available)
2. Explore codebase structure
3. Generate detailed plan
4. Submit plan for user confirmation
\`\`\`

### Tool Permission Configuration

Create \`.jcli/permissions.yaml\` in project root:

\`\`\`yaml
permissions:
  allow_all: false
  allow:
    - Read
    - Grep
    - Glob
  deny:
    - Bash          # macOS / Linux
    - PowerShell    # Windows
    - Write
\`\`\`

## Remote Control

\`\`\`bash
j chat --remote     # Enable remote control (scan QR code with phone)
j chat --remote --port 9390  # Custom port
\`\`\`
`,ie=e({default:()=>ae}),ae=`## Overview

Alias system for creating short aliases to paths and URLs for quick access.

> **Tip**: Press **Tab** to auto-complete file paths. Supports \`~\` expansion. E.g., \`j set work ~/Pro<Tab>\`.

## Basic Usage

### Add Alias

\`\`\`bash
j set <alias> <path>    # Add path alias
j set <alias> <url>     # Add URL alias
\`\`\`

### Execute Alias

\`\`\`bash
j <alias>               # Open path or URL
\`\`\`

### Manage Aliases

\`\`\`bash
j rm <alias>            # Remove alias
j rename <old> <new>    # Rename alias
j mf <alias> <new_path> # Modify alias target
\`\`\`

## Alias Types

### Path Alias

\`\`\`bash
# Add path
j set work ~/Projects/work
j set notes ~/Documents/notes

# Open path
j work    # Open in file manager
j notes   # Open in file manager
\`\`\`

### URL Alias

\`\`\`bash
# Add URL
j set gh https://github.com
j set gh-issues https://github.com/issues

# Open URL
j gh        # Open in browser
j gh-issues # Open in browser
\`\`\`

## Alias Storage

Aliases are stored in the configuration file under the data directory:

| Platform | Config File Path |
|----------|-----------------|
| macOS / Linux | \`~/.jdata/alias.yaml\` |
| Windows | \`%USERPROFILE%\\.jdata\\alias.yaml\` |

\`\`\`yaml
# macOS / Linux
path:
  chrome: "/Applications/Google Chrome.app"
  vscode: "/Applications/Visual Studio Code.app"
  work: /Users/user/Projects/work

inner_url:
  gh: https://github.com
  gh-issues: https://github.com/issues

# Windows
path:
  notepad: "C:\\\\Windows\\\\notepad.exe"
  vscode: "C:\\\\Users\\\\user\\\\AppData\\\\Local\\\\Programs\\\\Microsoft VS Code\\\\Code.exe"
  work: "C:\\\\Users\\\\user\\\\Projects\\\\work"

inner_url:
  gh: https://github.com
\`\`\`
`,oe=e({default:()=>b}),b=`## Overview

Browser is a tool in AI chat for web browsing, interaction, and content extraction.

## Platform Support

| Platform | Lite Mode | CDP Mode |
|----------|:---:|:---:|
| macOS | ✅ | ✅ |
| Linux | ✅ | ✅ |
| Windows | ✅ | ✅ |

> **Note**: CDP mode on Windows requires Chrome or Edge browser to be installed.

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
`,x=e({default:()=>S}),S=`## Overview

Commands are reusable prompt snippets that help you quickly invoke preset prompts.

## Slash Commands

Type \`/\` in the input box to trigger slash commands:

| Command | Description |
|---------|-------------|
| \`/copy\` | Copy the last AI response |
| \`/log\` | Open the log window |
| \`/browse\` | Browse message history |
| \`/config\` | Open the configuration panel |
| \`/model\` | Switch model |
| \`/archive\` | Archive current conversation |
| \`/clear\` | New conversation |
| \`/theme\` | Switch theme |
| \`/resume\` | Resume historical session |
| \`/dump\` | Export raw session messages |
| \`/dump-processed\` | Export processed session data |
| \`/teammate\` | Teammate panel |

## Custom Commands

### Usage

Reference a command with \`@command:<name>\` in the input box:

\`\`\`
@command:review Please review this code
\`\`\`

### Creating Commands

#### Directory Locations

| Level | macOS / Linux | Windows |
|-------|---------------|---------|
| User-level | \`~/.jdata/agent/commands/\` | \`%USERPROFILE%\\.jdata\\agent\\commands\\\` |
| Project-level | \`.jcli/commands/\` | \`.jcli\\commands\\\` |

Project-level commands have higher priority.

#### File Format

Each command is a Markdown file with YAML frontmatter and prompt body:

\`\`\`markdown
---
name: review
description: Code review prompt
---
Please perform a comprehensive review of the following code, focusing on:
- Code quality
- Potential issues
- Improvement suggestions
\`\`\`

#### Two Organization Methods

**Method 1: Single File**

Create \`.md\` files directly in the commands directory:

\`\`\`
commands/
  review.md
  test.md
\`\`\`

**Method 2: Directory Structure**

Create a directory with a \`COMMAND.md\` inside (suitable for complex commands with resource files):

\`\`\`
commands/
  review/
    COMMAND.md
    checklist.txt
\`\`\`

### Example

Create a \`plan.md\`:

\`\`\`markdown
---
name: plan
description: Enter PLAN mode
---
Please enter plan mode to plan the task
\`\`\`

Usage:

\`\`\`
@command:plan
\`\`\`

### Managing Commands

Press \`Ctrl+E\` in the TUI to open the configuration panel, switch to the Commands tab to enable or disable commands.
`,C=e({default:()=>w}),w=`## Overview

All j-cli data is stored in a unified user data directory, with support for custom paths via environment variable.

## Data Directory Path

| Platform | Default Path | Environment Variable Override |
|----------|-------------|------------------------------|
| macOS / Linux | \`~/.jdata/\` | \`J_DATA_PATH=/custom/path\` |
| Windows | \`%USERPROFILE%\\.jdata\\\` | \`$env:J_DATA_PATH="C:\\custom\\path"\` |

### Environment Variable Override

\`\`\`bash
# macOS / Linux
export J_DATA_PATH=/custom/path
j chat  # Data will be stored in /custom/path/

# Windows (PowerShell)
$env:J_DATA_PATH="C:\\custom\\path"
j chat  # Data will be stored in C:\\custom\\path\\
\`\`\`

## Directory Structure

\`\`\`
~/.jdata/                          # macOS / Linux
%USERPROFILE%\\.jdata\\              # Windows
├── config.yaml                    # Main configuration
├── alias.yaml                     # Alias definitions
├── report/                        # Daily/weekly reports
│   ├── report.md                  # Report file
│   ├── todo.json                  # Todo data
│   └── settings.json              # Week metadata
├── scripts/                       # User scripts
│   ├── deploy.sh                  # macOS / Linux
│   └── deploy.cmd                 # Windows
├── agent/                         # AI Agent data
│   ├── data/                      # Agent runtime data
│   │   ├── messages/              # Chat history
│   │   └── agent_config.json      # Agent configuration
│   └── skills/                    # User-defined skills
│       └── <skill_name>/
│           └── SKILL.md
└── hooks/                         # Hook scripts
    └── pre_chat/
        └── my_hook.sh             # macOS / Linux
        └── my_hook.cmd            # Windows
\`\`\`

## Configuration Files

### config.yaml

Main configuration file for global settings:

\`\`\`yaml
# API Configuration
api_key: "your-api-key"
base_url: "https://api.openai.com/v1"
model: "gpt-4"

# Report Configuration
report_file_path: "~/.jdata/report/report.md"

# Browser Configuration
settings:
  browser_headless: true
\`\`\`

### alias.yaml

Alias definitions for apps and URLs:

\`\`\`yaml
# macOS / Linux
chrome:
  path: "/Applications/Google Chrome.app"
  note: "browser"

# Windows
notepad:
  path: "C:\\\\Windows\\\\notepad.exe"
  note: "editor"

# URL Alias
github:
  path: "https://github.com"
  type: "inner_url"
\`\`\`

## Data Migration

### Backup

\`\`\`bash
# macOS / Linux
cp -r ~/.jdata ~/.jdata.backup

# Windows
Copy-Item "$env:USERPROFILE\\.jdata" "$env:USERPROFILE\\.jdata.backup" -Recurse
\`\`\`

### Restore

\`\`\`bash
# macOS / Linux
cp -r ~/.jdata.backup ~/.jdata

# Windows
Copy-Item "$env:USERPROFILE\\.jdata.backup" "$env:USERPROFILE\\.jdata" -Recurse
\`\`\`

### Cross-platform Migration

The data directory structure is consistent across all platforms. You can copy it directly:

1. Back up the source platform's data directory
2. Copy to the target platform's corresponding location
3. Adjust script file extensions (\`.sh\` → \`.cmd\`)
4. Update paths in alias.yaml to the target platform's format
`,T=e({default:()=>E}),E=`## Overview

The Hook system allows custom scripts to execute automatically at specific points during the AI chat lifecycle.

## Hook Types

| Hook | Trigger | Use Case |
|------|---------|----------|
| \`pre_chat\` | Before chat starts | Inject system prompts, set up environment |
| \`post_chat\` | After chat ends | Cleanup, notifications |
| \`pre_tool\` | Before tool execution | Security checks, parameter modification |
| \`post_tool\` | After tool execution | Logging, result processing |
| \`on_error\` | On error | Error notification, graceful degradation |

## Hook Configuration

Hooks are defined in \`.jcli/config.yaml\`:

\`\`\`yaml
hooks:
  pre_chat:
    - name: load_project_context
      command: "cat AGENTS.md 2>/dev/null || echo 'No AGENTS.md found'"
      timeout: 5
    - name: check_env
      command: "echo 'Environment ready'"
      timeout: 3
  post_tool:
    - name: log_tool_usage
      command: "echo 'Tool executed at $(date)' >> ~/.jdata/tool.log"
      timeout: 5
\`\`\`

## Execution

### macOS / Linux

Hook commands execute via \`sh -c\`:

\`\`\`bash
sh -c "<hook_command>"
\`\`\`

Supports all standard shell commands including pipes, redirection, and command substitution.

### Windows

Hook commands execute via \`cmd /c\`:

\`\`\`cmd
cmd /c "<hook_command>"
\`\`\`

Supports cmd built-in commands and external programs. Pipe \`|\`, redirection \`>\`, etc. are available.

> **Note**: Windows hooks do not support bash syntax (e.g., \`$(date)\`, \`2>/dev/null\`). Use cmd equivalents (e.g., \`%DATE%\`, \`2>NUL\`).

## Timeout Handling

- Default timeout: 10 seconds
- Process is killed on timeout:
  - macOS / Linux: SIGKILL signal
  - Windows: \`taskkill /F /T /PID\`
- Timeout does not block the chat flow; the current hook is simply skipped

## Environment Variables

Hooks have access to the following environment variables:

| Variable | Description |
|----------|-------------|
| \`J_DATA_PATH\` | Data directory path |
| \`J_<ALIAS>\` | App path for the alias (uppercase, \`-\` → \`_\`) |

### macOS / Linux Example

\`\`\`yaml
hooks:
  pre_chat:
    - name: open_project
      command: "cat $J_DATA_PATH/agent/data/context.txt"
      timeout: 5
\`\`\`

### Windows Example

\`\`\`yaml
hooks:
  pre_chat:
    - name: open_project
      command: "type %J_DATA_PATH%\\agent\\data\\context.txt"
      timeout: 5
\`\`\`

## PATH Injection

The hook directory is automatically prepended to PATH:
- macOS / Linux: uses \`:\` separator
- Windows: uses \`;\` separator

## Hook Script Files

In addition to inline commands, you can create standalone script files:

### macOS / Linux

\`\`\`bash
# ~/.jdata/hooks/pre_chat/my_hook.sh
#!/bin/bash
echo "Loading project context..."
cat AGENTS.md 2>/dev/null
\`\`\`

### Windows

\`\`\`cmd
@echo off
REM %USERPROFILE%\\.jdata\\hooks\\pre_chat\\my_hook.cmd
echo Loading project context...
type AGENTS.md 2>NUL
\`\`\`

## Notes

- Hook failures do not block the conversation
- Command output is injected as a system message into the chat context
- Avoid long-running operations in hooks
- Do not output sensitive information through hooks
`,D=e({default:()=>O}),O=`## One-click Install (Recommended)

### macOS / Linux

\`\`\`bash
# Install latest version
curl -fsSL https://raw.githubusercontent.com/LingoJack/jcli/main/install.sh | sh

# Install specific version
curl -fsSL https://raw.githubusercontent.com/LingoJack/jcli/main/install.sh | sh -s -- v1.0.0
\`\`\`

### Windows

\`\`\`powershell
# Install latest version
irm https://raw.githubusercontent.com/LingoJack/jcli/main/install.ps1 | iex

# Install specific version
$v="v1.0.0"; irm https://raw.githubusercontent.com/LingoJack/jcli/main/install.ps1 | iex
\`\`\`

> Windows install location: \`%LOCALAPPDATA%\\j-cli\\j.exe\`, automatically added to user PATH

## Install from crates.io

\`\`\`bash
# Standard version (Lite browser mode, no extra dependencies)
cargo install j-cli

# Full version (CDP browser mode, requires Chrome/Chromium)
cargo install j-cli --features browser_cdp
\`\`\`

## Build from Source

\`\`\`bash
git clone https://github.com/LingoJack/jcli.git
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

### macOS / Linux

\`\`\`bash
# Using install script (recommended)
curl -fsSL https://raw.githubusercontent.com/LingoJack/jcli/main/install.sh | sh -s -- --uninstall

# Or via cargo
cargo uninstall j-cli

# Or manual removal
sudo rm /usr/local/bin/j  # One-click install
rm ~/.cargo/bin/j          # Cargo install

# (Optional) Remove data directory
rm -rf ~/.jdata
\`\`\`

### Windows

\`\`\`powershell
# Using install script (recommended)
powershell -ExecutionPolicy Bypass -File install.ps1 -Uninstall

# Or manual removal
Remove-Item "$env:LOCALAPPDATA\\j-cli" -Recurse -Force

# (Optional) Remove data directory
Remove-Item "$env:USERPROFILE\\.jdata" -Recurse -Force
\`\`\`

## Platform Differences

| Feature | macOS / Linux | Windows |
|---------|---------------|---------|
| AI Shell Tool | Bash | PowerShell |
| Default Script | \`.sh\` + bash shebang | \`.cmd\` |
| Auto Update | \`.tar.gz\` extract | \`.zip\` extract |
| Data Directory | \`~/.jdata/\` | \`%USERPROFILE%\\.jdata\\\` |
| Install Location | \`/usr/local/bin/j\` | \`%LOCALAPPDATA%\\j-cli\\j.exe\` |
| Computer Use | Supported | Not supported |
| j-indicator | Menu bar indicator | Not supported |
`,k=e({default:()=>A}),A=`## Overview

File encryption and decryption using **AES-256-GCM** symmetric encryption, supporting single files or batch directory operations.

## Basic Usage

### Encrypt a file

\`\`\`bash
j lock <password> <file-or-directory-path>
\`\`\`

### Decrypt a file

\`\`\`bash
j unlock <password> <file-or-directory-path>
\`\`\`

## Parameters

| Parameter | Description |
|-----------|-------------|
| password | Used to derive the AES-256 key; not stored on disk |
| file-or-directory-path | Target file or directory; directories are processed recursively |

## Examples

### Encrypt a single file

\`\`\`bash
j lock mypassword secret.txt
# Output: secret.txt.lock (original file is deleted)
\`\`\`

### Decrypt a single file

\`\`\`bash
j unlock mypassword secret.txt.lock
# Output: secret.txt (.lock file is deleted)
\`\`\`

### Batch encrypt a directory

\`\`\`bash
j lock mypassword ./documents/
# Recursively encrypts all files, generating .lock files for each
\`\`\`

### Batch decrypt a directory

\`\`\`bash
j unlock mypassword ./documents/
# Recursively decrypts all .lock files in the directory
\`\`\`

## Encryption Details

| Item | Description |
|------|-------------|
| Algorithm | AES-256-GCM (authenticated encryption, tamper-proof) |
| Key derivation | HKDF-SHA256, derives 256-bit key from password + random Salt |
| Randomness | Each encryption uses an independent random Salt (32 bytes) and Nonce (12 bytes) |

### Encrypted File Format

\`\`\`
MAGIC(4) + VERSION(1) + SALT(32) + NONCE(12) + CIPHERTEXT+TAG(16)
\`\`\`

| Field | Length | Description |
|-------|--------|-------------|
| MAGIC | 4 bytes | \`JLCK\`, identifies j-cli encrypted files |
| VERSION | 1 byte | Format version (currently \`0x01\`) |
| SALT | 32 bytes | HKDF salt |
| NONCE | 12 bytes | AES-GCM random nonce |
| CIPHERTEXT+TAG | variable + 16 bytes | Encrypted data + GCM authentication tag |

## Behavior

| Behavior | Description |
|----------|-------------|
| After encryption | **Deletes the original file**, keeping only the \`.lock\` encrypted file |
| After decryption | **Deletes the \`.lock\` file**, keeping only the restored original file |
| Skip rule | Skips files already having the \`.lock\` suffix during encryption |
| Hidden files | Skips hidden files and directories (starting with \`.\`) |
| Symlinks | Skips symbolic links |

## Important Notes

- **Remember your password** — lost passwords cannot recover file contents
- Passwords are not stored in the system; must be entered each time
- Original files are deleted after encryption; verify encryption succeeded before cleanup`,j=e({default:()=>M}),M=`## Overview

The built-in Markdown editor is a Typora-like terminal editor with line-level rendering toggle and full Vim mode support.

Core features:
- **Line-level rendering**: Current editing line shows source code, other lines show rendered output
- **Vim mode**: Full support for Normal, Insert, Visual, Command, and Search modes
- **Live preview**: Instant rendering of headings, code blocks, tables, lists, etc.

## Vim Modes

### Mode Switching

| Mode | Border Color | Description |
|------|--------------|-------------|
| Normal | Dark gray | Default browsing mode |
| Insert | Cyan | Text editing mode |
| Visual | Yellow | Visual selection mode |
| Command | Dark gray | Command mode (\`:\`) |
| Search | Magenta | Search mode (\`/\`) |

### Normal Mode Shortcuts

| Shortcut | Action |
|----------|--------|
| \`h/j/k/l\` | Move left/down/up/right |
| \`w/b/e\` | Word forward/back/end |
| \`0/$\` | Line start/end |
| \`g/G\` | File top/bottom |
| \`i/a/A/I\` | Enter Insert mode |
| \`o/O\` | Insert new line below/above |
| \`x/X\` | Delete character |
| \`dd\` | Delete line |
| \`dw/d$\` | Delete word/to end of line |
| \`cc\` | Change entire line |
| \`cw/c$\` | Change word/to end of line |
| \`yy\` | Yank (copy) line |
| \`p\` | Paste |
| \`u\` | Undo |
| \`Ctrl+R\` | Redo |
| \`v\` | Enter Visual mode |
| \`:\` | Enter Command mode |
| \`/\` | Enter Search mode |
| \`n/N\` | Next/previous search match |

### Insert Mode

| Shortcut | Action |
|----------|--------|
| \`Esc\` | Return to Normal mode |
| Others | Normal text input |

### Visual Mode

| Shortcut | Action |
|----------|--------|
| \`h/j/k/l\` | Extend selection |
| \`y\` | Yank (copy) selection |
| \`Esc\` | Return to Normal mode |

### Command Mode

| Command | Action |
|---------|--------|
| \`:w\` | Save and submit |
| \`:wq\` | Save and submit |
| \`:x\` | Save and submit |
| \`:q\` | Cancel editing |
| \`:q!\` | Cancel editing |

### Search Mode

- Press \`Enter\` after typing search pattern to search
- \`n\` jumps to next match
- \`N\` jumps to previous match

## Global Shortcuts

| Shortcut | Action |
|----------|--------|
| \`Ctrl+S\` | Save and submit |
| \`Ctrl+Q\` | Cancel editing |
| \`PageUp/PageDown\` | Page scroll |

## Markdown Rendering

### Headings

\`\`\`
# Heading 1     →  ◆ Heading 1
## Heading 2    →  ◇ Heading 2
### Heading 3   →  〈 Heading 3
#### Heading 4  →  › Heading 4
\`\`\`

### Code Blocks

Code blocks render with a bordered style:

\`\`\`
┌─ rust ────────────┐
│ let x = 42;       │
│ println!("{}", x);│
└───────────────────┘
\`\`\`

Syntax highlighting is supported, language identifier extracted from fence line (e.g., \` \`\`\`rust\`).

### Tables

Auto-aligned column widths, rendered as a formatted table:

\`\`\`
│ Header1 │ Header2 │
├────────┼────────┤
│ cell1  │ cell2  │
\`\`\`

### Other Elements

| Syntax | Rendered |
|--------|----------|
| \`**bold**\` | **bold** |
| \`*italic*\` | *italic* |
| \`~~strikethrough~~\` | ~~strikethrough~~ |
| \`\` \`code\` \`\` | \`code\` |
| \`- list item\` | • list item |
| \`- [ ] task\` | ○ task |
| \`- [x] done\` | ● done |
| \`> quote\` | │ quote |
| \`[link](url)\` | link ↗ |
| \`![image](url)\` | 🖼 alt |

## Use Cases

- Writing daily/weekly reports
- Editing Markdown documents
- Quick note-taking
- Code snippet editing

The editor returns the edited content on save, and returns empty on cancel.
`,N=e({default:()=>P}),P=`## Permission Configuration File

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

### Platform Differences

| Tool | macOS / Linux | Windows |
|------|---------------|---------|
| Shell | \`Bash(...)\` | \`PowerShell(...)\` |
| Computer Use | \`ComputerUse(...)\` | Not available |

> **Note**: On Windows, use \`PowerShell(...)\` rules instead of \`Bash(...)\`.

### Bash / PowerShell Command Matching

macOS / Linux:

\`\`\`yaml
allow:
  - "Bash(cargo:*)"        # cargo build, cargo test, etc.
  - "Bash(git status:*)"   # git status
  - "Bash(ls:*)"           # ls, ls -la, etc.

deny:
  - "Bash(rm -rf:*)"       # Block rm -rf
  - "Bash(/.*sudo.*/)"     # Block all sudo commands
\`\`\`

Windows:

\`\`\`yaml
allow:
  - "PowerShell(cargo:*)"        # cargo build, cargo test, etc.
  - "PowerShell(git status:*)"   # git status
  - "PowerShell(dir:*)"          # dir, dir /s, etc.

deny:
  - "PowerShell(Remove-Item -Recurse -Force:*)"  # Block recursive force delete
  - "PowerShell(/.*Format-.*/)"                   # Block format commands
\`\`\`

### File Path Matching (Write/Edit/Read)

\`\`\`yaml
# macOS / Linux
allow:
  - "Write(path:/src/*)"   # Allow writes to /src directory
  - "Edit(path:/lib/*)"    # Allow edits to /lib directory

deny:
  - "Write(path:/etc/*)"   # Block writes to /etc

# Windows
allow:
  - "Write(path:C:\\\\Projects\\\\myapp\\\\src\\\\*)"  # Allow writes to project src directory
  - "Edit(path:C:\\\\Projects\\\\myapp\\\\lib\\\\*)"   # Allow edits to project lib directory

deny:
  - "Write(path:C:\\\\Windows\\\\System32\\\\*)"     # Block writes to system directory
\`\`\`

### URL Domain Matching (WebFetch)

\`\`\`yaml
allow:
  - "WebFetch(domain:docs.rs)"
  - "WebFetch(domain:github.com)"
  - "WebFetch(domain:/.*\\\\.google\\\\.com$/)"  # Regex match all google subdomains
\`\`\`
`,F=e({default:()=>I}),I=`## Register App Aliases

### macOS / Linux

\`\`\`bash
j set chrome "/Applications/Google Chrome.app"
j set vscode "/Applications/Visual Studio Code.app"

# Register URL aliases (auto-detected as inner_url)
j set github https://github.com
\`\`\`

### Windows

\`\`\`powershell
j set notepad "C:\\Windows\\notepad.exe"
j set vscode "C:\\Users\\%USERNAME%\\AppData\\Local\\Programs\\Microsoft VS Code\\Code.exe"

# Register URL aliases
j set github https://github.com
\`\`\`

## Mark Categories

\`\`\`bash
j note chrome browser
j note vscode editor
\`\`\`

## Open Apps

### macOS / Linux

\`\`\`bash
j chrome                  # Open Chrome
j chrome github           # Open github URL with Chrome
j chrome "rust lang"      # Search "rust lang" with Chrome
j vscode ./src            # Open src directory with VSCode
\`\`\`

### Windows

\`\`\`powershell
j notepad                 # Open Notepad
j vscode .\\src            # Open src directory with VSCode
j github                  # Open github URL
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
\`\`\``,L=e({default:()=>R}),R=`## Overview

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
`,z=e({default:()=>B}),B=`## Overview

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

| File | macOS / Linux | Windows |
|------|---------------|---------|
| Main config | \`~/.jdata/config.yaml\` | \`%USERPROFILE%\\.jdata\\config.yaml\` |
| Week metadata | \`<report_dir>/settings.json\` | \`<report_dir>\\settings.json\` |

| File | Description |
|------|-------------|
| Main config | report_file_path, git_repo settings |
| Week metadata | week_num, last_day info |

## Auto Week Switch

When current date exceeds \`last_day\`, writing to report automatically:
1. Generates new week title \`# WeekN[start_date - end_date]\`
2. Updates week_num and last_day
`,V=e({default:()=>H}),H=`## Overview

Script system for creating and managing executable scripts via \`concat\` command.

## Basic Usage

### Create Script

\`\`\`bash
j concat <name>              # Open TUI editor to write script
j concat <name> "<content>"  # Create script directly
\`\`\`

### Edit Script

\`\`\`bash
j concat <name>              # Enter edit mode if script exists
\`\`\`

### Run Script

\`\`\`bash
j <name>           # Run via alias directly
j <name> <args...> # Run with arguments
\`\`\`

### Delete Script

\`\`\`bash
j rm <name>        # Remove alias (also deletes script file)
\`\`\`

## Script Storage

Scripts are stored in platform-specific directories:

| Platform | Storage Path |
|----------|-------------|
| macOS / Linux | \`~/.jdata/scripts/\` |
| Windows | \`%USERPROFILE%\\.jdata\\scripts\\\` |

\`\`\`
# macOS / Linux
~/.jdata/scripts/
├── deploy.sh
├── build.sh
└── test.sh

# Windows
%USERPROFILE%\\.jdata\\scripts\\
├── deploy.cmd
├── build.cmd
└── test.cmd
\`\`\`

Scripts are automatically registered as aliases after creation, executable via \`j <name>\`.

## Platform Differences

| Feature | macOS / Linux | Windows |
|---------|---------------|---------|
| Script extension | \`.sh\` | \`.cmd\` |
| Default shebang | \`#!/bin/bash\` | (none) |
| Shell | Bash | cmd.exe |
| PATH separator | \`:\` | \`;\` |

## Example

### macOS / Linux

\`\`\`bash
# Create deploy script
j concat deploy

# In editor, input:
#!/bin/bash
set -e
npm run build
rsync -avz dist/ user@server:/var/www/

# Run script
j deploy
\`\`\`

### Windows

\`\`\`powershell
# Create deploy script
j concat deploy

# In editor, input:
@echo off
npm run build
xcopy dist\\ \\\\server\\www\\ /E /I /Y

# Run script
j deploy
\`\`\`

## Environment Variables

All registered alias paths are automatically injected as environment variables when executing scripts. Naming rule: \`J_<ALIAS_UPPERCASE>\` (hyphens converted to underscores).

### macOS / Linux (.sh)

\`\`\`bash
#!/bin/bash
# Registered: chrome -> /Applications/Google Chrome.app
# Registered: my-tool -> /usr/local/bin/my-tool

open -a "$J_CHROME" https://example.com
"$J_MY_TOOL" --version
\`\`\`

### Windows (.cmd)

\`\`\`cmd
@echo off
REM Registered: notepad -> C:\\Windows\\notepad.exe
REM Registered: vscode -> C:\\Users\\%USERNAME%\\AppData\\Local\\Programs\\Microsoft VS Code\\Code.exe

start "" "%J_VSCODE%" .\\src
"%J_NOTEPAD%" readme.txt
\`\`\`

> Paths with spaces must be wrapped in double quotes: \`"$J_CHROME"\` / \`"%J_VSCODE%"\``,U=e({default:()=>W}),W=`## Overview

Skills are specialized prompt modules that extend AI capabilities, loaded via the \`LoadSkill\` tool.

## Skill Structure

\`\`\`
# macOS / Linux
~/.jdata/agent/skills/<skill_name>/
├── SKILL.md          # Skill definition (required)
├── references/       # Reference documents (AI reads on demand via Read tool)
└── scripts/          # Script files (AI executes on demand via Bash/PowerShell tool)

# Windows
%USERPROFILE%\\.jdata\\agent\\skills\\<skill_name>\\
├── SKILL.md          # Skill definition (required)
├── references\\       # Reference documents (AI reads on demand via Read tool)
└── scripts\\          # Script files (AI executes on demand via PowerShell tool)
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

| Source | macOS / Linux | Windows | Priority |
|--------|---------------|---------|----------|
| User level | \`~/.jdata/agent/skills/\` | \`%USERPROFILE%\\.jdata\\agent\\skills\\\` | Low |
| Project level | \`.jcli/skills/\` | \`.jcli\\skills\\\` | High (overrides user level) |

## Disabling Skills

Disable specific skills via the TUI configuration interface. Settings are saved in the data directory:

| Platform | Config Path |
|----------|-------------|
| macOS / Linux | \`~/.jdata/agent/data/agent_config.json\` |
| Windows | \`%USERPROFILE%\\.jdata\\agent\\data\\agent_config.json\` |

\`\`\`json
{
  "disabled_skills": ["skill-name-1", "skill-name-2"]
}
\`\`\`
`,G=e({default:()=>K}),K=`## Overview

Todo management system with status transitions and TUI interface.

## Basic Usage

### Open TUI Interface

\`\`\`bash
j todo              # Open todo management TUI
\`\`\`

### Command Line Operations

\`\`\`bash
j todo list              # List all todos
j todo list --done       # List completed only
j todo list --undone     # List undone only
j todo add "Finish docs"  # Quick add todo
\`\`\`

## TUI Operations

| Shortcut | Action |
|----------|--------|
| \`j/k\` | Move up/down |
| \`Enter\` | Toggle completion |
| \`a\` | Add new todo |
| \`e\` | Edit current item |
| \`d\` | Delete current item |
| \`r\` | Write to daily report |
| \`Tab\` | Toggle filter (all/undone/done) |
| \`?\` | Show help |
| \`q/Esc\` | Exit |

## Todo Status

| Status | Display |
|--------|---------|
| Undone | \`[ ]\` |
| Done | \`[x]\` |

## Data Storage

Todo data is stored in the data directory:

| Platform | Path |
|----------|------|
| macOS / Linux | \`~/.jdata/report/todo.json\` |
| Windows | \`%USERPROFILE%\\.jdata\\report\\todo.json\` |
`,q=e({default:()=>se}),se=`## File Tools

### Read

Read file contents, supports image formats.

| Parameter | Type | Required | Description |
|------|------|------|------|
| path | string | Yes | File path (absolute or relative) |
| offset | uint | No | Starting line number (0-based, defaults to beginning) |
| limit | uint | No | Number of lines to read (defaults to all) |

**Supported image formats:** PNG, JPG, GIF, WEBP, BMP

\`\`\`json
{"path": "src/main.rs"}
{"path": "screenshot.png"}
{"path": "large.log", "offset": 100, "limit": 50}
\`\`\`

### Write

Write to file, auto-creates parent directories. Overwrites existing files.

| Parameter | Type | Required | Description |
|------|------|------|------|
| path | string | Yes | File path |
| content | string | Yes | Content to write |

\`\`\`json
{"path": "src/new_file.rs", "content": "fn main() {}\\n"}
\`\`\`

### Edit

String replacement editing. \`old_string\` must uniquely match content in the file.

| Parameter | Type | Required | Description |
|------|------|------|------|
| path | string | Yes | File path |
| old_string | string | Yes | Original string to replace (must be unique) |
| new_string | string | No | Replacement string (empty deletes) |
| replace_all | bool | No | Replace all matches (skip uniqueness check) |
| confirm_token | string | No | One-time confirmation token for bulk replacement |

**Bulk replacement flow:** When \`replace_all=true\`, two calls are required: first call returns match preview and \`confirm_token\`, second call carries the token to confirm execution.

\`\`\`json
{"path": "src/main.rs", "old_string": "fn main() {}", "new_string": "fn main() { println!(\\"Hello\\"); }"}
\`\`\`

### Glob

Find files by pattern.

| Parameter | Type | Required | Description |
|------|------|------|------|
| pattern | string | Yes | Glob pattern (e.g., \`**/*.rs\`, \`src/**/*.ts\`) |
| path | string | No | Search directory (defaults to current) |
| excludePattern | string | No | Exclude pattern (e.g., \`**/node_modules/**\`) |
| limit | uint | No | Max results (default 100) |
| offset | uint | No | Skip first N results (for pagination) |

\`\`\`json
{"pattern": "**/*.rs"}
{"pattern": "src/**/*.tsx", "excludePattern": "**/node_modules/**"}
\`\`\`

### Grep

Regex search in file contents.

| Parameter | Type | Required | Description |
|------|------|------|------|
| pattern | string | Yes | Regex pattern |
| path | string | No | Search path (defaults to current) |
| glob | string | No | File filter (e.g., \`*.rs\`) |
| type | string | No | File type (js/py/rust/go/java) |
| output_mode | string | No | Output mode: content/files_with_matches/count |
| context | uint | No | Context lines |
| head_limit | uint | No | Max results |
| ignore_case | bool | No | Case insensitive (default false) |

\`\`\`json
{"pattern": "fn\\\\s+\\\\w+", "type": "rust"}
{"pattern": "TODO", "glob": "*.rs", "output_mode": "count"}
\`\`\`

## Execution Tools

### Bash

Execute shell commands (macOS / Linux only).

| Parameter | Type | Required | Description |
|------|------|------|------|
| command | string | Yes | Shell command (executes in bash -c) |
| description | string | No | Command description (shown in UI) |
| cwd | string | No | Working directory |
| timeout | uint | No | Timeout in seconds (default 120, max 600) |
| run_in_background | bool | No | Run in background (returns task_id) |
| interactive | bool | No | Interactive mode (returns sid, use with Session) |

**Notes:**
- Interactive commands not supported
- Build commands: recommended timeout 300-600
- Background tasks: use \`TaskOutput\` to get results

\`\`\`json
{"command": "cargo build --release", "timeout": 300}
{"command": "npm run dev", "run_in_background": true}
\`\`\`

### PowerShell

Execute PowerShell commands (Windows only).

| Parameter | Type | Required | Description |
|------|------|------|------|
| command | string | Yes | PowerShell command (executes in powershell.exe -NoProfile -Command) |
| description | string | No | Command description |
| cwd | string | No | Working directory |
| timeout | uint | No | Timeout in seconds (default 120, max 600) |
| run_in_background | bool | No | Run in background (returns task_id) |

**Notes:**
- Interactive commands not supported
- Build commands: recommended timeout 300-600
- Background tasks: use \`TaskOutput\` to get results

\`\`\`json
{"command": "cargo build --release", "timeout": 300}
{"command": "npm run dev", "run_in_background": true}
\`\`\`

### TaskOutput

Get output from background task.

| Parameter | Type | Required | Description |
|------|------|------|------|
| task_id | string | Yes | Background task ID |
| block | bool | No | Wait for completion (default true) |
| timeout | uint | No | Wait timeout in ms (default 30000, max 600000) |

## Network Tools

### WebFetch

Fetch web content, auto-converts to Markdown or plain text.

| Parameter | Type | Required | Description |
|------|------|------|------|
| url | string | Yes | Full URL (http:// or https://) |
| extract_mode | string | No | Output format: markdown/text |
| max_chars | uint | No | Max characters (default 50000) |
| headers | object | No | Custom headers |
| authorization | string | No | Authorization header |

\`\`\`json
{"url": "https://docs.rs/serde"}
{"url": "https://api.github.com/repos/rust-lang/rust", "headers": {"Accept": "application/vnd.github.v3+json"}}
\`\`\`

### WebSearch

Search the web using Exa (requires \`EXA_API_KEY\` environment variable).

| Parameter | Type | Required | Description |
|------|------|------|------|
| query | string | Yes | Search keywords |
| count | uint | No | Number of results (1-10, default 5) |
| type | string | No | Search type: auto/keyword/neural |

## Interaction Tools

### Ask

Request structured input from user, supports single/multi-select.

| Parameter | Type | Required | Description |
|------|------|------|------|
| questions | array | Yes | List of questions (1-4) |

**Question structure:**

| Field | Type | Description |
|------|------|------|
| header | string | Short tag |
| question | string | Full question text |
| options | array | Option list (2-4) |
| multi_select | bool | Allow multiple selections |

\`\`\`json
{
  "questions": [{
    "header": "Style",
    "question": "Choose code style",
    "options": ["Concise", "Detailed", "Standard"],
    "multi_select": false
  }]
}
\`\`\`

## Task Tools

### Task

Manage tasks (create/get/list/update).

**Actions:**

| action | Description |
|--------|------|
| create | Create task (requires title) |
| get | Get task details (requires taskId) |
| list | List all tasks |
| update | Update task status |

**Task status flow:** \`pending\` → \`in_progress\` → \`completed\`

\`\`\`json
{"action": "create", "title": "Implement user auth", "description": "Add login/register functionality"}
{"action": "update", "taskId": 1, "status": "in_progress"}
{"action": "list", "ready": true}
\`\`\`

### TodoWrite

Manage todo list.

| Parameter | Type | Description |
|------|------|------|
| todos | array | Todo list |
| merge | bool | Merge update |

**Todo structure:**

| Field | Type | Description |
|------|------|------|
| id | string/int | Todo ID |
| content | string | Content |
| status | string | Status: pending/in_progress/completed |

**Rule:** Only ONE item can be \`in_progress\` at any time

### TodoRead

Read current todo list. Returns id, content, and status for all items.

## Plan Tools

### EnterPlanMode

Enter plan mode. Read-only tools available, write tools blocked.

**Use cases:** Explore codebase and design implementation approach before writing code.

**When to use:**
- New feature with architectural decisions
- Multiple valid approaches need user choice
- Code changes affect existing behavior
- Multi-file changes (more than 2-3 files)

### ExitPlanMode

Exit plan mode, submit plan for user approval.

| Parameter | Type | Description |
|------|------|------|
| allowedPrompts | array | Prompt permissions needed for implementation |

## Extension Tools

### LoadSkill

Load specified skill into context.

| Parameter | Type | Required | Description |
|------|------|------|------|
| name | string | Yes | Skill name |
| arguments | string | No | Arguments to pass to skill |

**Available skills:**

| Skill | Description |
|------|------|
| chat-module-guide | j-cli Chat module architecture quick reference |
| fullstack-team | Multi-agent collaborative full-stack development |
| html-ppt | HTML PPT Studio — professional static HTML presentations |
| hyperframes | Video composition, animation, captions, audio-reactive visuals |
| j-cli | CLI workflow automation (reports, todos, aliases, scripts) |
| jcli-dev-guide | j-cli project developer guide (structure, workflow) |
| skill-creator | Guide for creating skills |
| sql-to-go-struct-and-dao | SQL to Go struct and DAO code generation (gorm-based) |
| swift-ios-app-gen | iOS native app development |
| ui-ux-pro-max | UI/UX design and frontend implementation guidance |
| webapp-gen | One-sentence full web app generation |

### Agent

Launch sub-agent for complex multi-step tasks. Sub-agent uses fresh context, can use all tools except Agent.

| Parameter | Type | Required | Description |
|------|------|------|------|
| prompt | string | Yes | Task description for sub-agent |
| description | string | No | Brief description (3-5 words) |
| run_in_background | bool | No | Run in background |
| worktree | bool | No | Create isolated git worktree |
| inherit_permissions | bool | No | Inherit all tool permissions (no confirmation needed) |

### RegisterHook

Register, list, remove session-level hooks.

**Actions:**

| action | Description |
|--------|------|
| register | Register hook (requires event + command) |
| list | List all hooks |
| remove | Remove hook (requires event + index) |
| help | View protocol documentation |

## Session Process Tools

### Session

Operate on interactive process session's stdin/stdout. Returns sid (session ID) when Bash starts with \`interactive: true\`.

**Actions:**

| action | Description |
|--------|------|
| stdin | Write to process (\`\\n\` for newline) |
| stdout | Read process output (blocks for new output) |
| quit | Terminate session (process receives SIGHUP) |

| Parameter | Type | Required | Description |
|------|------|------|------|
| sid | string | Yes | Session ID (Bash task_id) |
| action | string | Yes | Operation: stdin/stdout/quit |
| text | string | No | Content to write (action=stdin, \`\\n\` for newline) |
| timeout_ms | uint | No | Max wait ms for stdout (default 500) |

## Collaboration Tools

### Teammate

Create independent teammate Agent with separate LLM connection and context. Teammates communicate via \`SendMessage\` tool.

| Parameter | Type | Required | Description |
|------|------|------|------|
| name | string | Yes | Teammate name (e.g., "Frontend", "Backend") |
| prompt | string | Yes | Initial task/instructions |
| role | string | No | Role description (shown in team summary) |
| inherit_permissions | bool | No | Inherit all tool permissions (no confirmation) |
| worktree | bool | No | Create isolated git worktree |

### SendMessage

Send message to teammate Agent. Supports \`@mention\` broadcast.

### IgnoreMessage

Ignore teammate Agent message.

## Tool Loading

### LoadTool

Load deferred tool to make it available in subsequent turns.

| Parameter | Type | Required | Description |
|------|------|------|------|
| name | string | Yes | Tool name to load |

**Currently deferred tools:** RegisterHook, Browser, ComputerUse, Task

## Git Worktree Tools

### EnterWorktree

Create isolated git worktree and switch session into it. Useful when multiple sessions edit the same repository simultaneously.

| Parameter | Type | Required | Description |
|------|------|------|------|
| name | string | No | Worktree name (letters, digits, dots, underscores, dashes only) |

**Location:** \`.jcli/worktrees/{name}\` with branch \`worktree-{name}\`

### ExitWorktree

Exit current git worktree session.

| Parameter | Type | Required | Description |
|------|------|------|------|
| action | string | Yes | "keep" preserves worktree / "remove" deletes worktree and branch |
| discard_changes | bool | No | Required true when action=remove with uncommitted changes |

## Session Tools

### Compact

Compress conversation context to free up context window.

**Use when:**
- Conversation getting long
- Multiple failed attempts at solving a problem

## ComputerUse Tool

macOS desktop control tool supporting screenshots, clicks, typing, etc. (macOS only).

### Screenshot Operations

#### screenshot

Capture screen with SoM (Set-of-Mark) annotations.

| Parameter | Type | Description |
|------|------|------|
| som | bool | Enable SoM annotations (default true) |
| app | string | Target app name |

**SoM annotations:**
- Draws numbered bounding boxes for each interactive element
- Returns element index for click reference
- Supports clicking via \`element\` parameter

### Mouse Operations

#### click

Click at position or element.

| Parameter | Type | Description |
|------|------|------|
| x, y | number | Coordinates (logical points) |
| element | uint | SoM element number |

#### double_click

Double-click at position.

#### right_click

Right-click at position.

#### drag

Drag operation.

| Parameter | Type | Description |
|------|------|------|
| start_x, start_y | number | Start coordinates |
| end_x, end_y | number | End coordinates |
| start_element, end_element | uint | SoM element numbers |
| duration_ms | uint | Drag duration |

#### scroll

Scroll operation.

| Parameter | Type | Description |
|------|------|------|
| dx, dy | int | Scroll amount (negative = up) |

### Keyboard Operations

#### type

Type text.

| Parameter | Type | Description |
|------|------|------|
| text | string | Text to type |
| delay_ms | uint | Keystroke delay in ms |

#### key

Press single key.

| Parameter | Type | Description |
|------|------|------|
| key | string | Key name (e.g., enter, tab, escape) |

#### keys

Key combination.

| Parameter | Type | Description |
|------|------|------|
| keys | array | Key list (e.g., \`["cmd", "c"]\`) |

**Supported keys:**
- Letters: a-z
- Numbers: 0-9
- Special: enter, tab, space, escape, delete, backspace
- Arrows: up, down, left, right
- Function: f1-f12, home, end, pageup, pagedown
- Modifiers: cmd, shift, alt/option, ctrl

### Helper Operations

#### find_element

Find element.

| Parameter | Type | Description |
|------|------|------|
| query | string | Search query |

#### ax_tree

Query accessibility tree.

| Parameter | Type | Description |
|------|------|------|
| app | string | Target app |
| depth | uint | Tree depth limit |
| role | string | Role filter (e.g., AXButton) |
| clickable | bool | Show only clickable elements |

## Permission Configuration

Permissions are configured in \`.jcli/permissions.yaml\`, supporting three rule types:

\`\`\`yaml
# .jcli/permissions.yaml
permissions:
  # Allow all (skip all tool confirmations)
  allow_all: false
  
  # Allow list (skip confirmation if matched, supports regex)
  allow:
    - Read
    - Grep
    - Glob
    - "Bash:ls.*"       # Regex match on command parameter
    - "Bash:git status"
  
  # Deny list (takes priority over allow, direct reject if matched)
  deny:
    - "Bash:rm -rf.*"   # Block dangerous commands
    - "Bash:.*sudo.*"   # Block sudo commands
\`\`\`

### Rule Matching

- **Simple match**: Tool name (e.g., \`Read\`, \`Bash\`)
- **Regex match**: \`ToolName:regex_pattern\` (e.g., \`Bash:rm.*\` matches Bash tool's command parameter)
- **Priority**: deny > allow > default confirmation

## Context References

| Reference | Description |
|------|------|
| \`@file:path\` | Include file content (auto-read and inject into context) |
| \`@skill:name\` | Load and activate specified skill |
`,ce=e({default:()=>le}),le=`## 概述

AI 对话系统，支持多模型、上下文引用和 Agent 自主执行。

## 启动对话

\`\`\`bash
j chat              # 进入 TUI 对话界面
j chat "你好"       # 快速提问并打印回复
j chat -c           # 延续上一个会话
j chat --session <id>  # 恢复指定会话
\`\`\`

## 快捷键

| 快捷键 | 功能 |
|--------|------|
| \`Enter\` | 发送消息 |
| \`Esc\` | 取消响应/退出 |
| \`Ctrl+Y\` | 复制最后一条 AI 回复 |
| \`Ctrl+B\` | 消息浏览模式 |
| \`Ctrl+G\` | 打开日志窗口 |
| \`Ctrl+O\` | 展开/折叠工具详情 |
| \`Ctrl+E\` | 打开配置界面 |
| \`F1\` 或 \`?\` | 显示帮助 |

## 斜杠命令

在输入框中输入 \`/\` 触发斜杠命令：

| 命令 | 功能 |
|------|------|
| \`/copy\` | 复制最后一条 AI 回复 |
| \`/log\` | 打开日志窗口 |
| \`/browse\` | 浏览历史消息 |
| \`/config\` | 打开配置界面 |
| \`/model\` | 切换模型 |
| \`/archive\` | 归档当前对话 |

## 上下文引用

输入框中以 \`@\` 触发补全：

\`\`\`
@skill:<name>       # 引用技能
@command:<name>     # 引用自定义命令
@file:<path>        # 引用文件内容（支持图片）
\`\`\`

## Agent 能力

AI 对话内置 Agent 能力，可自主规划并执行多步骤任务：

- **自主推理**：AI 规划并执行多步任务
- **工具集成**：自动使用可用工具（Read、Write、Bash/PowerShell 等）
- **任务管理**：Task 和 Todo 工具管理复杂任务
- **计划模式**：先探索代码库再制定计划

### 可用工具

| 工具 | macOS / Linux | Windows | 描述 |
|------|:---:|:---:|------|
| Read | ✅ | ✅ | 读取文件 |
| Write | ✅ | ✅ | 写入文件 |
| Edit | ✅ | ✅ | 编辑文件 |
| Glob | ✅ | ✅ | 文件名搜索 |
| Grep | ✅ | ✅ | 内容搜索 |
| Bash | ✅ | ❌ | Shell 命令执行 |
| PowerShell | ❌ | ✅ | PowerShell 命令执行 |
| WebFetch | ✅ | ✅ | 网页抓取 |
| WebSearch | ✅ | ✅ | 网页搜索 |
| ComputerUse | ✅ | ❌ | 屏幕截图与操作 |
| Browser | ✅ | ✅ | 浏览器自动化 |

### 计划模式

对于复杂任务，可先进入计划模式探索代码库：

\`\`\`
分析这个项目的架构并设计重构方案

# AI 会：
1. 进入计划模式（只读工具可用）
2. 探索代码库结构
3. 生成详细计划
4. 提交计划等待用户确认
\`\`\`

### 工具权限配置

在项目根目录创建 \`.jcli/permissions.yaml\`：

\`\`\`yaml
permissions:
  allow_all: false
  allow:
    - Read
    - Grep
    - Glob
  deny:
    - Bash          # macOS / Linux
    - PowerShell    # Windows
    - Write
\`\`\`

## 远程控制

\`\`\`bash
j chat --remote     # 启用远程控制（手机扫码）
j chat --remote --port 9390  # 指定端口
\`\`\`
`,ue=e({default:()=>de}),de=`## 概述

别名系统，为路径和网址创建简短别名以便快速访问。

> **提示**：输入路径时按 **Tab** 键可自动补全，支持 \`~\` 展开。如 \`j set work ~/Pro<Tab>\`。

## 基本用法

### 添加别名

\`\`\`bash
j set <alias> <path>    # 添加路径别名
j set <alias> <url>     # 添加网址别名
\`\`\`

### 执行别名

\`\`\`bash
j <alias>               # 打开路径或网址
\`\`\`

### 管理别名

\`\`\`bash
j rm <alias>            # 删除别名
j rename <old> <new>    # 重命名别名
j mf <alias> <new_path> # 修改别名指向
\`\`\`

## 别名类型

### 路径别名

\`\`\`bash
# 添加路径
j set work ~/Projects/work
j set notes ~/Documents/notes

# 打开路径
j work    # 在文件管理器中打开
j notes   # 在文件管理器中打开
\`\`\`

### 网址别名

\`\`\`bash
# 添加网址
j set gh https://github.com
j set gh-issues https://github.com/issues

# 打开网址
j gh        # 在浏览器中打开
j gh-issues # 在浏览器中打开
\`\`\`

## 别名存储

别名单独存放在数据目录下的配置文件中：

| 平台 | 配置文件路径 |
|------|-------------|
| macOS / Linux | \`~/.jdata/alias.yaml\` |
| Windows | \`%USERPROFILE%\\.jdata\\alias.yaml\` |

\`\`\`yaml
# macOS / Linux
path:
  chrome: "/Applications/Google Chrome.app"
  vscode: "/Applications/Visual Studio Code.app"
  work: /Users/user/Projects/work

inner_url:
  gh: https://github.com
  gh-issues: https://github.com/issues

# Windows
path:
  notepad: "C:\\\\Windows\\\\notepad.exe"
  vscode: "C:\\\\Users\\\\user\\\\AppData\\\\Local\\\\Programs\\\\Microsoft VS Code\\\\Code.exe"
  work: "C:\\\\Users\\\\user\\\\Projects\\\\work"

inner_url:
  gh: https://github.com
\`\`\`
`,fe=e({default:()=>pe}),pe=`## 概述

Browser 是 AI 对话中的工具，支持网页浏览、交互和内容提取。

## 平台支持

| 平台 | Lite 模式 | CDP 模式 |
|------|:---:|:---:|
| macOS | ✅ | ✅ |
| Linux | ✅ | ✅ |
| Windows | ✅ | ✅ |

> **注意**：Windows 上 CDP 模式需要 Chrome 或 Edge 浏览器已安装。

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
`,me=e({default:()=>he}),he=`## 概述

Commands 是可复用的提示词片段，帮助快速调用预设的提示词。

## 斜杠命令

在输入框中输入 \`/\` 触发斜杠命令：

| 命令 | 描述 |
|------|------|
| \`/copy\` | 复制最后一条 AI 回复 |
| \`/log\` | 打开日志窗口 |
| \`/browse\` | 浏览历史消息 |
| \`/config\` | 打开配置界面 |
| \`/model\` | 切换模型 |
| \`/archive\` | 归档当前对话 |
| \`/clear\` | 新建对话 |
| \`/theme\` | 切换主题 |
| \`/resume\` | 恢复历史会话 |
| \`/dump\` | 导出原始会话消息 |
| \`/dump-processed\` | 导出处理后的会话数据 |
| \`/teammate\` | Teammate 面板 |

## 自定义命令

### 使用方式

在输入框中以 \`@command:<名称>\` 引用：

\`\`\`
@command:review 请审查这段代码
\`\`\`

### 创建命令

#### 目录位置

| 级别 | macOS / Linux | Windows |
|------|---------------|---------|
| 用户级 | \`~/.jdata/agent/commands/\` | \`%USERPROFILE%\\.jdata\\agent\\commands\\\` |
| 项目级 | \`.jcli/commands/\` | \`.jcli\\commands\\\` |

项目级命令优先级更高。

#### 文件格式

每个命令是一个 Markdown 文件，包含 YAML frontmatter 和提示词正文：

\`\`\`markdown
---
name: review
description: 代码审查提示词
---
请对以下代码进行全面审查，关注：
- 代码质量
- 潜在问题
- 改进建议
\`\`\`

#### 两种组织方式

**方式一：单文件制**

直接在 commands 目录下创建 \`.md\` 文件：

\`\`\`
commands/
  review.md
  test.md
\`\`\`

**方式二：目录制**

创建目录并在其中放置 \`COMMAND.md\`（适合复杂的命令，可附带资源文件）：

\`\`\`
commands/
  review/
    COMMAND.md
    checklist.txt
\`\`\`

### 示例

创建一个 \`plan.md\`：

\`\`\`markdown
---
name: plan
description: 进入 PLAN 模式
---
请进入 plan 模式规划任务
\`\`\`

使用：

\`\`\`
@command:plan
\`\`\`

### 管理命令

在 TUI 中按 \`Ctrl+E\` 打开配置界面，切换到 Commands 标签页，可以启用或禁用命令。
`,ge=e({default:()=>_e}),_e=`## 概述

j-cli 的所有数据存储在统一的用户数据目录中，支持通过环境变量自定义路径。

## 数据目录路径

| 平台 | 默认路径 | 环境变量覆盖 |
|------|---------|-------------|
| macOS / Linux | \`~/.jdata/\` | \`J_DATA_PATH=/custom/path\` |
| Windows | \`%USERPROFILE%\\.jdata\\\` | \`$env:J_DATA_PATH="C:\\custom\\path"\` |

### 环境变量覆盖

\`\`\`bash
# macOS / Linux
export J_DATA_PATH=/custom/path
j chat  # 数据将存储在 /custom/path/

# Windows (PowerShell)
$env:J_DATA_PATH="C:\\custom\\path"
j chat  # 数据将存储在 C:\\custom\\path\\
\`\`\`

## 目录结构

\`\`\`
~/.jdata/                          # macOS / Linux
%USERPROFILE%\\.jdata\\              # Windows
├── config.yaml                    # 主配置文件
├── alias.yaml                     # 别名定义
├── report/                        # 日报/周报
│   ├── report.md                  # 日报文件
│   ├── todo.json                  # 待办数据
│   └── settings.json              # 周报元数据
├── scripts/                       # 用户脚本
│   ├── deploy.sh                  # macOS / Linux
│   └── deploy.cmd                 # Windows
├── agent/                         # AI Agent 数据
│   ├── data/                      # Agent 运行时数据
│   │   ├── messages/              # 对话历史
│   │   └── agent_config.json      # Agent 配置
│   └── skills/                    # 用户自定义 Skill
│       └── <skill_name>/
│           └── SKILL.md
└── hooks/                         # Hook 脚本
    └── pre_chat/
        └── my_hook.sh             # macOS / Linux
        └── my_hook.cmd            # Windows
\`\`\`

## 配置文件说明

### config.yaml

主配置文件，存储全局设置：

\`\`\`yaml
# API 配置
api_key: "your-api-key"
base_url: "https://api.openai.com/v1"
model: "gpt-4"

# 报表配置
report_file_path: "~/.jdata/report/report.md"

# 浏览器配置
settings:
  browser_headless: true
\`\`\`

### alias.yaml

别名定义，存储应用和 URL 别名：

\`\`\`yaml
# macOS / Linux
chrome:
  path: "/Applications/Google Chrome.app"
  note: "browser"

# Windows
notepad:
  path: "C:\\\\Windows\\\\notepad.exe"
  note: "editor"

# URL 别名
github:
  path: "https://github.com"
  type: "inner_url"
\`\`\`

## 数据迁移

### 备份

\`\`\`bash
# macOS / Linux
cp -r ~/.jdata ~/.jdata.backup

# Windows
Copy-Item "$env:USERPROFILE\\.jdata" "$env:USERPROFILE\\.jdata.backup" -Recurse
\`\`\`

### 恢复

\`\`\`bash
# macOS / Linux
cp -r ~/.jdata.backup ~/.jdata

# Windows
Copy-Item "$env:USERPROFILE\\.jdata.backup" "$env:USERPROFILE\\.jdata" -Recurse
\`\`\`

### 跨平台迁移

数据目录结构在所有平台上一致，可以直接复制迁移：

1. 备份源平台数据目录
2. 复制到目标平台对应位置
3. 调整脚本文件扩展名（\`.sh\` → \`.cmd\`）
4. 更新 alias.yaml 中的路径格式
`,ve=e({default:()=>ye}),ye=`## 概述

Hook 系统允许在 AI 对话生命周期的特定节点自动执行自定义脚本。

## Hook 类型

| Hook | 触发时机 | 用途 |
|------|---------|------|
| \`pre_chat\` | 对话开始前 | 注入系统提示、设置环境 |
| \`post_chat\` | 对话结束后 | 清理、通知 |
| \`pre_tool\` | 工具执行前 | 安全检查、参数修改 |
| \`post_tool\` | 工具执行后 | 日志、结果处理 |
| \`on_error\` | 发生错误时 | 错误通知、降级处理 |

## Hook 配置

Hook 在 \`.jcli/config.yaml\` 中定义：

\`\`\`yaml
hooks:
  pre_chat:
    - name: load_project_context
      command: "cat AGENTS.md 2>/dev/null || echo 'No AGENTS.md found'"
      timeout: 5
    - name: check_env
      command: "echo 'Environment ready'"
      timeout: 3
  post_tool:
    - name: log_tool_usage
      command: "echo 'Tool executed at $(date)' >> ~/.jdata/tool.log"
      timeout: 5
\`\`\`

## 执行方式

### macOS / Linux

Hook 命令通过 \`sh -c\` 执行：

\`\`\`bash
sh -c "<hook_command>"
\`\`\`

支持所有标准 shell 命令，包括管道、重定向、命令替换等。

### Windows

Hook 命令通过 \`cmd /c\` 执行：

\`\`\`cmd
cmd /c "<hook_command>"
\`\`\`

支持 cmd 内置命令和外部程序，管道 \`|\`、重定向 \`>\` 等语法可用。

> **注意**：Windows hook 不支持 bash 语法（如 \`$(date)\`、\`2>/dev/null\`），需使用对应的 cmd 语法（如 \`%DATE%\`、\`2>NUL\`）。

## 超时处理

- 默认超时 10 秒
- 超时后自动终止进程：
  - macOS / Linux：发送 SIGKILL 信号
  - Windows：使用 \`taskkill /F /T /PID\` 终止
- 超时不会阻断对话流程，仅跳过当前 hook

## 环境变量

Hook 执行时可访问以下环境变量：

| 变量 | 描述 |
|------|------|
| \`J_DATA_PATH\` | 数据目录路径 |
| \`J_<ALIAS>\` | 别名对应的应用路径（大写，\`-\` 转 \`_\`） |

### macOS / Linux 示例

\`\`\`yaml
hooks:
  pre_chat:
    - name: open_project
      command: "cat $J_DATA_PATH/agent/data/context.txt"
      timeout: 5
\`\`\`

### Windows 示例

\`\`\`yaml
hooks:
  pre_chat:
    - name: open_project
      command: "type %J_DATA_PATH%\\agent\\data\\context.txt"
      timeout: 5
\`\`\`

## PATH 注入

Hook 目录会自动注入到 PATH 环境变量中：
- macOS / Linux：使用 \`:\` 分隔符
- Windows：使用 \`;\` 分隔符

## Hook 脚本文件

除了内联命令，也可以创建独立的脚本文件：

### macOS / Linux

\`\`\`bash
# ~/.jdata/hooks/pre_chat/my_hook.sh
#!/bin/bash
echo "Loading project context..."
cat AGENTS.md 2>/dev/null
\`\`\`

### Windows

\`\`\`cmd
@echo off
REM %USERPROFILE%\\.jdata\\hooks\\pre_chat\\my_hook.cmd
echo Loading project context...
type AGENTS.md 2>NUL
\`\`\`

## 注意事项

- Hook 执行失败不会阻断对话
- 命令输出会作为系统消息注入到对话上下文中
- 避免在 hook 中执行耗时操作
- 敏感信息不要通过 hook 输出
`,be=e({default:()=>xe}),xe=`## 一键安装（推荐）

### macOS / Linux

\`\`\`bash
# 安装最新版本
curl -fsSL https://raw.githubusercontent.com/LingoJack/jcli/main/install.sh | sh

# 安装指定版本
curl -fsSL https://raw.githubusercontent.com/LingoJack/jcli/main/install.sh | sh -s -- v1.0.0
\`\`\`

### Windows

\`\`\`powershell
# 安装最新版本
irm https://raw.githubusercontent.com/LingoJack/jcli/main/install.ps1 | iex

# 安装指定版本
$v="v1.0.0"; irm https://raw.githubusercontent.com/LingoJack/jcli/main/install.ps1 | iex
\`\`\`

> Windows 安装位置: \`%LOCALAPPDATA%\\j-cli\\j.exe\`，自动添加到用户 PATH

## 从 crates.io 安装

\`\`\`bash
# 标准版（Lite 浏览器模式，无额外依赖）
cargo install j-cli

# 完整版（CDP 浏览器模式，需要 Chrome/Chromium）
cargo install j-cli --features browser_cdp
\`\`\`

## 从源码构建

\`\`\`bash
git clone https://github.com/LingoJack/jcli.git
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

### macOS / Linux

\`\`\`bash
# 使用安装脚本（推荐）
curl -fsSL https://raw.githubusercontent.com/LingoJack/jcli/main/install.sh | sh -s -- --uninstall

# 或通过 cargo
cargo uninstall j-cli

# 或手动删除
sudo rm /usr/local/bin/j  # 一键安装
rm ~/.cargo/bin/j          # Cargo 安装

# （可选）删除数据目录
rm -rf ~/.jdata
\`\`\`

### Windows

\`\`\`powershell
# 使用安装脚本（推荐）
powershell -ExecutionPolicy Bypass -File install.ps1 -Uninstall

# 或手动删除
Remove-Item "$env:LOCALAPPDATA\\j-cli" -Recurse -Force

# （可选）删除数据目录
Remove-Item "$env:USERPROFILE\\.jdata" -Recurse -Force
\`\`\`

## 平台差异说明

| 功能 | macOS / Linux | Windows |
|------|---------------|---------|
| AI Shell 工具 | Bash | PowerShell |
| 默认脚本 | \`.sh\` + bash shebang | \`.cmd\` |
| 自动更新 | \`.tar.gz\` 解压 | \`.zip\` 解压 |
| 数据目录 | \`~/.jdata/\` | \`%USERPROFILE%\\.jdata\\\` |
| 安装位置 | \`/usr/local/bin/j\` | \`%LOCALAPPDATA%\\j-cli\\j.exe\` |
| Computer Use | 支持 | 不支持 |
| j-indicator | 菜单栏指示灯 | 不支持 |
`,Se=e({default:()=>Ce}),Ce=`## 概述

文件加密与解密功能，使用 **AES-256-GCM** 对文件进行对称加密，支持单文件或目录批量操作。

## 基本用法

### 加密文件

\`\`\`bash
j lock <密码> <文件或目录路径>
\`\`\`

### 解密文件

\`\`\`bash
j unlock <密码> <文件或目录路径>
\`\`\`

## 参数说明

| 参数 | 说明 |
|------|------|
| 密码 | 用于派生 AES-256 密钥，不会存储到磁盘 |
| 文件或目录路径 | 目标文件或目录；目录时会递归处理所有文件 |

## 使用示例

### 加密单个文件

\`\`\`bash
j lock mypassword secret.txt
# 输出: secret.txt.lock（原文件被删除）
\`\`\`

### 解密单个文件

\`\`\`bash
j unlock mypassword secret.txt.lock
# 输出: secret.txt（.lock 文件被删除）
\`\`\`

### 批量加密目录

\`\`\`bash
j lock mypassword ./documents/
# 递归加密目录下所有文件，每个文件生成对应的 .lock 文件
\`\`\`

### 批量解密目录

\`\`\`bash
j unlock mypassword ./documents/
# 递归解密目录下所有 .lock 文件
\`\`\`

## 加密原理

| 项目 | 说明 |
|------|------|
| 算法 | AES-256-GCM（认证加密，防篡改） |
| 密钥派生 | HKDF-SHA256，从密码 + 随机 Salt 派生 256-bit 密钥 |
| 随机性 | 每次加密使用独立的随机 Salt（32 字节）和 Nonce（12 字节） |

### 加密文件格式

\`\`\`
MAGIC(4) + VERSION(1) + SALT(32) + NONCE(12) + CIPHERTEXT+TAG(16)
\`\`\`

| 字段 | 长度 | 说明 |
|------|------|------|
| MAGIC | 4 字节 | \`JLCK\`，标识 j-cli 加密文件 |
| VERSION | 1 字节 | 格式版本号（当前为 \`0x01\`） |
| SALT | 32 字节 | HKDF 盐值 |
| NONCE | 12 字节 | AES-GCM 随机 nonce |
| CIPHERTEXT+TAG | 变长 + 16 字节 | 加密数据 + GCM 认证标签 |

## 行为说明

| 行为 | 说明 |
|------|------|
| 加密后 | **删除原文件**，只保留 \`.lock\` 加密文件 |
| 解密后 | **删除 .lock 文件**，只保留还原后的原文件 |
| 跳过规则 | 加密时跳过已有 \`.lock\` 后缀的文件 |
| 隐藏文件 | 跳过以 \`.\` 开头的隐藏文件和隐藏目录 |
| 符号链接 | 跳过符号链接 |

## 注意事项

- 请务必**记住密码**，密码丢失无法恢复文件内容
- 密码不存储在系统中，每次加密/解密需手动输入
- 加密后原文件被删除，请确认加密成功后再清理`,we=e({default:()=>Te}),Te=`## 概述

内置 Markdown 编辑器是一个类 Typora 的终端编辑器，支持行级渲染切换和完整 Vim 模式。

核心特性：
- **行级渲染**：当前编辑行显示源码，其他行显示渲染效果
- **Vim 模式**：Normal、Insert、Visual、Command、Search 完整支持
- **实时预览**：标题、代码块、表格、列表等即时渲染

## Vim 模式

### 模式切换

| 模式 | 边框颜色 | 说明 |
|------|----------|------|
| Normal | 深灰 | 默认浏览模式 |
| Insert | 青色 | 文本编辑模式 |
| Visual | 黄色 | 可视选择模式 |
| Command | 深灰 | 命令模式（\`:\`） |
| Search | 紫色 | 搜索模式（\`/\`） |

### Normal 模式快捷键

| 快捷键 | 功能 |
|--------|------|
| \`h/j/k/l\` | 左/下/上/右移动 |
| \`w/b/e\` | 按词移动 |
| \`0/$\` | 行首/行尾 |
| \`g/G\` | 文件首/尾 |
| \`i/a/A/I\` | 进入 Insert 模式 |
| \`o/O\` | 在下方/上方插入新行 |
| \`x/X\` | 删除字符 |
| \`dd\` | 删除整行 |
| \`dw/d$\` | 删除词/删除到行尾 |
| \`cc\` | 修改整行 |
| \`cw/c$\` | 修改词/修改到行尾 |
| \`yy\` | 复制行 |
| \`p\` | 粘贴 |
| \`u\` | 撤销 |
| \`Ctrl+R\` | 重做 |
| \`v\` | 进入 Visual 模式 |
| \`:\` | 进入 Command 模式 |
| \`/\` | 进入 Search 模式 |
| \`n/N\` | 下一个/上一个搜索结果 |

### Insert 模式

| 快捷键 | 功能 |
|--------|------|
| \`Esc\` | 返回 Normal 模式 |
| 其他 | 正常文本输入 |

### Visual 模式

| 快捷键 | 功能 |
|--------|------|
| \`h/j/k/l\` | 扩展选择 |
| \`y\` | 复制选中内容 |
| \`Esc\` | 返回 Normal 模式 |

### Command 模式

| 命令 | 功能 |
|------|------|
| \`:w\` | 保存并提交 |
| \`:wq\` | 保存并提交 |
| \`:x\` | 保存并提交 |
| \`:q\` | 取消编辑 |
| \`:q!\` | 取消编辑 |

### Search 模式

- 输入搜索词后按 \`Enter\` 开始搜索
- \`n\` 跳转到下一个匹配
- \`N\` 跳转到上一个匹配

## 全局快捷键

| 快捷键 | 功能 |
|--------|------|
| \`Ctrl+S\` | 保存并提交 |
| \`Ctrl+Q\` | 取消编辑 |
| \`PageUp/PageDown\` | 翻页 |

## Markdown 渲染

### 标题

\`\`\`
# 一级标题      →  ◆ 一级标题
## 二级标题     →  ◇ 二级标题
### 三级标题    →  〈 三级标题
#### 四级标题   →  › 四级标题
\`\`\`

### 代码块

代码块会渲染为带边框的样式：

\`\`\`
┌─ rust ────────────┐
│ let x = 42;       │
│ println!("{}", x);│
└───────────────────┘
\`\`\`

支持语法高亮，语言标识从围栏行提取（如 \` \`\`\`rust\`）。

### 表格

自动对齐列宽，渲染为美观的表格格式：

\`\`\`
│ Header1 │ Header2 │
├────────┼────────┤
│ cell1  │ cell2  │
\`\`\`

### 其他元素

| 语法 | 渲染效果 |
|------|----------|
| \`**粗体**\` | **粗体** |
| \`*斜体*\` | *斜体* |
| \`~~删除线~~\` | ~~删除线~~ |
| \`\` \`代码\` \`\` | \`代码\` |
| \`- 列表项\` | • 列表项 |
| \`- [ ] 任务\` | ○ 任务 |
| \`- [x] 完成\` | ● 完成 |
| \`> 引用\` | │ 引用 |
| \`[链接](url)\` | 链接 ↗ |
| \`![图片](url)\` | 🖼 alt |

## 使用场景

- 编写日报、周报
- 编辑 Markdown 文档
- 快速记录笔记
- 代码片段编辑

编辑器会在保存时返回编辑后的内容，取消时返回空。
`,Ee=e({default:()=>De}),De=`## 权限配置文件

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

### 平台差异

| 工具 | macOS / Linux | Windows |
|------|---------------|---------|
| Shell | \`Bash(...)\` | \`PowerShell(...)\` |
| Computer Use | \`ComputerUse(...)\` | 不可用 |

> **注意**：Windows 上使用 \`PowerShell(...)\` 规则匹配，而非 \`Bash(...)\`。

### Bash / PowerShell 命令匹配

macOS / Linux:

\`\`\`yaml
allow:
  - "Bash(cargo:*)"        # cargo build, cargo test 等
  - "Bash(git status:*)"   # git status
  - "Bash(ls:*)"           # ls, ls -la 等

deny:
  - "Bash(rm -rf:*)"       # 阻止 rm -rf
  - "Bash(/.*sudo.*/)"     # 阻止所有 sudo 命令
\`\`\`

Windows:

\`\`\`yaml
allow:
  - "PowerShell(cargo:*)"        # cargo build, cargo test 等
  - "PowerShell(git status:*)"   # git status
  - "PowerShell(dir:*)"          # dir, dir /s 等

deny:
  - "PowerShell(Remove-Item -Recurse -Force:*)"  # 阻止递归强制删除
  - "PowerShell(/.*Format-.*/)"                   # 阻止格式化命令
\`\`\`

### 文件路径匹配（Write/Edit/Read）

\`\`\`yaml
# macOS / Linux
allow:
  - "Write(path:/src/*)"   # 允许写入 /src 目录
  - "Edit(path:/lib/*)"    # 允许编辑 /lib 目录

deny:
  - "Write(path:/etc/*)"   # 阻止写入 /etc

# Windows
allow:
  - "Write(path:C:\\\\Projects\\\\myapp\\\\src\\\\*)"  # 允许写入项目 src 目录
  - "Edit(path:C:\\\\Projects\\\\myapp\\\\lib\\\\*)"   # 允许编辑项目 lib 目录

deny:
  - "Write(path:C:\\\\Windows\\\\System32\\\\*)"     # 阻止写入系统目录
\`\`\`

### URL 域名匹配（WebFetch）

\`\`\`yaml
allow:
  - "WebFetch(domain:docs.rs)"
  - "WebFetch(domain:github.com)"
  - "WebFetch(domain:/.*\\\\.google\\\\.com$/)"  # 正则匹配所有 google 子域名
\`\`\`
`,Oe=e({default:()=>ke}),ke=`## 注册应用别名

### macOS / Linux

\`\`\`bash
j set chrome "/Applications/Google Chrome.app"
j set vscode "/Applications/Visual Studio Code.app"

# 注册 URL 别名（自动识别为 inner_url）
j set github https://github.com
\`\`\`

### Windows

\`\`\`powershell
j set notepad "C:\\Windows\\notepad.exe"
j set vscode "C:\\Users\\%USERNAME%\\AppData\\Local\\Programs\\Microsoft VS Code\\Code.exe"

# 注册 URL 别名
j set github https://github.com
\`\`\`

## 标记分类

\`\`\`bash
j note chrome browser
j note vscode editor
\`\`\`

## 打开应用

### macOS / Linux

\`\`\`bash
j chrome                  # 打开 Chrome
j chrome github           # 用 Chrome 打开 github URL
j chrome "rust lang"      # 用 Chrome 搜索 "rust lang"
j vscode ./src            # 用 VSCode 打开 src 目录
\`\`\`

### Windows

\`\`\`powershell
j notepad                 # 打开记事本
j vscode .\\src            # 用 VSCode 打开 src 目录
j github                  # 打开 github URL
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
\`\`\``,Ae=e({default:()=>je}),je=`## 概述

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
`,Me=e({default:()=>Ne}),Ne=`## 概述

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

| 文件 | macOS / Linux | Windows |
|------|---------------|---------|
| 主配置 | \`~/.jdata/config.yaml\` | \`%USERPROFILE%\\.jdata\\config.yaml\` |
| 周报元数据 | \`<report_dir>/settings.json\` | \`<report_dir>\\settings.json\` |

| 文件 | 描述 |
|------|------|
| 主配置 | report_file_path、git_repo 等设置 |
| 周报元数据 | week_num、last_day 等周报信息 |

## 自动周切换

当当前日期超过 \`last_day\` 时，写入日报会自动：
1. 生成新周标题 \`# WeekN[开始日期 - 结束日期]\`
2. 更新 week_num 和 last_day
`,Pe=e({default:()=>Fe}),Fe=`## 概述

脚本系统，通过 \`concat\` 命令创建和管理可执行脚本。

## 基本用法

### 创建脚本

\`\`\`bash
j concat <name>              # 打开 TUI 编辑器编写脚本
j concat <name> "<content>"  # 直接创建脚本
\`\`\`

### 编辑脚本

\`\`\`bash
j concat <name>              # 如果脚本已存在，进入编辑模式
\`\`\`

### 运行脚本

\`\`\`bash
j <name>           # 直接通过别名运行
j <name> <args...> # 带参数运行
\`\`\`

### 删除脚本

\`\`\`bash
j rm <name>        # 删除别名（同时删除脚本文件）
\`\`\`

## 脚本存储

脚本按平台存储在不同目录：

| 平台 | 存储路径 |
|------|---------|
| macOS / Linux | \`~/.jdata/scripts/\` |
| Windows | \`%USERPROFILE%\\.jdata\\scripts\\\` |

\`\`\`
# macOS / Linux
~/.jdata/scripts/
├── deploy.sh
├── build.sh
└── test.sh

# Windows
%USERPROFILE%\\.jdata\\scripts\\
├── deploy.cmd
├── build.cmd
└── test.cmd
\`\`\`

脚本创建后自动注册为别名，可直接通过 \`j <name>\` 执行。

## 平台差异

| 功能 | macOS / Linux | Windows |
|------|---------------|---------|
| 脚本扩展名 | \`.sh\` | \`.cmd\` |
| 默认 shebang | \`#!/bin/bash\` | 无 |
| 执行 shell | Bash | cmd.exe |
| PATH 分隔符 | \`:\` | \`;\` |

## 示例

### macOS / Linux

\`\`\`bash
# 创建部署脚本
j concat deploy

# 在编辑器中输入：
#!/bin/bash
set -e
npm run build
rsync -avz dist/ user@server:/var/www/

# 运行脚本
j deploy
\`\`\`

### Windows

\`\`\`powershell
# 创建部署脚本
j concat deploy

# 在编辑器中输入：
@echo off
npm run build
xcopy dist\\ \\\\server\\www\\ /E /I /Y

# 运行脚本
j deploy
\`\`\`

## 环境变量注入

执行脚本时，所有已注册的别名路径会自动注入为环境变量，命名规则为 \`J_<别名大写>\`（\`-\` 转为 \`_\`）。

### macOS / Linux（.sh 脚本）

\`\`\`bash
#!/bin/bash
# 已注册: chrome -> /Applications/Google Chrome.app
# 已注册: my-tool -> /usr/local/bin/my-tool

open -a "$J_CHROME" https://example.com
"$J_MY_TOOL" --version
\`\`\`

### Windows（.cmd 脚本）

\`\`\`cmd
@echo off
REM 已注册: notepad -> C:\\Windows\\notepad.exe
REM 已注册: vscode -> C:\\Users\\%USERNAME%\\AppData\\Local\\Programs\\Microsoft VS Code\\Code.exe

start "" "%J_VSCODE%" .\\src
"%J_NOTEPAD%" readme.txt
\`\`\`

> 路径含空格时，脚本中必须用双引号包裹变量：\`"$J_CHROME"\` / \`"%J_VSCODE%"\`
`,Ie=e({default:()=>Le}),Le=`## 概述

Skill 是扩展 AI 能力的专用提示词模块，通过 \`LoadSkill\` 工具加载。

## Skill 结构

\`\`\`
# macOS / Linux
~/.jdata/agent/skills/<skill_name>/
├── SKILL.md          # Skill 定义（必需）
├── references/       # 参考文档（AI 按需 Read 读取）
└── scripts/          # 脚本文件（AI 按需 Bash/PowerShell 执行）

# Windows
%USERPROFILE%\\.jdata\\agent\\skills\\<skill_name>\\
├── SKILL.md          # Skill 定义（必需）
├── references\\       # 参考文档（AI 按需 Read 读取）
└── scripts\\          # 脚本文件（AI 按需 PowerShell 执行）
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

| 来源 | macOS / Linux | Windows | 优先级 |
|------|---------------|---------|--------|
| 用户级 | \`~/.jdata/agent/skills/\` | \`%USERPROFILE%\\.jdata\\agent\\skills\\\` | 低 |
| 项目级 | \`.jcli/skills/\` | \`.jcli\\skills\\\` | 高（覆盖用户级） |

## 禁用 Skill

在 TUI 配置界面中禁用特定 skill，配置保存在数据目录下：

| 平台 | 配置路径 |
|------|---------|
| macOS / Linux | \`~/.jdata/agent/data/agent_config.json\` |
| Windows | \`%USERPROFILE%\\.jdata\\agent\\data\\agent_config.json\` |

\`\`\`json
{
  "disabled_skills": ["skill-name-1", "skill-name-2"]
}
\`\`\`
`,Re=e({default:()=>ze}),ze=`## 概述

待办管理系统，支持任务状态流转和 TUI 交互界面。

## 基本用法

### 进入 TUI 界面

\`\`\`bash
j todo              # 打开待办管理 TUI
\`\`\`

### 命令行操作

\`\`\`bash
j todo list              # 列出所有待办
j todo list --done       # 只列出已完成
j todo list --undone     # 只列出未完成
j todo add "完成文档"     # 快速添加待办
\`\`\`

## TUI 操作

| 快捷键 | 功能 |
|--------|------|
| \`j/k\` | 上下移动 |
| \`Enter\` | 切换完成状态 |
| \`a\` | 添加新待办 |
| \`e\` | 编辑当前项 |
| \`d\` | 删除当前项 |
| \`r\` | 写入日报 |
| \`Tab\` | 切换筛选（全部/未完成/已完成） |
| \`?\` | 显示帮助 |
| \`q/Esc\` | 退出 |

## 待办状态

| 状态 | 显示 |
|------|------|
| 未完成 | \`[ ]\` |
| 已完成 | \`[x]\` |

## 数据存储

待办数据存储在数据目录下：

| 平台 | 路径 |
|------|------|
| macOS / Linux | \`~/.jdata/report/todo.json\` |
| Windows | \`%USERPROFILE%\\.jdata\\report\\todo.json\` |
`,Be=e({default:()=>Ve}),Ve=`## 文件工具

### Read

读取文件内容，支持图片格式。

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| path | string | 是 | 文件路径（绝对或相对路径） |
| offset | uint | 否 | 起始行号（0-based，默认从头开始） |
| limit | uint | 否 | 读取行数（默认读取全部） |

**支持的图片格式：** PNG、JPG、GIF、WEBP、BMP

\`\`\`json
{"path": "src/main.rs"}
{"path": "screenshot.png"}
{"path": "large.log", "offset": 100, "limit": 50}
\`\`\`

### Write

写入文件，自动创建父目录。会覆盖已存在的文件。

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| path | string | 是 | 文件路径 |
| content | string | 是 | 写入内容 |

\`\`\`json
{"path": "src/new_file.rs", "content": "fn main() {}\\n"}
\`\`\`

### Edit

字符串替换编辑文件。\`old_string\` 必须唯一匹配文件中的内容。

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| path | string | 是 | 文件路径 |
| old_string | string | 是 | 要替换的原始字符串（必须唯一匹配） |
| new_string | string | 否 | 替换后的字符串（空则删除） |
| replace_all | bool | 否 | 替换所有匹配项（跳过唯一性检查） |
| confirm_token | string | 否 | 批量替换的一次性确认 token |

**批量替换流程：** \`replace_all=true\` 时需两步调用：首次返回匹配预览和 \`confirm_token\`，再次携带 token 确认执行。

\`\`\`json
{"path": "src/main.rs", "old_string": "fn main() {}", "new_string": "fn main() { println!(\\"Hello\\"); }"}
\`\`\`

### Glob

按模式查找文件。

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| pattern | string | 是 | Glob 模式（如 \`**/*.rs\`、\`src/**/*.ts\`） |
| path | string | 否 | 搜索目录（默认当前目录） |
| excludePattern | string | 否 | 排除模式（如 \`**/node_modules/**\`） |
| limit | uint | 否 | 最大结果数（默认 100） |
| offset | uint | 否 | 跳过前 N 个结果（用于分页） |

\`\`\`json
{"pattern": "**/*.rs"}
{"pattern": "src/**/*.tsx", "excludePattern": "**/node_modules/**"}
\`\`\`

### Grep

正则搜索文件内容。

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| pattern | string | 是 | 正则表达式 |
| path | string | 否 | 搜索路径（默认当前目录） |
| glob | string | 否 | 文件过滤（如 \`*.rs\`） |
| type | string | 否 | 文件类型（js/py/rust/go/java） |
| output_mode | string | 否 | 输出模式：content/files_with_matches/count |
| context | uint | 否 | 上下文行数 |
| head_limit | uint | 否 | 最大结果数 |
| ignore_case | bool | 否 | 忽略大小写（默认 false） |

\`\`\`json
{"pattern": "fn\\\\s+\\\\w+", "type": "rust"}
{"pattern": "TODO", "glob": "*.rs", "output_mode": "count"}
\`\`\`

## 执行工具

### Bash

执行 shell 命令（仅 macOS / Linux）。

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| command | string | 是 | Shell 命令（在 bash -c 中执行） |
| description | string | 否 | 命令描述（显示在 UI 中） |
| cwd | string | 否 | 工作目录 |
| timeout | uint | 否 | 超时秒数（默认 120，最大 600） |
| run_in_background | bool | 否 | 后台执行（返回 task_id） |
| interactive | bool | 否 | 交互模式（返回 sid，配合 Session 使用） |

**注意事项：**
- 不支持交互式命令
- 构建命令建议 timeout 设为 300-600
- 后台任务使用 \`TaskOutput\` 获取结果

\`\`\`json
{"command": "cargo build --release", "timeout": 300}
{"command": "npm run dev", "run_in_background": true}
\`\`\`

### PowerShell

执行 PowerShell 命令（仅 Windows）。

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| command | string | 是 | PowerShell 命令（在 powershell.exe -NoProfile -Command 中执行） |
| description | string | 否 | 命令描述 |
| cwd | string | 否 | 工作目录 |
| timeout | uint | 否 | 超时秒数（默认 120，最大 600） |
| run_in_background | bool | 否 | 后台执行（返回 task_id） |

**注意事项：**
- 不支持交互式命令
- 构建命令建议 timeout 设为 300-600
- 后台任务使用 \`TaskOutput\` 获取结果

\`\`\`json
{"command": "cargo build --release", "timeout": 300}
{"command": "npm run dev", "run_in_background": true}
\`\`\`

### TaskOutput

获取后台任务输出。

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| task_id | string | 是 | 后台任务 ID |
| block | bool | 否 | 是否等待完成（默认 true） |
| timeout | uint | 否 | 等待超时毫秒（默认 30000，最大 600000） |

## 网络工具

### WebFetch

获取网页内容，自动转换为 Markdown 或纯文本。

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| url | string | 是 | 完整 URL（http:// 或 https://） |
| extract_mode | string | 否 | 输出格式：markdown/text |
| max_chars | uint | 否 | 最大字符数（默认 50000） |
| headers | object | 否 | 自定义请求头 |
| authorization | string | 否 | Authorization 头 |

\`\`\`json
{"url": "https://docs.rs/serde"}
{"url": "https://api.github.com/repos/rust-lang/rust", "headers": {"Accept": "application/vnd.github.v3+json"}}
\`\`\`

### WebSearch

使用 Exa 搜索网络（需要 \`EXA_API_KEY\` 环境变量）。

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| query | string | 是 | 搜索关键词 |
| count | uint | 否 | 结果数量（1-10，默认 5） |
| type | string | 否 | 搜索类型：auto/keyword/neural |

## 交互工具

### Ask

向用户请求结构化输入，支持单选/多选。

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| questions | array | 是 | 问题列表（1-4 个） |

**Question 结构：**

| 字段 | 类型 | 描述 |
|------|------|------|
| header | string | 短标签 |
| question | string | 完整问题文本 |
| options | array | 选项列表（2-4 个） |
| multi_select | bool | 是否多选 |

\`\`\`json
{
  "questions": [{
    "header": "风格",
    "question": "选择代码风格",
    "options": ["简洁", "详细", "标准"],
    "multi_select": false
  }]
}
\`\`\`

## 任务工具

### Task

管理任务（create/get/list/update）。

**操作类型：**

| action | 描述 |
|--------|------|
| create | 创建任务（需要 title） |
| get | 获取任务详情（需要 taskId） |
| list | 列出所有任务 |
| update | 更新任务状态 |

**任务状态流转：** \`pending\` → \`in_progress\` → \`completed\`

\`\`\`json
{"action": "create", "title": "实现用户认证", "description": "添加登录/注册功能"}
{"action": "update", "taskId": 1, "status": "in_progress"}
{"action": "list", "ready": true}
\`\`\`

### TodoWrite

管理待办事项列表。

| 参数 | 类型 | 描述 |
|------|------|------|
| todos | array | 待办列表 |
| merge | bool | 是否合并更新 |

**Todo 结构：**

| 字段 | 类型 | 描述 |
|------|------|------|
| id | string/int | 待办 ID |
| content | string | 内容 |
| status | string | 状态：pending/in_progress/completed |

**规则：** 同时只能有一个 \`in_progress\` 项

### TodoRead

读取当前待办列表。返回所有待办项的 id、content 和 status。

## 计划工具

### EnterPlanMode

进入计划模式，只读工具可用，写工具被阻止。

**用途：** 在开始非平凡实现任务前，探索代码库并设计实现方案。

**适用场景：**
- 新功能实现涉及架构决策
- 存在多种有效方案需要用户选择
- 代码修改影响现有行为
- 多文件变更（超过 2-3 个文件）

### ExitPlanMode

退出计划模式，提交计划供用户审批。

| 参数 | 类型 | 描述 |
|------|------|------|
| allowedPrompts | array | 实现计划所需的提示权限 |

## 扩展工具

### LoadSkill

加载指定技能到上下文。

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| name | string | 是 | 技能名称 |
| arguments | string | 否 | 传递给技能的参数 |

**可用技能：**

| 技能 | 描述 |
|------|------|
| chat-module-guide | j-cli Chat 模块架构速查（目录职责、核心数据流、关键结构体） |
| fullstack-team | 多 Agent 协作全栈应用开发 |
| html-ppt | HTML PPT Studio — 制作专业静态 HTML 演示文稿 |
| hyperframes | 视频合成、动画、字幕、音频响应视觉效果 |
| j-cli | 命令行工具工作流自动化（日报/周报/todo/别名/脚本） |
| jcli-dev-guide | j-cli 项目开发者入门指南（项目结构、开发流程） |
| skill-creator | 创建新技能指南 |
| sql-to-go-struct-and-dao | SQL 生成 Go 结构体和 DAO 层代码（基于 gorm） |
| swift-ios-app-gen | iOS 原生应用开发 |
| ui-ux-pro-max | UI/UX 设计和前端实现指导 |
| webapp-gen | 一句话生成完整 Web 应用 |

### Agent

启动子代理处理复杂多步任务。子代理使用全新上下文，可以使用除 Agent 外的所有工具。

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| prompt | string | 是 | 子代理任务描述 |
| description | string | 否 | 简短描述（3-5 词） |
| run_in_background | bool | 否 | 后台运行 |
| worktree | bool | 否 | 创建隔离的 git worktree |
| inherit_permissions | bool | 否 | 继承所有工具权限（无需确认） |

### RegisterHook

注册、列出、移除会话级钩子。

**操作：**

| action | 描述 |
|--------|------|
| register | 注册钩子（需要 event + command） |
| list | 列出所有钩子 |
| remove | 移除钩子（需要 event + index） |
| help | 查看协议文档 |

## 会话进程工具

### Session

操作交互进程会话的 stdin/stdout。当 Bash 以 \`interactive: true\` 启动时返回 sid（会话 ID）。

**操作类型：**

| action | 描述 |
|--------|------|
| stdin | 向进程写入内容（\`\\n\` 表示换行） |
| stdout | 读取进程输出（阻塞等待新输出） |
| quit | 终止会话（进程收到 SIGHUP） |

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| sid | string | 是 | 会话 ID（即 Bash 返回的 task_id） |
| action | string | 是 | 操作类型：stdin/stdout/quit |
| text | string | 否 | action=stdin 时写入的内容（\`\\n\` 为换行） |
| timeout_ms | uint | 否 | action=stdout 时等待输出的最大毫秒（默认 500） |

## 协作工具

### Teammate

创建独立运行的队友 Agent，每个队友有独立的 LLM 连接和对话上下文。队友间通过 \`SendMessage\` 工具通信。

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| name | string | 是 | 队友名称（如 "Frontend"、"Backend"） |
| prompt | string | 是 | 队友的初始任务/指令 |
| role | string | 否 | 角色描述（显示在团队摘要中） |
| inherit_permissions | bool | 否 | 继承所有工具权限（无需确认） |
| worktree | bool | 否 | 创建隔离的 git worktree |

### SendMessage

向队友 Agent 发送消息。支持 \`@mention\` 广播。

### IgnoreMessage

忽略队友 Agent 的消息。

## 工具加载

### LoadTool

加载延迟加载的工具，使其在后续回合中可用。

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| name | string | 是 | 要加载的工具名称 |

**当前延迟加载的工具：** RegisterHook、Browser、ComputerUse、Task

## Git Worktree 工具

### EnterWorktree

创建隔离的 git worktree 并切换会话到其中。适用于多个会话同时编辑同一仓库的场景。

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| name | string | 否 | worktree 名称（仅允许字母、数字、点、下划线、连字符） |

**创建位置：** \`.jcli/worktrees/{name}\`，分支名 \`worktree-{name}\`

### ExitWorktree

退出当前 git worktree 会话。

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| action | string | 是 | "keep" 保留 worktree / "remove" 删除 worktree 和分支 |
| discard_changes | bool | 否 | action=remove 且有未提交更改时必须设为 true |

## 会话工具

### Compact

压缩对话上下文，释放上下文窗口。

**适用场景：**
- 对话变长需要继续高效工作
- 多次尝试解决问题失败后需要清理思路

## ComputerUse 工具

macOS 桌面控制工具，支持截图、点击、输入等操作（仅 macOS）。

### 截图操作

#### screenshot

截取屏幕并返回带有 SoM (Set-of-Mark) 标注的图片。

| 参数 | 类型 | 描述 |
|------|------|------|
| som | bool | 启用 SoM 标注（默认 true） |
| app | string | 目标应用名称 |

**SoM 标注：**
- 为每个可交互元素绘制编号边框
- 返回元素索引供后续点击引用
- 支持通过 \`element\` 参数点击编号元素

### 鼠标操作

#### click

点击指定位置或元素。

| 参数 | 类型 | 描述 |
|------|------|------|
| x, y | number | 坐标点（逻辑点） |
| element | uint | SoM 元素编号 |

#### double_click

双击指定位置。

#### right_click

右键点击指定位置。

#### drag

拖拽操作。

| 参数 | 类型 | 描述 |
|------|------|------|
| start_x, start_y | number | 起始坐标 |
| end_x, end_y | number | 结束坐标 |
| start_element, end_element | uint | SoM 元素编号 |
| duration_ms | uint | 拖拽持续时间 |

#### scroll

滚动操作。

| 参数 | 类型 | 描述 |
|------|------|------|
| dx, dy | int | 滚动量（负值向上） |

### 键盘操作

#### type

输入文本。

| 参数 | 类型 | 描述 |
|------|------|------|
| text | string | 要输入的文本 |
| delay_ms | uint | 按键间隔毫秒 |

#### key

按下单个按键。

| 参数 | 类型 | 描述 |
|------|------|------|
| key | string | 按键名称（如 enter、tab、escape） |

#### keys

按键组合。

| 参数 | 类型 | 描述 |
|------|------|------|
| keys | array | 按键列表（如 \`["cmd", "c"]\`） |

**支持的按键：**
- 字母：a-z
- 数字：0-9
- 特殊：enter、tab、space、escape、delete、backspace
- 方向：up、down、left、right
- 功能：f1-f12、home、end、pageup、pagedown
- 修饰：cmd、shift、alt/option、ctrl

### 辅助操作

#### find_element

查找元素。

| 参数 | 类型 | 描述 |
|------|------|------|
| query | string | 搜索查询 |

#### ax_tree

查询无障碍树。

| 参数 | 类型 | 描述 |
|------|------|------|
| app | string | 目标应用 |
| depth | uint | 树深度限制 |
| role | string | 角色过滤（如 AXButton） |
| clickable | bool | 仅显示可点击元素 |

## 权限配置

权限配置位于 \`.jcli/permissions.yaml\`，支持三种规则：

\`\`\`yaml
# .jcli/permissions.yaml
permissions:
  # 完全放开（跳过所有工具确认）
  allow_all: false
  
  # 允许列表（匹配则跳过确认，支持正则）
  allow:
    - Read
    - Grep
    - Glob
    - "Bash:ls.*"       # 正则匹配命令参数
    - "Bash:git status"
  
  # 拒绝列表（优先于 allow，匹配则直接拒绝）
  deny:
    - "Bash:rm -rf.*"   # 阻止危险命令
    - "Bash:.*sudo.*"   # 阻止 sudo 命令
\`\`\`

### 规则匹配说明

- **简单匹配**：工具名（如 \`Read\`、\`Bash\`）
- **正则匹配**：\`工具名:正则表达式\`（如 \`Bash:rm.*\` 匹配 Bash 工具的 command 参数）
- **优先级**：deny > allow > 默认需要确认

## 上下文引用

| 引用 | 描述 |
|------|------|
| \`@file:路径\` | 包含文件内容（自动读取并注入上下文） |
| \`@skill:名称\` | 加载并激活指定 skill |
`,He={en:{gettingStarted:{title:`Getting Started`,children:{installation:`Installation`,quickStart:`Quick Start`,dataDirectory:`Data Directory`}},coreFeatures:{title:`Core Features`,children:{alias:`Alias Management`,report:`Daily Reports`,todo:`Todo Management`,script:`Script System`,markdownEditor:`Markdown Editor`}},aiFeatures:{title:`AI Features`,children:{aiChat:`AI Chat`,tools:`AI Tools`,commands:`Command`,skills:`Skill`,hooks:`Hook`}},advanced:{title:`Advanced`,children:{browser:`Browser Automation`,remote:`Remote Control`,permissions:`Permissions`,lock:`File Encryption`}}},zh:{gettingStarted:{title:`快速开始`,children:{installation:`安装`,quickStart:`快速上手`,dataDirectory:`数据目录`}},coreFeatures:{title:`核心功能`,children:{alias:`别名管理`,report:`日报系统`,todo:`待办管理`,script:`脚本系统`,markdownEditor:`Markdown 编辑器`}},aiFeatures:{title:`AI 功能`,children:{aiChat:`AI 对话`,tools:`AI 工具`,commands:`Command`,skills:`Skill`,hooks:`Hook`}},advanced:{title:`进阶功能`,children:{browser:`浏览器自动化`,remote:`远程控制`,permissions:`权限配置`,lock:`文件加密`}}}},Ue={en:{back:`← Back to Home`,github:`GitHub`,menu:`Menu`},zh:{back:`← 返回首页`,github:`GitHub`,menu:`菜单`}},J={en:{installation:`Installation`,quickStart:`Quick Start`,dataDirectory:`Data Directory`,alias:`Alias Management`,report:`Daily Reports`,todo:`Todo Management`,script:`Script System`,markdownEditor:`Markdown Editor`,aiChat:`AI Chat`,tools:`AI Tools`,commands:`Command`,skills:`Skill`,hooks:`Hook`,browser:`Browser Automation`,remote:`Remote Control`,permissions:`Permissions`,lock:`File Encryption`},zh:{installation:`安装`,quickStart:`快速上手`,dataDirectory:`数据目录`,alias:`别名管理`,report:`日报系统`,todo:`待办管理`,script:`脚本系统`,markdownEditor:`Markdown 编辑器`,aiChat:`AI 对话`,tools:`AI 工具`,commands:`Command`,skills:`Skill`,hooks:`Hook`,browser:`浏览器自动化`,remote:`远程控制`,permissions:`权限配置`,lock:`文件加密`}};function We(){return[`installation`,`quickStart`,`dataDirectory`,`alias`,`report`,`todo`,`script`,`markdownEditor`,`aiChat`,`tools`,`commands`,`skills`,`hooks`,`browser`,`remote`,`permissions`,`lock`]}var Ge=Object.assign({"./en/aiChat.md":ne,"./en/alias.md":ie,"./en/browser.md":oe,"./en/commands.md":x,"./en/dataDirectory.md":C,"./en/hooks.md":T,"./en/installation.md":D,"./en/lock.md":k,"./en/markdown-editor.md":j,"./en/permissions.md":N,"./en/quickStart.md":F,"./en/remote.md":L,"./en/report.md":z,"./en/script.md":V,"./en/skills.md":U,"./en/todo.md":G,"./en/tools.md":q}),Y=Object.assign({"./zh/aiChat.md":ce,"./zh/alias.md":ue,"./zh/browser.md":fe,"./zh/commands.md":me,"./zh/dataDirectory.md":ge,"./zh/hooks.md":ve,"./zh/installation.md":be,"./zh/lock.md":Se,"./zh/markdown-editor.md":we,"./zh/permissions.md":Ee,"./zh/quickStart.md":Oe,"./zh/remote.md":Ae,"./zh/report.md":Me,"./zh/script.md":Pe,"./zh/skills.md":Ie,"./zh/todo.md":Re,"./zh/tools.md":Be});function Ke(){let e={en:{},zh:{}},t=e=>e.replace(/-([a-z])/g,(e,t)=>t.toUpperCase());for(let[n,r]of Object.entries(Ge)){let i=n.match(/\.\/en\/([\w-]+)\.md$/);if(i&&r?.default){let n=t(i[1]);e.en[n]=r.default}}for(let[n,r]of Object.entries(Y)){let i=n.match(/\.\/zh\/([\w-]+)\.md$/);if(i&&r?.default){let n=t(i[1]);e.zh[n]=r.default}}return e}var X=Ke();function Z(e,t){return X[e]?.[t]||X.en[t]||``}function qe(e,t){return J[e]?.[t]||J.en[t]||t}var Je={en:{prev:`Previous`,next:`Next`},zh:{prev:`上一页`,next:`下一页`}};function Ye({lang:e,activeSection:t,onNavigate:n}){let r=We(),i=J[e],a=Je[e],o=r.indexOf(t),s=o>0?r[o-1]:null,c=o<r.length-1?r[o+1]:null;return(0,d.jsxs)(`div`,{className:`flex items-center justify-between py-8 mt-8 border-t border-stone-200`,children:[(0,d.jsx)(`div`,{className:`flex-1`,children:s&&(0,d.jsxs)(`button`,{onClick:()=>n(s),className:`group flex flex-col items-start text-left hover:bg-stone-100 rounded-lg p-3 -ml-3 transition-colors`,children:[(0,d.jsxs)(`span`,{className:`text-xs text-stone-400 mb-1 flex items-center gap-1`,children:[(0,d.jsx)(`svg`,{className:`w-4 h-4`,fill:`none`,stroke:`currentColor`,viewBox:`0 0 24 24`,strokeWidth:2,children:(0,d.jsx)(`path`,{strokeLinecap:`round`,strokeLinejoin:`round`,d:`M15 19l-7-7 7-7`})}),a.prev]}),(0,d.jsx)(`span`,{className:`text-sm font-medium text-stone-700 group-hover:text-stone-900 transition-colors`,children:i[s]})]})}),(0,d.jsx)(`div`,{className:`flex-1 flex justify-end`,children:c&&(0,d.jsxs)(`button`,{onClick:()=>n(c),className:`group flex flex-col items-end text-right hover:bg-stone-100 rounded-lg p-3 -mr-3 transition-colors`,children:[(0,d.jsxs)(`span`,{className:`text-xs text-stone-400 mb-1 flex items-center gap-1`,children:[a.next,(0,d.jsx)(`svg`,{className:`w-4 h-4`,fill:`none`,stroke:`currentColor`,viewBox:`0 0 24 24`,strokeWidth:2,children:(0,d.jsx)(`path`,{strokeLinecap:`round`,strokeLinejoin:`round`,d:`M9 5l7 7-7 7`})})]}),(0,d.jsx)(`span`,{className:`text-sm font-medium text-stone-700 group-hover:text-stone-900 transition-colors`,children:i[c]})]})})]})}var Xe={en:`On This Page`,zh:`本文目录`},Q=70;function $(e){return e.toLowerCase().replace(/[^\w\u4e00-\u9fa5]+/g,`-`).replace(/^-+|-+$/g,``).slice(0,50)}function Ze(e){let t=e.split(`
`),n=[],r=new Set,i=!1;return t.forEach(e=>{if(e.startsWith("```")){i=!i;return}if(i)return;let t,a;if(e.startsWith(`## `))t=e.slice(3).trim(),a=2;else if(e.startsWith(`### `))t=e.slice(4).trim(),a=3;else if(e.startsWith(`#### `))t=e.slice(5).trim(),a=4;else return;t=t.replace(/\*\*([^*]+)\*\*/g,`$1`),t=t.replace(/\*([^*]+)\*/g,`$1`),t=t.replace(/`([^`]+)`/g,`$1`);let o=$(t),s=1;for(;r.has(o);)o=`${$(t)}-${s}`,s++;r.add(o),n.push({id:o,text:t,level:a})}),n}function Qe({content:e,lang:t}){let n=(0,u.useMemo)(()=>Ze(e),[e]),[r,i]=(0,u.useState)(null),a=(0,u.useRef)(!1),o=(0,u.useCallback)(e=>{let t=document.getElementById(e);if(!t)return;i(e),a.current=!0;let n=t.offsetTop-Q;window.scrollTo({top:n,behavior:`smooth`}),setTimeout(()=>{a.current=!1},500)},[]);return(0,u.useEffect)(()=>{if(n.length===0)return;let e=()=>{if(a.current)return;let e=window.scrollY+Q+20,t=null;for(let r of n){let n=document.getElementById(r.id);n&&n.offsetTop<=e&&(t=r.id)}t&&t!==r&&i(t)};return e(),window.addEventListener(`scroll`,e,{passive:!0}),()=>window.removeEventListener(`scroll`,e)},[n,r]),n.length===0?null:(0,d.jsxs)(`nav`,{className:`hidden xl:block fixed right-0 top-[65px] w-52 h-[calc(100vh-65px)] border-l border-stone-200/70 bg-[#faf9f6]/95 backdrop-blur-sm`,children:[(0,d.jsx)(`div`,{className:`sticky top-0 px-4 py-3 border-b border-stone-200/50 bg-[#faf9f6]`,children:(0,d.jsx)(`span`,{className:`text-xs font-semibold text-stone-400 uppercase tracking-wider`,children:Xe[t]})}),(0,d.jsx)(`ul`,{className:`py-2 px-1 overflow-y-auto max-h-[calc(100vh-120px)]`,children:n.map(({id:e,text:t,level:n})=>{let i=r===e,a=n===3,s=n===4;return(0,d.jsx)(`li`,{children:(0,d.jsx)(`button`,{onClick:()=>o(e),className:`
                  relative w-full text-left py-1.5 px-3 rounded-lg transition-all duration-200
                  ${s?`pl-8 text-xs`:a?`pl-6 text-xs`:`text-sm mt-1`}
                  ${i?s?`text-stone-700 font-medium bg-stone-50 before:absolute before:left-5 before:top-1/2 before:-translate-y-1/2 before:w-1 before:h-1 before:bg-stone-400 before:rounded-full`:a?`text-stone-800 font-medium bg-stone-50 before:absolute before:left-3 before:top-1/2 before:-translate-y-1/2 before:w-1 before:h-1 before:bg-stone-500 before:rounded-full`:`text-stone-900 font-medium bg-stone-100 before:absolute before:left-0 before:top-1/2 before:-translate-y-1/2 before:w-0.5 before:h-4 before:bg-stone-900 before:rounded-full`:s?`text-stone-400 hover:text-stone-500 hover:bg-stone-50`:a?`text-stone-400 hover:text-stone-600 hover:bg-stone-50`:`text-stone-500 hover:text-stone-700 hover:bg-stone-50`}
                `,children:t})},e)})})]})}function $e(){let[e,t]=i(),[n,r]=(0,u.useState)(`zh`),[o,s]=(0,u.useState)(!1),c=e.get(`section`)||`installation`,p=Ue[n],m=He[n],h=e=>{t({section:e})};return(0,u.useEffect)(()=>{let e=document.getElementById(c);e&&e.scrollIntoView({behavior:`smooth`,block:`start`})},[c]),(0,d.jsxs)(`div`,{className:`min-h-screen bg-[#faf9f6] text-stone-800`,children:[(0,d.jsx)(`nav`,{className:`fixed top-0 left-0 right-0 z-50 bg-[#faf9f6]/95 backdrop-blur-sm border-b border-stone-200/50`,children:(0,d.jsxs)(`div`,{className:`px-4 sm:px-6 py-4 flex items-center justify-between`,children:[(0,d.jsxs)(`div`,{className:`flex items-center gap-3`,children:[(0,d.jsx)(`button`,{onClick:()=>s(!o),className:`lg:hidden p-2 -ml-2 text-stone-500 hover:text-stone-900 transition-colors`,children:(0,d.jsx)(`svg`,{className:`w-6 h-6`,fill:`none`,stroke:`currentColor`,viewBox:`0 0 24 24`,strokeWidth:2,children:o?(0,d.jsx)(`path`,{strokeLinecap:`round`,strokeLinejoin:`round`,d:`M6 18L18 6M6 6l12 12`}):(0,d.jsx)(`path`,{strokeLinecap:`round`,strokeLinejoin:`round`,d:`M4 6h16M4 12h16M4 18h16`})})}),(0,d.jsxs)(a,{to:`/`,className:`flex items-center gap-2`,children:[(0,d.jsx)(`span`,{className:`text-2xl font-bold text-stone-900`,children:`j`}),(0,d.jsx)(`span`,{className:`text-stone-400 text-sm hidden sm:inline`,children:`docs`})]})]}),(0,d.jsxs)(`div`,{className:`flex items-center gap-3 sm:gap-5`,children:[(0,d.jsx)(a,{to:`/`,className:`text-stone-500 hover:text-stone-900 transition-colors text-sm hidden sm:inline`,children:n===`zh`?`首页`:`Home`}),(0,d.jsx)(l,{lang:n,onChange:r}),(0,d.jsxs)(`a`,{href:`https://github.com/LingoJack/jcli`,target:`_blank`,rel:`noopener noreferrer`,className:`flex items-center gap-2 text-stone-500 hover:text-stone-900 transition-colors`,children:[(0,d.jsx)(`svg`,{className:`w-5 h-5`,fill:`currentColor`,viewBox:`0 0 24 24`,children:(0,d.jsx)(`path`,{fillRule:`evenodd`,clipRule:`evenodd`,d:`M12 2C6.477 2 2 6.477 2 12c0 4.42 2.87 8.17 6.84 9.5.5.08.66-.23.66-.5v-1.69c-2.77.6-3.36-1.34-3.36-1.34-.46-1.16-1.11-1.47-1.11-1.47-.91-.62.07-.6.07-.6 1 .07 1.53 1.03 1.53 1.03.87 1.52 2.34 1.07 2.91.83.09-.65.35-1.09.63-1.34-2.22-.25-4.55-1.11-4.55-4.92 0-1.11.38-2 1.03-2.71-.1-.25-.45-1.29.1-2.64 0 0 .84-.27 2.75 1.02.79-.22 1.65-.33 2.5-.33.85 0 1.71.11 2.5.33 1.91-1.29 2.75-1.02 2.75-1.02.55 1.35.2 2.39.1 2.64.65.71 1.03 1.6 1.03 2.71 0 3.82-2.34 4.66-4.57 4.91.36.31.69.92.69 1.85v2.74c0 .27.16.59.67.5C19.14 20.16 22 16.42 22 12A10 10 0 0012 2z`})}),(0,d.jsx)(`span`,{className:`text-sm hidden sm:inline`,children:p.github})]})]})]})}),(0,d.jsx)(f,{tree:m,activeSection:c,onNavigate:h,isOpen:o,onClose:()=>s(!1)}),(0,d.jsx)(`main`,{className:`lg:ml-64 xl:mr-52 pt-[65px]`,children:(0,d.jsxs)(`div`,{className:`max-w-3xl mx-auto px-6 pb-16`,children:[(()=>{let e=Z(n,c),t=qe(n,c);return e?(0,d.jsxs)(`div`,{id:c,className:`py-8`,children:[(0,d.jsx)(`h1`,{className:`text-3xl font-light text-stone-900 mb-6`,children:t}),(0,d.jsx)(te,{content:e})]},`${n}-${c}`):null})(),(0,d.jsx)(Ye,{lang:n,activeSection:c,onNavigate:h})]})}),(0,d.jsx)(Qe,{content:Z(n,c)||``,lang:n}),(0,d.jsx)(`footer`,{className:`lg:ml-64 xl:mr-52 border-t border-stone-200 py-8 px-6 bg-[#faf9f6]`,children:(0,d.jsxs)(`div`,{className:`max-w-3xl mx-auto flex items-center justify-between text-sm`,children:[(0,d.jsx)(a,{to:`/`,className:`text-stone-500 hover:text-stone-900 transition-colors`,children:p.back}),(0,d.jsxs)(`div`,{className:`flex items-center gap-6`,children:[(0,d.jsx)(`a`,{href:`https://github.com/LingoJack/jcli`,target:`_blank`,rel:`noopener noreferrer`,className:`text-stone-500 hover:text-stone-900 transition-colors`,children:`GitHub`}),(0,d.jsx)(`a`,{href:`https://crates.io/crates/j-cli`,target:`_blank`,rel:`noopener noreferrer`,className:`text-stone-500 hover:text-stone-900 transition-colors`,children:`crates.io`})]})]})})]})}export{$e as default};