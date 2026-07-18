#!/usr/bin/env python3
"""Audit default-build comment-prefix ownership and retained parity evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path
from typing import Any


REFERENCE_COMMIT = "17026b1bfe61aaf223cfaae54947c8d2679c31a0"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--expected",
        type=Path,
        help="Fail unless the computed audit exactly matches this retained JSON file.",
    )
    return parser.parse_args()


def read_text(root: Path, relative: str) -> str:
    return (root / relative).read_text(encoding="utf-8")


def read_json(root: Path, relative: str) -> dict[str, Any]:
    return json.loads(read_text(root, relative))


def sha256(root: Path, relative: str) -> str:
    return hashlib.sha256((root / relative).read_bytes()).hexdigest()


def main() -> int:
    args = parse_args()
    root = Path(__file__).resolve().parents[2]
    c_defines = read_text(root, "eprover/BASICS/clb_defines.h")
    c_configure = read_text(root, "eprover/configure")
    rust_defines = read_text(root, "src/basics/defines.rs")
    rust_signals = read_text(root, "src/inout/signals.rs")
    rust_eprover = read_text(root, "src/prover/eprover.rs")
    deduction_findings = read_text(
        root,
        "experiments/2026-07-17-044-deduction-server-run-framing/FINDINGS.md",
    )

    format_path = "experiments/2026-07-18-104-format-option-integration/reference.json"
    proof_path = "experiments/2026-07-18-105-proof-output-integration/reference.json"
    reporting_path = (
        "experiments/2026-07-17-050-reporting-strategy-limit-matrix/"
        "results-summary.json"
    )
    format_reference = read_json(root, format_path)
    proof_reference = read_json(root, proof_path)
    reporting_reference = read_json(root, reporting_path)

    checks: list[dict[str, Any]] = []

    def check(name: str, condition: bool, detail: str) -> None:
        checks.append({"name": name, "passed": condition, "detail": detail})

    default_macro_block = (
        '#ifndef UNIX_COMMENTS\n'
        '// Doubled for printf...\n'
        '#define COMCHAR "%%"\n'
        '#define COMCHARRAW "%"'
    )
    unix_macro_block = (
        '#else\n#define COMCHAR "#"\n#define COMCHARRAW "#"\n#endif'
    )
    check(
        "c_default_macro_pair",
        default_macro_block in c_defines,
        'default C defines COMCHAR="%%" and COMCHARRAW="%"',
    )
    check(
        "c_optional_unix_macro_pair",
        unix_macro_block in c_defines,
        'UNIX_COMMENTS changes both C macros to "#"',
    )
    check(
        "c_unix_mode_is_build_configuration",
        '"--unix-comments"' in c_configure
        and "unix_comments=' -DUNIX_COMMENTS'" in c_configure,
        "C exposes the alternate prefix only through configure-time compilation",
    )
    check(
        "c_formatted_status_owner",
        '#define TSTPOUT(file,msg) fprintf(file, COMCHAR" SZS status %s\\n", msg)'
        in c_defines,
        "TSTPOUT passes COMCHAR through fprintf and renders one percent sign",
    )
    check(
        "c_direct_status_owner",
        'WriteStr(fd, COMCHAR" SZS status ");' in c_defines,
        "TSTPOUTFD writes COMCHAR directly and preserves two percent signs",
    )
    check(
        "rust_default_macro_pair",
        'pub const DEFAULT_COMCHAR_RAW: &str = "%";' in rust_defines
        and 'pub const DEFAULT_COMCHAR_DIRECT: &str = "%%";' in rust_defines,
        "Rust names the rendered and direct default-build spellings explicitly",
    )
    check(
        "rust_formatted_status_owner",
        "DEFAULT_COMCHAR_RAW,\n        c_string_prefix_str(status)" in rust_defines,
        "tstp_out_string owns the rendered single-percent status shape",
    )
    check(
        "rust_direct_status_owner",
        'DEFAULT_COMCHAR_DIRECT,\n        " SZS status ",' in rust_defines,
        "tstp_out_fd_chunks owns the literal double-percent descriptor shape",
    )

    direct_owner_files = sorted(
        path.relative_to(root).as_posix()
        for path in (root / "src").rglob("*.rs")
        if "DEFAULT_COMCHAR_DIRECT" in path.read_text(encoding="utf-8")
    )
    check(
        "rust_direct_owner_files_are_bounded",
        direct_owner_files == ["src/basics/defines.rs", "src/inout/signals.rs"],
        f"direct spelling owners: {', '.join(direct_owner_files)}",
    )
    check(
        "rust_hard_timeout_is_direct",
        "{DEFAULT_COMCHAR_DIRECT} Failure: Resource limit exceeded (time)"
        in rust_signals
        and "{DEFAULT_COMCHAR_DIRECT} SZS status ResourceOut" in rust_signals,
        "hard-timeout descriptor output uses the direct double-percent spelling",
    )
    check(
        "rust_eprover_stream_output_is_formatted",
        "DEFAULT_COMCHAR_RAW" in rust_eprover
        and "DEFAULT_COMCHAR_DIRECT" not in rust_eprover,
        "ordinary eprover stream output uses only the rendered single-percent spelling",
    )
    check(
        "rust_prefix_regression",
        "tstp_status_helpers_preserve_formatted_and_direct_comment_prefixes"
        in rust_defines
        and '"% SZS status Theorem\\n"' in rust_defines
        and '"%% SZS status ResourceOut\\n"' in rust_defines,
        "unit regression pins both status spellings",
    )

    rust_configuration_text = "\n".join(
        path.read_text(encoding="utf-8")
        for path in [root / "Cargo.toml", *(root / "src").rglob("*.rs")]
    ).lower()
    check(
        "rust_has_no_speculative_unix_mode",
        "unix_comments" not in rust_configuration_text
        and "unix-comments" not in rust_configuration_text,
        "Rust does not broaden its CLI or feature surface for an unsupported C build",
    )

    format_statuses = [
        case["status"]
        for implementation in ("c", "rust")
        for case in format_reference[implementation].values()
        if case["status"] is not None
    ]
    check(
        "format_matrix_is_exact",
        format_reference["all_exact"]
        and format_reference["all_effects_observed"]
        and format_reference["case_count"] == 18,
        "retained format-option matrix is 18/18 byte-exact with all effects observed",
    )
    check(
        "format_matrix_statuses_are_rendered",
        bool(format_statuses)
        and all(status.startswith("% SZS status ") for status in format_statuses),
        f"all {len(format_statuses)} retained non-null C/Rust statuses use one percent sign",
    )
    check(
        "proof_matrix_is_exact",
        proof_reference["all_exact"]
        and proof_reference["all_effects_observed"]
        and proof_reference["case_count"] == 15
        and proof_reference["upstream_commit"] == REFERENCE_COMMIT,
        "retained proof-output matrix is 15/15 byte-exact against pinned C",
    )
    reporting_results = reporting_reference["results"]
    check(
        "reporting_limit_matrix_is_exact",
        reporting_reference["reference_commit"] == REFERENCE_COMMIT
        and reporting_reference["case_count"] == 11
        and reporting_reference["exact_count"] == 11
        and all(result["exact_match"] for result in reporting_results),
        "retained reporting/limit matrix is 11/11 byte-exact against pinned C",
    )
    check(
        "default_c_direct_macro_bug_is_documented",
        'default `COMCHAR` is the printf-escaped\nstring `"%%"`' in deduction_findings
        and "searches for two percent signs" in deduction_findings
        and "`--unix-comments` build avoids the mismatch" in deduction_findings,
        "live deduction-server evidence records why direct ownership cannot be flattened",
    )

    passed_count = sum(check["passed"] for check in checks)
    result = {
        "schema_version": 1,
        "bead": "E_Rust_Port-j76.2.32",
        "reference_commit": REFERENCE_COMMIT,
        "decision": "support_default_c_comment_build_only",
        "retained_exact_cases": {
            "format_options": 18,
            "proof_outputs": 15,
            "reporting_and_limits": 11,
            "total": 44,
        },
        "input_sha256": {
            format_path: sha256(root, format_path),
            proof_path: sha256(root, proof_path),
            reporting_path: sha256(root, reporting_path),
        },
        "checks": checks,
        "passed_count": passed_count,
        "total_count": len(checks),
        "all_passed": passed_count == len(checks),
    }

    rendered = json.dumps(result, indent=2, sort_keys=True) + "\n"
    sys.stdout.write(rendered)
    if args.expected is not None:
        expected = json.loads(args.expected.read_text(encoding="utf-8"))
        if result != expected:
            print("computed audit differs from retained expectation", file=sys.stderr)
            return 1
    return 0 if result["all_passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
