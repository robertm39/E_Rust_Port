# Global output descriptor reconciliation

## Status

Completed for Bead `E_Rust_Port-j76.2.94`. The vendored C source remained
unchanged. A new permanent regression proves that `GlobalOutFD` is a working
raw-write surface for the selected global file, not merely a nonnegative
platform token.

## C source contract

`INOUT/cio_output.c` initializes `GlobalOutFD` to `STDOUT_FILENO`. `InitOutput`
restores the same value, and `OpenGlobalOut` opens the `FILE *` target and stores
`fileno(GlobalOut)`. The descriptor is a non-owning view of the same C stream;
`OutClose(GlobalOut)` flushes and closes the owning stream.

The only C behavior that needs the raw descriptor rather than `FILE *` is the
low-allocation signal path in `INOUT/cio_signals.c`. It calls `WriteStr` and
`TSTPOUTFD` so hard-timeout output can bypass the stdio buffer. That byte and
ordering contract was closed separately by the signal-delivery reconciliation.

## Rust platform boundary

Rust represents the process-global writer as an owning enum rather than a
`FILE *`, but retains an `i32` descriptor compatibility surface:

- stdout is exactly descriptor `1`;
- Unix file targets expose `File::as_raw_fd()`, so the descriptor is borrowed
  from the same owning file;
- MSVC Windows duplicates the owned file handle and transfers that duplicate
  to a UCRT descriptor;
- GNU/MinGW Windows uses the same shape with MSVCRT; and
- other target ABIs return the explicit `-1` sentinel and their low-level
  `WriteStr` fallback reports failure instead of treating an incompatible OS
  handle as a C-runtime descriptor.

The Windows duplicate is necessary because Rust `File` owns an OS handle while
`_open_osfhandle` transfers its handle to the returned CRT descriptor.
[`_close` then closes both that descriptor and its underlying duplicate
handle](https://learn.microsoft.com/en-us/cpp/c-runtime-library/reference/open-osfhandle?view=msvc-170).
`CompatFd` owns exactly that close operation; the original Rust `File` remains
the global output owner. Replacing or closing the global target drops both
owners once without double-closing either handle.

The reference deployment is Linux and the native port deployment is Windows;
both supported descriptor ABIs are therefore represented. Keeping an honest
sentinel on unrelated Rust targets is preferable to claiming compatibility for
an unimplemented C-runtime descriptor ABI. The broader question of removing
the process-global raw descriptor remains in post-compatibility Bead
`E_Rust_Port-j76.4.866`.

## Functional regression

The previous platform regression checked only that a supported file target did
not return `-1`. The new regression opens a real global output file, sends
`raw-` through `write_str_to_fd(global_out_fd(), ...)`, sends `owned` through
the ordinary global Rust writer, closes the owner, and reads the file back.
The exact `raw-owned` result proves all of the compatibility properties needed
by C's consumer:

- the descriptor accepts the platform's real one-shot raw write;
- it names the selected global file;
- it shares the file position with the owned writer; and
- closing the global target flushes and releases both ownership paths cleanly.

The test is compiled on Unix, MSVC Windows, and GNU/MinGW Windows. It passed on
the current `x86_64-pc-windows-msvc` host against UCRT. Linux's underlying raw
descriptor behavior is additionally exercised by the retained native hard
`SIGXCPU` evidence from the signal-delivery reconciliation; no WSL distribution
was visible in this sandbox for a new Unix run.

## Performance decision

Production code is unchanged. The descriptor conversion happens only when a
global file is opened, and the new work is test/documentation only. No
performance benchmark is warranted.

## Validation

- 8 focused `inout::output` tests, including the real raw-descriptor write;
- serialized all-target/all-feature Rust suite;
- strict all-target/all-feature Clippy with pedantic warnings denied;
- Rust formatting and documentation quality gates; and
- unchanged vendored C worktree.
