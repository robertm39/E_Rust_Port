# Summary
This project is a Rust port of the E theorem prover.

# Original Source
The original source of the E theorem prover is in `eprover/`. It is directly from the Github repo. Do not modify the contents of it at all.

# Source Control
This project uses git for source control. Make well-scoped commits with good commit messages. Push after committing. Use `git status` to confirm that the project is clean before starting a new piece of work.

# Porting Rules
This port must support all of the features of the original, so that the Rust executable is a drop-in replacement for the C executable. It must also have performance that is at least comparable to the original, and implement all of the optimizations the original implements (unless there is a specific and compelling reason otherwise). It should generally stay close to how the original is implemented, while still being idiomatic, high-quality Rust.

# Testing
Write thorough tests to confirm functionality and performance.

# Agent-Made Docs
Agent-made docs are in `DOCS.md` and in the locations specified in that file. Agents are absolutely not permitted to modify `AGENTS.md` in any way; they should modify `DOCS.md` instead. Also, Rust code standards are specified in `docs/c_source_docs/rust-code-standards.md`, and you must follow them (unless they conflict with this file).