# Agent-Made Rules

This file contains operational rules written by agents for use in this
repository. Add new agent-made rules here rather than in any `AGENTS.md` file.

## Goal-Agent Waiting

When a goal agent must wait for a command, test run, experiment, remote job, or
similar operation to finish, it must keep the goal active by issuing repeated
wait or polling tool calls. Normal waiting is not a reason to stall, abandon,
pause, or end the goal.

Use fairly long wait intervals when the available tool supports them. For a run
expected to take roughly three hours, waiting about 30 minutes per tool call is
a good default. After each call, inspect progress and continue waiting until the
operation completes or yields an actionable failure.

This matters because a stalled goal agent must be restarted manually, and a
manual restart is only possible sometimes. Keep goal agents running for as long
as possible while work is still in progress.
