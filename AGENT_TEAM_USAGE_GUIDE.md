# AgentTeam Tool - Quick Usage Guide

## What is AgentTeam?

AgentTeam is a new tool that lets you run multiple independent AI agents in parallel on different tasks. Each agent works on their own investigation, and results are aggregated together. Optionally, a coordinator agent can synthesize all the findings.

## When to Use It

| Scenario | Use Case | Agent | AgentTeam |
|----------|----------|-------|-----------|
| Single complex task | "Analyze this file" | ✅ | ❌ |
| Multiple similar tasks | "Analyze 5 similar files" | ✅ | ✅ (better) |
| Parallel research | "Research backend AND frontend" | ❌ | ✅ (designed for this) |
| Dependent tasks | "Task B needs Task A results" | ✅ | ❌ |
| Compare approaches | "Show me 3 solutions to X" | ❌ | ✅ (designed for this) |

## Basic Structure

Every AgentTeam call needs:

```json
{
  "prompts": [
    {
      "name": "Descriptive Name",
      "prompt": "The actual task/prompt for this agent"
    },
    {
      "name": "Another Agent",
      "prompt": "Another independent task"
    }
  ]
}
```

Optional fields:
- `coordinator_prompt`: Task for a final synthesis agent
- `timeout_secs`: How long to wait (default: 300 seconds)

## Examples

### 1. Parallel Code Review

```json
{
  "prompts": [
    {
      "name": "Security Reviewer",
      "prompt": "Review /src/command/chat/tools/agent_team.rs for security vulnerabilities. Look for: unsafe code, permission bypasses, data races, or injection risks."
    },
    {
      "name": "Performance Reviewer",
      "prompt": "Review /src/command/chat/tools/agent_team.rs for performance issues. Look for: unnecessary clones, inefficient data structures, or bottlenecks."
    },
    {
      "name": "Architecture Reviewer",
      "prompt": "Review /src/command/chat/tools/agent_team.rs for architecture issues. Look for: poor separation of concerns, tight coupling, or missing abstractions."
    }
  ],
  "coordinator_prompt": "Prioritize the findings by severity and impact. Which issues should be fixed first?"
}
```

### 2. Multi-Angle Investigation

```json
{
  "prompts": [
    {
      "name": "API Design Expert",
      "prompt": "Review how the AgentTeam tool accepts parameters. Is the JSON schema well-designed? Are parameter names clear? Would users understand how to use it?"
    },
    {
      "name": "Error Handling Expert",
      "prompt": "Review the error messages in AgentTeam. Are they helpful? Could a user fix issues based on these messages?"
    },
    {
      "name": "Documentation Expert",
      "prompt": "Review the tool description for AgentTeam. Is it clear when to use this vs Agent tool? Are there examples?"
    }
  ],
  "coordinator_prompt": "Summarize the UX improvements needed."
}
```

### 3. Comparative Analysis

```json
{
  "prompts": [
    {
      "name": "Pros Analyst",
      "prompt": "List 5 key advantages of using a multi-threaded agent team approach vs sequential agent calls"
    },
    {
      "name": "Cons Analyst",
      "prompt": "List 5 key disadvantages of using a multi-threaded agent team approach"
    },
    {
      "name": "Use Case Analyst",
      "prompt": "What are the top 5 scenarios where AgentTeam is better than using individual Agent calls?"
    }
  ],
  "coordinator_prompt": "Create a decision matrix: when should someone choose AgentTeam vs Agent?"
}
```

### 4. Parallel Investigation

```json
{
  "prompts": [
    {
      "name": "Frontend Investigator",
      "prompt": "Search for all React components in /src/command/chat/app/. List their names and what they render."
    },
    {
      "name": "Backend Investigator",
      "prompt": "Search for all tool implementations in /src/command/chat/tools/. List their names and descriptions."
    },
    {
      "name": "Data Model Investigator",
      "prompt": "Search for all main data structures in /src/command/chat/storage.rs. List them with their fields."
    }
  ],
  "coordinator_prompt": "Create a unified architecture diagram showing how these components interact"
}
```

## Tips for Best Results

