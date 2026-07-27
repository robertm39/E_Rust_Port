# Process-control shell compatibility

## Status

Completed for Bead `E_Rust_Port-j76.1.19` as a source-backed compatibility
decision with native-Windows execution and permanent Rust regressions. This
host has neither a C compiler nor an installed WSL distribution, so a fresh C
executable could not be built for a live byte comparison in this session.

## C source boundary

`ECtrlCreateGeneric` appends, in order, the prover text, `E_OPTIONS_BASE`, the
selected options, one space, extra options, ` --cpu-limit=`, the decimal CPU
limit, one space, and the input-file text. It performs no quoting or escaping
before passing the resulting string to `popen(..., "r")`.

The POSIX [`popen` contract](https://pubs.opengroup.org/onlinepubs/7990949875/functions/popen.html)
executes the command through `sh -c`. Microsoft documents `_popen` as spawning
the command processor and its [`_spawn` process-control reference](https://learn.microsoft.com/en-us/cpp/c-runtime-library/spawn-wspawn-functions?view=msvc-170)
identifies that shape as `cmd.exe /c`; the [`cmd` contract](https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/cmd)
defines `/c` as executing the supplied command string and exiting. The Rust
compatibility constructor therefore uses `/bin/sh -c` on POSIX and
`cmd.exe /C` on Windows. It does not honor an arbitrary `COMSPEC` override,
which could select a processor unlike the C runtime's documented target.

## Quoting decision

The complete C-shaped command remains one logical shell operand. Tests inspect
the `Command` program and arguments and pin the command text unchanged with:

- a quoted prover path containing spaces;
- a quoted option value;
- stderr redirection;
- a command separator; and
- an unquoted input-file path containing a space.

The unquoted path is intentionally not repaired: splitting and metacharacter
interpretation are observable legacy behavior of the C constructor. The normal
production constructor continues to use structured process arguments; only the
explicit compatibility constructor opts into this shell behavior.

## Diagnostic decision

`popen` can successfully create its shell pipe even when the command named
inside the shell does not exist. The shell then produces no PID line on stdout,
so C reaches `Error("Cannot read eprover PID line", OTHER_ERROR)` rather than
the earlier `SysError("Cannot start eprover subprocess", SYS_ERROR)` path.

The native-Windows regression exercises that boundary through `cmd.exe` and
pins `Cannot read eprover PID line` with exit code 11. A non-PID first line is
also pinned to code 11. Rust retains code 7 and the host I/O error suffix only
when the shell process itself cannot be spawned; the reusable library returns
that suffix inline because it does not own a C-style global program name and
stderr stream.

## Performance decision

The production structured-spawn path is unchanged. The compatibility path
still launches exactly one shell followed by the requested command, so a
benchmark is not warranted.

## Validation

- focused `control::proc_ctrl::tests`: 14 passed
- native-Windows compatibility constructor execution passed
- native-Windows missing-command/PID diagnostic regression passed
- `cargo fmt --all -- --check`
- `cargo test --locked --lib --quiet`: 4,111 passed
- `cargo test --locked --bins --quiet`: all binary targets passed
- `cargo test --locked --test eprover_schedule --quiet`: 3 passed
- `cargo test --locked --test executable_inventory --quiet`: 1 passed
- `cargo check --locked --all-targets`
- `cargo clippy --locked --all-targets -- -D warnings`
- C-source coverage: 492 source files covered by 266 unit docs
- Change Later wording: 269 Markdown files checked
- local Markdown links: 269 Markdown files checked
- regeneration preservation: manual sections preserved in 268 Markdown files
- `git diff --check`
