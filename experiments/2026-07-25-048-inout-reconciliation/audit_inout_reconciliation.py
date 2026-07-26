#!/usr/bin/env python3
"""Audit the final INOUT Change Later decisions."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import sys
from pathlib import Path


ORDINALS = [
    850,
    851,
    854,
    855,
    856,
    859,
    860,
    863,
    864,
    866,
    867,
    868,
    869,
    873,
    875,
    878,
    879,
    886,
    887,
]


def load_backlog_audit(repo: Path):
    path = (
        repo
        / "experiments/2026-07-25-029-post-compat-backlog-audit/audit_backlog.py"
    )
    spec = importlib.util.spec_from_file_location("post_compat_backlog_audit", path)
    if spec is None or spec.loader is None:
        raise RuntimeError("could not load the post-compatibility audit module")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def source(repo: Path, relative: str) -> str:
    return (repo / relative).read_text(encoding="utf-8")


def contains(repo: Path, relative: str, *needles: str) -> bool:
    text = source(repo, relative)
    return all(needle in text for needle in needles)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    parser.add_argument("--expected", type=Path)
    args = parser.parse_args()
    repo = args.repo.resolve()

    audit = load_backlog_audit(repo)
    issues = audit.load_children("E_Rust_Port-j76.4")
    records = [
        audit.issue_record("E_Rust_Port-j76.4", issue) for issue in issues
    ]
    audit.validate_parent("E_Rust_Port-j76.4", records)
    expected_ids = {f"E_Rust_Port-j76.4.{ordinal}" for ordinal in ORDINALS}
    selected = sorted(
        (record for record in records if record["id"] in expected_ids),
        key=lambda record: record["ordinal"],
    )
    issues_by_id = {issue["id"]: issue for issue in issues}
    stable_records = [
        {
            "content_sha256": record["content_sha256"],
            "id": record["id"],
            "legacy_text": record["legacy_text"],
            "ordinal": record["ordinal"],
            "source_file": record["source_file"],
        }
        for record in selected
    ]
    decision_digest = hashlib.sha256(
        json.dumps(
            stable_records, sort_keys=True, separators=(",", ":")
        ).encode("utf-8")
    ).hexdigest()

    checks = {
        "commandline_float_and_typed_option_tables_are_explicit": contains(
            repo,
            "eprover/INOUT/cio_commandline.h",
            "int         option_code;",
            "char        shortopt;",
            "char*       longopt;",
        )
        and contains(
            repo,
            "eprover/INOUT/cio_commandline.c",
            "for(i=0; options[i].option_code; i++)",
            "ret = strtod(arg, &eoarg);",
            "if(errno || *eoarg)",
        )
        and contains(
            repo,
            "src/inout/commandline.rs",
            "pub struct OptCell<Code>",
            "pub shortopt: Option<char>",
            "pub longopt: Option<&'static str>",
            "candidate.longopt.is_some_and(|longopt| longopt == name)",
            "candidate.shortopt == Some(option)",
            "match parse_c_double(arg)",
            "fn float_arg_matches_c_strtod_shape()",
        )
        and contains(
            repo,
            "docs/rust-port-status.md",
            "case-insensitive `strtod` named infinity/NaN spellings",
            "C99-style hexadecimal float spellings",
        ),
        "file_readability_and_safe_close_ownership_are_retained": contains(
            repo,
            "eprover/INOUT/cio_fileops.c",
            "if(file != stdin)",
            "if(fclose(file) != 0)",
            'fp = fopen(name, "r");',
            "fclose(fp);",
        )
        and contains(
            repo,
            "src/inout/fileops.rs",
            "pub fn input_close(input: InputSource) -> Result<(), Diagnostic>",
            "InputSource::Stdin => Ok(())",
            "InputSource::File(file) =>",
            "drop(file);",
            "pub fn file_exists(name: &Path) -> bool",
            "File::open(name).is_ok()",
            "fn input_close_skips_stdin_and_reports_successful_file_close()",
        )
        and contains(
            repo,
            "docs/rust-port-status.md",
            "safe ownership-based input close",
            "race-prone readability checks",
        ),
        "filevars_boolean_strcmp_bug_is_intentionally_exact": contains(
            repo,
            "eprover/INOUT/cio_filevars.c",
            'if(strcmp(cell->val1.p_val, "true"))',
            "*value = true;",
            'else if(strcmp(cell->val1.p_val, "false"))',
            "*value = false;",
        )
        and contains(
            repo,
            "src/inout/filevars.rs",
            'self.vars.get(name).map(|entry| entry.value != "true")',
            "fn bool_getter_preserves_c_strcmp_bug()",
            'assert_eq!(vars.get_bool("t"), Some(false));',
            'assert_eq!(vars.get_bool("f"), Some(true));',
            'assert_eq!(vars.get_bool("other"), Some(true));',
        ),
        "initio_persistence_and_program_name_ownership_are_covered": contains(
            repo,
            "eprover/INOUT/cio_initio.c",
            "InitError(progname);",
            'tmp = getenv("TPTP");',
            "if(tmp)",
            "TPTP_dir = DStrCopy(tmpstr);",
        )
        and contains(
            repo,
            "src/inout/initio.rs",
            "state.program_name = Some(program_name.to_owned());",
            "if let Some(tptp_dir) = tptp_env_dir()",
            "state.tptp_dir = Some(tptp_dir);",
            "lock_io_state().tptp_dir = None;",
            "fn init_io_preserves_c_tptp_reinitialization_shape()",
            'assert_eq!(tptp_dir().as_deref(), Some("First/"));',
            'assert_eq!(error_program_name(), "second");',
        )
        and contains(
            repo,
            "experiments/2026-07-18-125-executable-diagnostic-owner/FINDINGS.md",
            "all 26 Cargo binaries",
            "Every binary then calls `report_fatal_diagnostic`",
            "one shared path",
        ),
        "tcp_message_debug_empty_and_nul_surfaces_are_safe_and_exact": contains(
            repo,
            "eprover/INOUT/cio_network.c",
            'printf("read(Size)=%d\\n", res);',
            'printf("Message expected with %d bytes\\n", len);',
            'printf("read(msg)=%d\\n", res);',
            "if(res ==0)",
            "buffer[len] = '\\0';",
            "DStrAppendStr(msg->content, buffer);",
        )
        and contains(
            repo,
            "src/inout/network.rs",
            'writeln!(trace, "read(Size)={count}")',
            'writeln!(trace, "Message expected with {len} bytes")',
            'writeln!(trace, "read(msg)={count}")',
            "extend_from_slice(c_string_prefix(&buffer[..read]))",
            "fn read_reports_c_empty_payload_as_closed_after_header()",
            "fn read_tracing_matches_c_debug_lines()",
            "fn read_accumulates_payload_with_c_string_truncation()",
        ),
        "server_socket_reuse_precedes_bind_on_supported_targets": contains(
            repo,
            "eprover/INOUT/cio_network.c",
            "setsockopt(sock, SOL_SOCKET, SO_REUSEADDR",
            "res = bind(sock, (struct sockaddr *)&addr, sizeof(addr));",
            "if(res == -1)",
        )
        and contains(
            repo,
            "src/inout/network.rs",
            "set_reuse_addr(fd)",
            ".and_then(|()| bind_any(fd, port))",
            "set_reuse_addr(socket)",
            ".and_then(|()| bind_any(socket, port))",
            "fn server_and_client_socket_wrappers_exchange_loopback_bytes()",
        )
        and contains(
            repo,
            "experiments/2026-07-17-041-network-socket-reconciliation/FINDINGS.md",
            "sets `SO_REUSEADDR`, binds to",
            "Linux and Windows",
            "real loopback regression",
        ),
        "global_output_descriptor_and_flush_boundaries_are_owned": contains(
            repo,
            "eprover/INOUT/cio_output.c",
            "int   GlobalOutFD = STDOUT_FILENO;",
            "GlobalOutFD = fileno(GlobalOut);",
            "fflush(file);",
            "if(ferror(file))",
            "if(file != stdout)",
            "if(fclose(file) != 0)",
        )
        and contains(
            repo,
            "src/inout/output.rs",
            "pub enum OutputDestination",
            "Self::Stdout => io::stdout().flush()",
            "Self::File(file) => file.flush()",
            "state.global_out_fd = fd;",
            "fn global_output_file_fd_writes_to_the_owned_target()",
            'assert_eq!(std::fs::read(&path).unwrap(), b"raw-owned");',
        )
        and contains(
            repo,
            "experiments/2026-07-17-043-global-output-fd-reconciliation/FINDINGS.md",
            "`GlobalOutFD` is a working",
            "same owning file",
            "exact `raw-owned` result",
        ),
        "scanner_compatibility_shims_are_narrow_and_regression_tested": contains(
            repo,
            "eprover/INOUT/cio_scanner.h",
            "#define TOKENREALPOS(pos) ((pos) % MAXTOKENLOOKAHEAD)",
            "#define LookToken(in,look)",
        )
        and contains(
            repo,
            "eprover/INOUT/cio_scanner.c",
            "handle->include_key = NULL;",
            "void PrintToken(FILE* out, Token_p token)",
            'StrTreeStore(name_selector, "** Not a legal name**",',
        )
        and contains(
            repo,
            "src/inout/scanner.rs",
            'pub const EMPTY_INCLUDE_SELECTOR_SENTINEL: &str = "** Not a legal name**";',
            "pub fn from_file_following_includes(",
            "pub fn look_token_c_modulo(&self, look: usize) -> &Token",
            "pub fn print_token(output: &mut impl io::Write, token: &Token)",
            "fn parse_include_honors_skip_tree_and_empty_selector_sentinel()",
            "fn include_key_splices_included_files_and_resumes_parent_stream()",
            "fn scanner_c_modulo_lookahead_aliases_ring_positions()",
        )
        and contains(
            repo,
            "experiments/2026-07-18-126-explicit-include-policy/FINDINGS.md",
            "Every supported production",
            "formula-owner path uses explicit include parsing",
            "only two `from_file_following_includes` references",
        )
        and contains(
            repo,
            "experiments/2026-07-16-048-formula-parser-record-include-parity/FINDINGS.md",
            "empty selector over THF declarations/formulas",
            "the owner sets are empty",
        ),
        "hard_timeout_output_and_windows_cooperation_preserve_diagnostics": contains(
            repo,
            "eprover/INOUT/cio_signals.c",
            'WriteStr(GlobalOutFD, "\\n"COMCHAR" Failure: Resource limit exceeded (time)\\n");',
            'TSTPOUTFD(GlobalOutFD, "ResourceOut");',
            'Error("CPU time limit exceeded, terminating", CPU_LIMIT_ERROR);',
        )
        and contains(
            repo,
            "src/inout/signals.rs",
            'b"\\n%% Failure: Resource limit exceeded (time)\\n%% SZS status ResourceOut\\n";',
            "let global_out_fd = signal_global_out_fd();",
            "write_fd_all(global_out_fd, HARD_CPU_TIMEOUT_OUTPUT);",
            "write_pending_output(global_out_fd);",
            '#[cfg(not(target_os = "linux"))]',
            "SYSTEM_TIME_LIMIT.store(RLIM_INFINITY_COMPAT, Ordering::SeqCst);",
            "fn cpu_limit_outcome_finalizer_writes_c_hard_timeout_shape()",
        )
        and contains(
            repo,
            "src/prover/eprover.rs",
            '#[cfg(any(test, not(target_os = "linux")))]',
            "let status = finalize_cpu_limit_outcome(&mut direct_output, &outcome)?",
            "output.write_direct_global_out(&direct_output)?;",
        )
        and contains(
            repo,
            "experiments/2026-07-17-040-signal-delivery-reconciliation/FINDINGS.md",
            "Windows Job Object quota would terminate the process",
            "before E could emit its banner",
            "same finalizer",
        ),
        "eager_stream_bytes_make_close_time_file_errors_unobservable": contains(
            repo,
            "eprover/INOUT/cio_streams.c",
            "void DestroyStream(Stream_p stream)",
            "if(stream->file != stdin)",
            "if(fclose(stream->file) != 0)",
            'sprintf(ErrStr, "Cannot close file %s", DStrView(stream->source));',
        )
        and contains(
            repo,
            "src/inout/streams.rs",
            "let data = fs::read(path).map_err",
            "data: Arc::new(data)",
            "fn file_content_stream_reuses_the_input_vector_allocation()",
            "fn cloned_streams_share_input_bytes_and_advance_independently()",
        )
        and contains(
            repo,
            "docs/rust-port-status.md",
            "Initial stream and scanner support for string sources",
            "safe ownership-based input close",
        ),
        "temporary_file_creation_preserves_security_and_lifecycle": contains(
            repo,
            "eprover/INOUT/cio_tempfile.c",
            'DStrAppendStr(name, "epr_XXXXXX");',
            "fd = mkstemp(DStrView(name));",
            "close(fd);",
            "TempFileRegister(res);",
        )
        and contains(
            repo,
            "src/inout/tempfile.rs",
            "options.write(true).create_new(true);",
            "options.mode(0o600);",
            "for _ in 0..TEMP_ATTEMPTS",
            "fn temp_file_name_creates_and_registers_file_under_tmpdir()",
        )
        and contains(
            repo,
            "experiments/2026-07-17-039-tempfile-ownership-reconciliation/FINDINGS.md",
            '`mkstemp("epr_XXXXXX")`',
            "1,024 atomic `create_new` attempts",
            "Both creation paths atomically reserve an empty file",
        ),
        "full_inout_and_port_compatibility_evidence_is_current": contains(
            repo,
            "docs/rust-port-status.md",
            "TCP message helpers from `cio_network`",
            "Output helpers from `cio_output`",
            "Signal-state helpers from `cio_signals`",
            "Temporary-file helpers from `cio_tempfile`",
        )
        and contains(
            repo,
            "experiments/2026-07-25-046-external-reconciliation/"
            "validation-reference.json",
            '"rust_test_count": 4429',
            '"main_unexpected_difference_count": 0',
            '"tool_unexpected_difference_count": 0',
        ),
    }

    source_files = [
        "eprover/INOUT/cio_commandline.c",
        "eprover/INOUT/cio_commandline.h",
        "eprover/INOUT/cio_fileops.c",
        "eprover/INOUT/cio_fileops.h",
        "eprover/INOUT/cio_filevars.c",
        "eprover/INOUT/cio_filevars.h",
        "eprover/INOUT/cio_initio.c",
        "eprover/INOUT/cio_initio.h",
        "eprover/INOUT/cio_network.c",
        "eprover/INOUT/cio_network.h",
        "eprover/INOUT/cio_output.c",
        "eprover/INOUT/cio_output.h",
        "eprover/INOUT/cio_scanner.c",
        "eprover/INOUT/cio_scanner.h",
        "eprover/INOUT/cio_signals.c",
        "eprover/INOUT/cio_signals.h",
        "eprover/INOUT/cio_streams.c",
        "eprover/INOUT/cio_streams.h",
        "eprover/INOUT/cio_tempfile.c",
        "eprover/INOUT/cio_tempfile.h",
        "src/inout/commandline.rs",
        "src/inout/fileops.rs",
        "src/inout/filevars.rs",
        "src/inout/initio.rs",
        "src/inout/network.rs",
        "src/inout/output.rs",
        "src/inout/scanner.rs",
        "src/inout/signals.rs",
        "src/inout/streams.rs",
        "src/inout/tempfile.rs",
        "src/prover/eprover.rs",
        "docs/rust-port-status.md",
        "experiments/2026-07-16-048-formula-parser-record-include-parity/"
        "FINDINGS.md",
        "experiments/2026-07-17-039-tempfile-ownership-reconciliation/"
        "FINDINGS.md",
        "experiments/2026-07-17-040-signal-delivery-reconciliation/"
        "FINDINGS.md",
        "experiments/2026-07-17-041-network-socket-reconciliation/"
        "FINDINGS.md",
        "experiments/2026-07-17-043-global-output-fd-reconciliation/"
        "FINDINGS.md",
        "experiments/2026-07-18-125-executable-diagnostic-owner/FINDINGS.md",
        "experiments/2026-07-18-126-explicit-include-policy/FINDINGS.md",
        "experiments/2026-07-25-046-external-reconciliation/"
        "validation-reference.json",
    ]
    source_digest = hashlib.sha256(
        b"".join((repo / relative).read_bytes() for relative in source_files)
    ).hexdigest()
    report = {
        "content_hashes_verified": sum(
            record["content_sha_matches"] is True for record in selected
        ),
        "decision_count": len(selected),
        "decision_digest": decision_digest,
        "evidence_checks": checks,
        "schema_version": 1,
        "source_digest": source_digest,
        "source_file_count": len(source_files),
    }
    encoded = json.dumps(report, indent=2, sort_keys=True) + "\n"
    sys.stdout.write(encoded)

    selected_ids = {record["id"] for record in selected}
    selected_are_inout = all(
        issues_by_id[record["id"]].get("metadata", {}).get("subsystem")
        == "inout"
        for record in selected
    )
    if (
        selected_ids != expected_ids
        or len(selected) != 19
        or report["content_hashes_verified"] != 19
        or not selected_are_inout
        or not all(checks.values())
    ):
        return 1
    if args.expected is not None:
        expected = args.expected.read_text(encoding="utf-8")
        if encoded != expected:
            print("INOUT reconciliation reference changed", file=sys.stderr)
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