### Team Size
- 2-3 members: Perfect size for most use cases
- 4-5 members: Good for complex scenarios
- 6+ members: Avoid (diminishing returns + longer wait)

### Timeout Settings
- Quick tasks (5 min API time): `timeout_secs: 180`
- Medium tasks (10 min API time): `timeout_secs: 300`
- Long tasks (15+ min API time): `timeout_secs: 600`

### Member Prompts
✅ DO:
- Make each member's task independent
- Use descriptive role names
- Be specific about what to investigate
- Give each member a unique angle

❌ DON'T:
- Make member tasks dependent on each other
- Use generic names like "Task 1"
- Ask for the same thing twice
- Mix in unrelated questions

### Coordinator Prompt
✅ DO use when you want to:
- Synthesize findings across members
- Compare different approaches
- Prioritize recommendations
- Create unified output

❌ DON'T use when:
- You just want individual results
- Synthesis isn't adding value
- Results are already clear

## Common Patterns

### "Interview" Pattern
Multiple agents ask different questions:

```json
{
  "prompts": [
    {"name": "Q1", "prompt": "What is the main purpose of [component]?"},
    {"name": "Q2", "prompt": "Who uses [component] and how?"},
    {"name": "Q3", "prompt": "What are common failure modes of [component]?"}
  ],
  "coordinator_prompt": "Summarize as a short FAQ"
}
```

### "Angle" Pattern
Same task from different perspectives:

```json
{
  "prompts": [
    {"name": "User Perspective", "prompt": "How would an end user... [task]"},
    {"name": "Developer Perspective", "prompt": "How would a developer... [task]"},
    {"name": "Operations Perspective", "prompt": "How would ops staff... [task]"}
  ]
}
```

### "Parallel Search" Pattern
Find things in different locations:

```json
{
  "prompts": [
    {"name": "In Utils", "prompt": "Search /src/util/ for..."},
    {"name": "In Tools", "prompt": "Search /src/command/chat/tools/ for..."},
    {"name": "In App", "prompt": "Search /src/command/chat/app/ for..."}
  ]
}
```

## Expected Output Format

AgentTeam returns formatted results like:

```markdown
## Team Results

### Frontend Expert
**Status:** completed
**Output:**
```
[Member output here...]
```

### Backend Expert
**Status:** completed
**Output:**
```
[Member output here...]
```

## Coordinator Analysis

[Coordinator synthesis here...]
```

## Troubleshooting

| Problem | Solution |
|---------|----------|
| Results are incomplete | Increase `timeout_secs` |
| Results are the same for all members | Make prompts more distinct |
| Coordinator output is unhelpful | Provide clearer synthesis prompt |
| Some members fail | Check if task is impossible or too vague |
| Results take too long | Reduce team size or simplify tasks |

## Comparison with Agent Tool

```
┌─────────────────┬────────────┬──────────────┐
│ Feature         │ Agent      │ AgentTeam    │
├─────────────────┼────────────┼──────────────┤
│ Single task     │ ✅ Better  │ ❌ Overkill  │
│ Parallel tasks  │ ✅ Works   │ ✅ Better    │
│ Complexity      │ ✅ Simple  │ ⚠️ Complex   │
│ Results speed   │ ✅ Fast    │ ⚠️ Slower    │
│ Result quality  │ ✅ Good    │ ✅ Better    │
│ Code reuse      │ ✅ Yes     │ ✅ Yes       │
└─────────────────┴────────────┴──────────────┘
```

## Advanced: Coordinator Patterns

### Synthesis Pattern
```json
"coordinator_prompt": "Combine all findings into one cohesive summary"
```

### Prioritization Pattern
```json
"coordinator_prompt": "Rank the findings by importance and impact"
```

### Decision Matrix Pattern
```json
"coordinator_prompt": "Create a comparison table of the findings"
```

### Recommendation Pattern
```json
"coordinator_prompt": "Based on all findings, what should we do?"
```

## Next Steps

1. Try a simple 2-agent team first
2. Observe the output format
3. Add a coordinator_prompt once comfortable
4. Experiment with larger teams (4-5 members)
5. Use for regular code review and analysis tasks

