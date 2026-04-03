# System Prompt Construction Flow Diagram

## Runtime Resolution Process

```
┌─────────────────────────────────────────────────────────────┐
│ user: j chat                                                │
│       │ initiates chat session                              │
└───────┼─────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────┐
│ ChatApp::init()                                             │
│ ├─ Creates AgentConfig from ~/.jdata/config.yaml           │
│ ├─ Loads disabled_skills, disabled_commands, disabled_tools│
│ └─ Prepares tool_registry                                  │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────────────────┐
│ start_agent() in app.rs (line 2911+)                        │
│                                                             │
│ Creates system_prompt_fn closure:                          │
│                                                             │
│ Box::new(move || {                                         │
│    use super::storage::{load_memory, load_soul,            │
│                        load_style, load_system_prompt};   │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────────────────┐
│ Step 1: Load Template                                       │
│                                                             │
│ let template = load_system_prompt()?;                      │
│ (from ~/.jdata/agent/data/system_prompt.md)               │
│                                                             │
│ ✓ Or use assets/system_prompt_default.md on first run     │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────────────────┐
│ Step 2a: Build Dynamic Summaries                           │
│                                                             │
│ skills_summary = skill::build_skills_summary(             │
│   &loaded_skills, &disabled_skills)                       │
│                                                             │
│ commands_summary = command::build_commands_summary(       │
│   &loaded_commands, &disabled_commands)                   │
│                                                             │
│ tools_summary = tool_registry.build_tools_summary(        │
│   &disabled_tools)                                        │
│                                                             │
│ ✓ Each summary is formatted as markdown list               │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────────────────┐
│ Step 2b: Load Persistent Data                              │
│                                                             │
│ style_text = load_style()                                 │
│   (from ~/.jdata/agent/data/style.md)                     │
│                                                             │
│ memory_text = load_memory()                               │
│   (from ~/.jdata/agent/data/memory.md)                    │
│                                                             │
│ soul_text = load_soul()                                   │
│   (from ~/.jdata/agent/data/soul.md)                      │
│                                                             │
│ current_dir = std::env::current_dir()                     │
│   (runtime working directory)                              │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────────────────┐
│ Step 3: String Substitution                                │
│                                                             │
│ let resolved = template                                   │
│   .replace("{{.current_dir}}", &current_dir)             │
│   .replace("{{.skills}}", &skills_summary)               │
│   .replace("{{.skill_dir}}", &skill_dir)                 │
│   .replace("{{.project_skill_dir}}", &project_skill_dir) │
│   .replace("{{.commands}}", &commands_summary)           │
│   .replace("{{.tools}}", &tools_summary)                 │
│   .replace("{{.style}}", &style_text)                    │
│   .replace("{{.memory}}", &memory_text)                  │
│   .replace("{{.soul}}", &soul_text);                     │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────────────────┐
│ Step 4: Return Resolved System Prompt                      │
│                                                             │
│ Some(resolved)  // Returned from system_prompt_fn()       │
│                                                             │
│ // Closure ends                                            │
│ })                                                         │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────────────────┐
│ AgentHandle::spawn(                                         │
│   ..., system_prompt_fn, ...)                             │
│                                                             │
│ Passes closure to background thread                        │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────────────────┐
│ Background Thread: run_agent_loop()                         │
│                                                             │
│ ┌─ Main Loop Iteration ─────────────────────────┐         │
│ │                                               │         │
│ │ First time in loop:                          │         │
│ │ system_prompt = system_prompt_fn()           │         │
│ │   ↑ File I/O happens here (non-blocking)    │         │
│ │                                               │         │
│ └───────────────────────────────────────────────┘         │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────────────────┐
│ build_request_with_tools(                                  │
│   provider, messages, tools, system_prompt.as_deref())   │
│                                                             │
│ if let Some(sys) = system_prompt {                        │
│     let trimmed = sys.trim();                             │
│     if !trimmed.is_empty() {                              │
│         ChatCompletionRequestSystemMessageArgs::default() │
│             .content(trimmed)                             │
│             .build()?                                     │
│     }                                                     │
│ }                                                         │
│                                                             │
│ ✓ Injects as OpenAI system message                        │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────────────────┐
│ CreateChatCompletionRequest {                              │
│   model: "claude-3-5-sonnet",                             │
│   messages: [                                              │
│     SystemMessage { content: resolved_prompt },           │
│     ...UserMessages,                                      │
│     ...AssistantMessages,                                 │
│     ...ToolMessages,                                      │
│   ],                                                      │
│   tools: [...enabled_tools],                             │
│ }                                                         │
│                                                             │
│ ▶ Ready to send to OpenAI API                             │
└─────────────────────────────────────────────────────────────┘
```

## Template Placeholders Reference

| Placeholder | Source | Content |
|-------------|--------|---------|
| `{{.current_dir}}` | Runtime | Current working directory |
| `{{.skills}}` | Dynamic | Markdown list of available skills |
| `{{.skill_dir}}` | Config | Path to global skills directory |
| `{{.project_skill_dir}}` | Config | Path to project-specific skills |
| `{{.commands}}` | Dynamic | Markdown list of available commands |
| `{{.tools}}` | Dynamic | Markdown list of available tools |
| `{{.style}}` | File | User's response style preferences |
| `{{.memory}}` | File | Persistent user information |
| `{{.soul}}` | File | User's personality/instructions |

## File Dependencies

```
system_prompt_default.md (template)
    ├─ Embedded in binary by rust-embed
    ├─ Copied to ~/.jdata/agent/data/system_prompt.md on first run
    └─ User can edit ~/.jdata/agent/data/system_prompt.md

Runtime Resolution depends on:
    ├─ ~/.jdata/agent/data/style.md (optional)
    ├─ ~/.jdata/agent/data/memory.md (optional)
    ├─ ~/.jdata/agent/data/soul.md (optional)
    ├─ ~/.jdata/skills/ (all available skills)
    ├─ ~/.jdata/agent/config.yaml (disabled_skills, etc.)
    └─ Current working directory
```

## When System Prompt is Resolved

- **Once per session** when `start_agent()` is called
- **In background thread** to avoid blocking UI
- **Before first LLM request** to ensure all data is ready
- **Static for entire session** (no dynamic updates per turn)

## When System Prompt is NOT Updated

- During agent loop iterations (reused)
- When compacting conversation (not included in compact summary)
- Between tool calls (fixed for session)
- On task completion (no auto-update)

