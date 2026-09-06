# kb - AI Agent Guide

Welcome to `kb`. This repository contains **shippable code, tests, and public docs only**.
All private developer memory, specifications, handoffs, and research live in `scratch/`.

## 1. Operating Model & Read Order

When entering this repository, load context in this priority order:

1. **`scratch/HANDOFF.md`**: Read this FIRST. It contains active state, active blockers, and immediate next commands.
2. **`.agent-rules/behavior/agent-standards.md`**: Cross-project AI behavior, safety boundaries, and hygiene.
3. **`.agent-rules/behavior/spec-lifecycle.md`**: Drift defense rules (Code is Ground Truth, keeping specs synced).
4. **`scratch/AGENTS.md`**: Project-specific invariants, domain rules, and validation gates.
5. **`.agent-rules/skills/<language>/core/SKILL.md`**: Language-specific engineering standards (lazy-load sub-rules as needed).

## 2. Working Inside this Project

You have full access to the real codebase, compiler, tests, and linters.
* **Code is Ground Truth**: If existing scratch documents contradict real code, the code wins. Untangle and clean up outdated scratch documents as you implement.
* **Update Specs on Architecture Changes**: If an implementation detail deviates from `scratch/spec/`, update that spec before finishing your task.
* **Session Close Mandate**: Always update the 3-bullet status block at the top of `scratch/HANDOFF.md` (`Current State`, `Blockers / Breakages`, `Immediate Next Action`) so the next session starts without amnesia.

## 3. Cardinal Rules (Never Break)

- **Do NOT commit, push, or tag** unless explicitly requested by the user.
- **Run the project validation gate** before reporting work complete (e.g. format, lint, tests).
- **Every bug fix MUST include a reproducing test.**
- **Never print, log, or commit secrets.**

## 4. Local Machine Wiring

```text
scratch      -> /home/kristency/knowledge-base/projects/kb-cli
.agent-rules -> /home/kristency/knowledge-base/agent-rules
```

Both `scratch` and `.agent-rules` are local machine symlinks ignored by Git via `.gitignore`.
