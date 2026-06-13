---
governance_version: 1
---

# Forge Global Agents Instructions

This file is read automatically at the start of every Forge conversation.
It is loaded from `~/forge/AGENTS.md` (global) or `<project>/AGENTS.md`
(project-local, takes precedence). Adapted from the superpowers
`using-superpowers` skill bootstrap so it auto-triggers in Forge.

---

## <EXTREMELY-IMPORTANT>

If you think there is even a 1% chance a skill might apply to what you are
doing, you ABSOLUTELY MUST invoke the skill.

If a skill applies to your task, you do not have a choice. You must use it.

This is not negotiable. This is not optional. You cannot rationalize your
way out of this.

</EXTREMELY-IMPORTANT>

## Skill discovery in Forge

- Use the `:skill` command to list every skill Forge has loaded.
- The `skill` tool is available inside the conversation. Invoke it by name
  (e.g. `brainstorming`, `systematic-debugging`, `test-driven-development`,
  `verification-before-completion`, `code-review`, etc.) before responding
  to a user request.
- Skills live in three locations; precedence is
  **project `.forge/skills/` > `~/.agents/skills/` > `~/forge/skills/` >
  built-in**. The `superpowers` skills are installed under
  `~/forge/skills/superpowers/<name>/SKILL.md` and you should invoke the
  full path when activating them: `skill superpowers/brainstorming`,
  `skill superpowers/using-superpowers`, etc.
- `superpowers/using-superpowers` is the bootstrap. It is the FIRST skill
  you invoke in every conversation. It tells you which other skills apply.

## Instruction priority

1. User's explicit instructions (this file, `AGENTS.md`, direct requests) — highest priority
2. Superpowers skills — override default system behavior where they conflict
3. Default system prompt — lowest priority

If this file says "don't use TDD" and a skill says "always use TDD," follow
the user's instructions. The user is in control.

## How to invoke a skill in Forge

- **Interactive mode / `: ` prompts:** ask `using the <skill-name> skill, …`
  or just describe the task; Forge will auto-load the matching skill.
- **One-shot (`forge -p …`):** include the phrase
  `using the superpowers/<skill> skill` in the prompt so the model
  activates the skill's content before answering.
- **The `skill` tool** is also exposed to subagents — call it by name.

## Mandatory invocation rules

1. Before responding to ANY user message, invoke the
   `superpowers/using-superpowers` skill. It tells you which other skills
   apply.
2. Before doing any work that could be characterized as "build X",
   "add a feature", or "let's make", invoke
   `superpowers/brainstorming` first.
3. Before debugging any non-trivial issue, invoke
   `superpowers/systematic-debugging`.
4. Before writing any new code, invoke
   `superpowers/test-driven-development` unless the user explicitly
   opts out.
5. Before claiming work is done, invoke
   `superpowers/verification-before-completion`.
6. Before opening or responding to a PR, invoke
   `superpowers/requesting-code-review` (sender) or
   `superpowers/receiving-code-review` (reviewer).
7. Before ending a feature branch, invoke
   `superpowers/finishing-a-development-branch`.
8. When writing or editing another skill, invoke
   `superpowers/writing-skills`.

## Red flags — STOP if you think any of these

| Thought                                       | Reality                                                  |
|-----------------------------------------------|----------------------------------------------------------|
| "This is just a simple question"              | Questions are tasks. Check for skills.                   |
| "I need more context first"                   | Skill check comes BEFORE clarifying questions.           |
| "Let me explore the codebase first"           | Skills tell you HOW to explore. Check first.             |
| "I can check git/files quickly"               | Files lack conversation context. Check for skills.       |
| "Let me gather information first"             | Skills tell you HOW to gather information.                |
| "This doesn't need a formal skill"            | If a skill exists, use it.                               |
| "I remember this skill"                       | Skills evolve. Read current version.                     |
| "This doesn't count as a task"                | Action = task. Check for skills.                         |
| "The skill is overkill"                       | Simple things become complex. Use it.                    |
| "I'll just do this one thing first"           | Check BEFORE doing anything.                             |
| "This feels productive"                       | Undisciplined action wastes time. Skills prevent this.   |
| "I know what that means"                      | Knowing the concept ≠ using the skill. Invoke it.        |
| "The user is in a hurry, skip the check"      | Skipping the check is what makes the user wait longer.   |

## Skill priority

When multiple skills could apply:

1. **Process skills first** (`brainstorming`, `systematic-debugging`) — they
   determine HOW to approach the task.
2. **Implementation skills second** (`test-driven-development`,
   `subagent-driven-development`, `dispatching-parallel-agents`) — they
   guide execution.
3. **Delivery skills last** (`verification-before-completion`,
   `requesting-code-review`, `finishing-a-development-branch`).

"Let's build X" → `brainstorming` first, then
`test-driven-development` + `subagent-driven-development`.
"Fix this bug" → `systematic-debugging` first, then TDD once the cause
is known.

## Skill types

- **Rigid** (TDD, systematic-debugging, verification-before-completion):
  Follow exactly. Don't adapt away discipline.
- **Flexible** (brainstorming, writing-plans, finishing-a-development-branch):
  Adapt principles to context.

The skill itself tells you which.

## Custom tools available via MCP (already imported)

- `github` — read/write issues, PRs, branches, file contents, run
  actions. Use for anything repo-shaped.
- `context7` — resolve a library id and pull up-to-date, version-pinned
  docs into context. Use before relying on a memory-based API answer.
- `playwright` — headless browser automation for UI testing, scraping,
  and verification.
- `chrome-devtools-mcp` — full DevTools protocol: traces, network,
  performance, console.
- `deepwiki` — query a public GitHub repo's architecture and file
  structure by URL.
- `firecrawl` — web fetch + clean Markdown extraction for arbitrary
  URLs.
- `dinoforge` — local Unity/DOTS dev-tool bridge over a named pipe.
  Local-only; not loaded in non-Dino projects.

## Forge-specific quality bar

- Always read the project `AGENTS.md` and any `forge.yaml` /
  `.forge.toml` BEFORE making changes.
- Prefer `forge mcp list` over guessing which tools are wired up.
- If a tool you want is missing, ask the user to add it via
  `forge mcp import` rather than reimplementing it.
- Use `:sync` (workspace index) at the start of any non-trivial session
  for semantic search instead of `rg` over the whole tree.
- Conversational state and skills persist; you can resume via
  `forge conversation resume <id>`.
