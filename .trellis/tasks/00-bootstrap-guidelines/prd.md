# Bootstrap Task: Fill Project Development Guidelines

> 2026-10-09 coverage handoff: the related, independently approved
> `.trellis/tasks/10-09-quality-standards-foundation` populated source-backed first-party
> guidance and retained this task/state/history. Start at `.trellis/spec/index.md` and
> `docs/audits/2026-10-09-quality-standards-foundation.md` for current coverage and limits.
> The bootstrap checkboxes below are historical bookkeeping, not a remaining-template claim;
> this task was not automatically completed or archived.

**You (the AI) are running this task. The developer does not read this file.**

The developer just ran `trellis init` on this project for the first time.
`.trellis/` now exists with empty spec scaffolding, and this bootstrap task
exists under `.trellis/tasks/`. When they want to work on it, they should start
this task from a session that provides Trellis session identity.

**Your job**: help them populate `.trellis/spec/` with the team's real
coding conventions. Every future AI session — this project's
`trellis-implement` and `trellis-check` sub-agents — auto-loads spec files
listed in per-task jsonl manifests. Empty spec = sub-agents write generic
code. Real spec = sub-agents match the team's actual patterns.

Don't dump instructions. Open with a short greeting, figure out if the repo
has any existing convention docs (CLAUDE.md, .cursorrules, etc.), and drive
the rest conversationally.

---

## Status (update the checkboxes as you complete each item)

- [ ] Fill guidelines for nxtools-browser-host
- [ ] Fill guidelines for nxtools-cli
- [ ] Fill guidelines for nxtools-desktop
- [ ] Fill guidelines for nxtools-indexer
- [ ] Fill guidelines for nxtools-clipboard
- [ ] Fill guidelines for nxtools-core
- [ ] Fill guidelines for nxtools-download
- [ ] Fill guidelines for nxtools-download-ipc
- [ ] Fill guidelines for nxtools-finder
- [ ] Fill guidelines for nxtools-ipc
- [ ] Fill guidelines for nxtools-network
- [ ] Fill guidelines for nxtools-protocol
- [ ] Add code examples

---

## Spec files to populate

### Package: nxtools-browser-host (`spec/nxtools-browser-host/`)

- Backend guidelines: `.trellis/spec/nxtools-browser-host/backend/`

### Package: nxtools-cli (`spec/nxtools-cli/`)

- Backend guidelines: `.trellis/spec/nxtools-cli/backend/`

### Package: nxtools-desktop (`spec/nxtools-desktop/`)

- Backend guidelines: `.trellis/spec/nxtools-desktop/backend/`

### Package: nxtools-indexer (`spec/nxtools-indexer/`)

- Backend guidelines: `.trellis/spec/nxtools-indexer/backend/`

### Package: nxtools-clipboard (`spec/nxtools-clipboard/`)

- Backend guidelines: `.trellis/spec/nxtools-clipboard/backend/`

### Package: nxtools-core (`spec/nxtools-core/`)

- Backend guidelines: `.trellis/spec/nxtools-core/backend/`

### Package: nxtools-download (`spec/nxtools-download/`)

- Backend guidelines: `.trellis/spec/nxtools-download/backend/`

### Package: nxtools-download-ipc (`spec/nxtools-download-ipc/`)

- Backend guidelines: `.trellis/spec/nxtools-download-ipc/backend/`

### Package: nxtools-finder (`spec/nxtools-finder/`)

- Backend guidelines: `.trellis/spec/nxtools-finder/backend/`

### Package: nxtools-ipc (`spec/nxtools-ipc/`)

- Backend guidelines: `.trellis/spec/nxtools-ipc/backend/`

### Package: nxtools-network (`spec/nxtools-network/`)

- Backend guidelines: `.trellis/spec/nxtools-network/backend/`

### Package: nxtools-protocol (`spec/nxtools-protocol/`)

- Backend guidelines: `.trellis/spec/nxtools-protocol/backend/`


### Thinking guides (already populated)

`.trellis/spec/guides/` contains general thinking guides pre-filled with
best practices. Customize only if something clearly doesn't fit this project.

---

## How to fill the spec

### Step 1: Import from existing convention files first (preferred)

Search the repo for existing convention docs. If any exist, read them and
extract the relevant rules into the matching `.trellis/spec/` files —
usually much faster than documenting from scratch.

| File / Directory | Tool |
|------|------|
| `CLAUDE.md` / `CLAUDE.local.md` | Claude Code |
| `AGENTS.md` | Codex / Claude Code / agent-compatible tools |
| `.cursorrules` | Cursor |
| `.cursor/rules/*.mdc` | Cursor (rules directory) |
| `.windsurfrules` | Windsurf |
| `.clinerules` | Cline |
| `.roomodes` | Roo Code |
| `.github/copilot-instructions.md` | GitHub Copilot |
| `.vscode/settings.json` → `github.copilot.chat.codeGeneration.instructions` | VS Code Copilot |
| `CONVENTIONS.md` / `.aider.conf.yml` | aider |
| `CONTRIBUTING.md` | General project conventions |
| `.editorconfig` | Editor formatting rules |

### Step 2: Analyze the codebase for anything not covered by existing docs

Scan real code to discover patterns. Before writing each spec file:
- Find 2-3 real examples of each pattern in the codebase.
- Reference real file paths (not hypothetical ones).
- Document anti-patterns the team clearly avoids.

### Step 3: Document reality, not ideals

**Critical**: write what the code *actually does*, not what it should do.
Sub-agents match the spec, so aspirational patterns that don't exist in the
codebase will cause sub-agents to write code that looks out of place.

If the team has known tech debt, document the current state — improvement
is a separate conversation, not a bootstrap concern.

---

## Quick explainer of the runtime (share when they ask "why do we need spec at all")

- Every AI coding task spawns two sub-agents: `trellis-implement` (writes
  code) and `trellis-check` (verifies quality).
- Each task has `implement.jsonl` / `check.jsonl` manifests listing which
  spec files to load.
- The platform hook auto-injects those spec files + the task's `prd.md`
  into every sub-agent prompt, so the sub-agent codes/reviews per team
  conventions without anyone pasting them manually.
- Source of truth: `.trellis/spec/`. That's why filling it well now pays
  off forever.

---

## Completion

When the developer confirms the checklist items above are done with real
examples (not placeholders), guide them to run:

```bash
python ./.trellis/scripts/task.py finish
python ./.trellis/scripts/task.py archive 00-bootstrap-guidelines
```

After archive, every new developer who joins this project will get a
`00-join-<slug>` onboarding task instead of this bootstrap task.

---

## Suggested opening line

"Welcome to Trellis! Your init just set me up to help you fill the project
spec — a one-time setup so every future AI session follows the team's
conventions instead of writing generic code. Before we start, do you have
any existing convention docs (CLAUDE.md, .cursorrules, CONTRIBUTING.md,
etc.) I can pull from, or should I scan the codebase from scratch?"
