# Project Instructions for AI Agents

This file provides instructions and context for AI coding agents working on this project.

## Project Identity

Umlaut is an independent automated theorem prover written in Rust. It began as
a port of E, but E is now a read-only compatibility, regression, provenance,
and algorithmic reference rather than the product identity or a universal
design and performance authority. Umlaut must retain E's substantive feature
coverage and broadly compatible interfaces, except that its package and
executables intentionally use Umlaut names without legacy aliases. New
features and measured implementation improvements do not need an E analogue.

<!-- BEGIN BEADS INTEGRATION v:1 profile:minimal hash:6cd5cc61 -->
## Beads Issue Tracker

This project uses **bd (beads)** for issue tracking. Run `bd prime` to see full workflow context and commands.

### Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work
bd close <id>         # Complete work
```

### Rules

- Use `bd` for ALL task tracking — do NOT use TodoWrite, TaskCreate, or markdown TODO lists
- Run `bd prime` for detailed command reference and session close protocol
- Use `bd remember` for persistent knowledge — do NOT use MEMORY.md files

**Architecture in one line:** issues live in a local Dolt DB; sync uses `refs/dolt/data` on your git remote; `.beads/issues.jsonl` is a passive export. See https://github.com/gastownhall/beads/blob/main/docs/SYNC_CONCEPTS.md for details and anti-patterns.

## Agent Context Profiles

The managed Beads block is task-tracking guidance, not permission to override repository, user, or orchestrator instructions.

- **Conservative (default)**: Use `bd` for task tracking. Do not run git commits, git pushes, or Dolt remote sync unless explicitly asked. At handoff, report changed files, validation, and suggested next commands.
- **Minimal**: Keep tool instruction files as pointers to `bd prime`; use the same conservative git policy unless active instructions say otherwise.
- **Team-maintainer**: Only when the repository explicitly opts in, agents may close beads, run quality gates, commit, and push as part of session close. A current "do not commit" or "do not push" instruction still wins.

## Session Completion

This protocol applies when ending a Beads implementation workflow. It is subordinate to explicit user, repository, and orchestrator instructions.

1. **File issues for remaining work** - Create beads for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **Handle git/sync by active profile**:
   ```bash
   # Conservative/minimal/default: report status and proposed commands; wait for approval.
   git status

   # Team-maintainer opt-in only, unless current instructions forbid it:
   git pull --rebase
   git push
   git status
   ```
5. **Hand off** - Summarize changes, validation, issue status, and any blocked sync/commit/push step

**Critical rules:**
- Explicit user or orchestrator instructions override this Beads block.
- Do not commit or push without clear authority from the active profile or the current user request.
- If a required sync or push is blocked, stop and report the exact command and error.
<!-- END BEADS INTEGRATION -->

## Repository Beads And Git Policy

This repository explicitly opts into the Beads `team-maintainer` workflow.
Tracked Beads exports are project state and must be committed. Include them in
the same scoped commit as the source or documentation work they describe; use
a dedicated `chore(beads): ...` commit only for Beads-only changes. Keep
automatic export enabled and automatic Git staging disabled so staging remains
intentional. Do not commit ignored Dolt databases, locks, caches, or temporary
files. At successful session close, close completed Beads, run quality gates,
push the Dolt state with `bd dolt push`, commit the tracked exports, push Git,
and verify a clean worktree.


## Build & Test

All Rust and C formatting, compilation, tests, execution, compatibility
comparisons, benchmarks, and profiles run on the ephemeral Ubuntu Linode. Do
not invoke Cargo, Rust binaries, the C build, C binaries, WSL, Valgrind, or
Callgrind on the local computer. This includes quick smoke tests and commands
inside local containers or virtual machines: they are not supported
substitutes for the Linode.

From local PowerShell, orchestrate the complete remote lifecycle:

```powershell
.\linode-runner.ps1 run
```

The command acquires or reuses an exact-match runner, uploads the exact
worktree, performs every required check on the Linode, collects artifacts,
sanitizes the host, and parks it through the already-paid billing hour. A local
Windows task and independent restricted remote reaper guarantee deletion near
the billing boundary; unsafe or missing reaper setup falls back to immediate
deletion. Linux is the runtime compatibility authority. Windows GNU x64 is
compile-only and is never executed. See `DOCS.md` and
`docs/linode-runner.md`.

The default 8 GiB profile costs $0.14 an hour. Use `--high-memory` only when a
task should more closely resemble the CASC configuration; its 150 GB profile
costs $0.74 an hour. No new high-memory `up` or `run` may start after managed
high-memory usage reaches the current fixed UTC-05:00 day's bank-adjusted
capacity (no daylight-saving adjustment). The four-hour daily base accrues into
a bank capped at four hours; overuse consumes the bank and then becomes uncapped
debt that reduces later capacity. Check actual usage, bank, debt, and capacity
with `.\linode-runner.ps1 check --high-memory`. For a closer CASC match, give
every actual prover process `--memory-limit=131072`, the prover's MB value for
128 GiB.

For an exceptional individual Rust or C command, use only the runbook's
guarded `up`/`sync`/`exec`/`down` lifecycle. Put `down` in a PowerShell
`finally` block so a failed remote command is sanitized and parked with both
reapers armed. Use `down --now` only when immediate deletion is intended.
Do not issue a direct local Cargo, compiler, prover, benchmark, Valgrind, or
Callgrind command first.

## Architecture Overview

Umlaut and the unchanged upstream C reference are compared natively on the
same Linux worker. Compatibility comparisons protect supported behavior but do
not require E's internal architecture or reproduce documented upstream
defects. The local machine is limited to editing, orchestration, Git,
documentation checks, PowerShell parsing, and Python controller tests.

## Conventions & Patterns

_Add your project-specific conventions here_
