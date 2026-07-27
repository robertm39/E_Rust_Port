#!/usr/bin/env python3
"""Audit the OS-wrapper and optional performance-counter ownership boundary."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any


EXPECTED_COUNTERS = [
    "MguTimer",
    "SatTimer",
    "ParamodTimer",
    "PMIndexTimer",
    "IndexUnifTimer",
    "BWRWTimer",
    "BWRWIndexTimer",
    "IndexMatchTimer",
    "FreqVecTimer",
    "FVIndexTimer",
    "SubsumeTimer",
    "SetSubsumeTimer",
    "ClauseEvalTimer",
]
REMOVED_RUST_COUNTERS = [
    "GenerateTimer",
    "ForwardModifyTimer",
    "InsertNewTimer",
    "ForwardRewriteTimer",
    "OrientTimer",
    "SimplifyReflectTimer",
    "SelectionTimer",
]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--expected", type=Path)
    return parser.parse_args()


def source(repo: Path, relative: str) -> str:
    return (repo / relative).read_text(encoding="utf-8")


def rust_counter_names(perf: str) -> list[str]:
    all_counters = re.search(
        r"const ALL_COUNTERS:.*?= \[(.*?)\];", perf, re.DOTALL
    )
    if all_counters is None:
        return []
    variants = re.findall(r"PerfCounter::([A-Za-z0-9_]+)", all_counters.group(1))
    names = dict(re.findall(r'Self::([A-Za-z0-9_]+) => "([^"]+)"', perf))
    return [names.get(variant, f"<missing:{variant}>") for variant in variants]


def secure_fopen_callers(repo: Path) -> list[dict[str, str]]:
    callers = []
    for path in (repo / "eprover").rglob("*.c"):
        if path.name == "clb_os_wrapper.c":
            continue
        text = path.read_text(encoding="utf-8")
        for match in re.finditer(r'SecureFOpen\(([^,]+),\s*"([^"]+)"\)', text):
            callers.append(
                {
                    "path": path.relative_to(repo).as_posix(),
                    "mode": match.group(2),
                }
            )
    return sorted(callers, key=lambda caller: caller["path"])


def collect(repo: Path) -> dict[str, Any]:
    c_eprover = source(repo, "eprover/PROVER/eprover.c")
    rust_perf = source(repo, "src/basics/perf_counters.rs")
    rust_os = source(repo, "src/basics/os_wrapper.rs")
    rust_sat = source(repo, "src/clauses/satinterface.rs")
    rust_eprover = source(repo, "src/prover/eprover.rs")
    rust_proofcontrol = source(repo, "src/heuristics/proofcontrol.rs")
    c_printed = re.findall(r"PERF_CTR_PRINT\(GlobalOut,\s*([A-Za-z0-9_]+)\)", c_eprover)
    rust_printed = rust_counter_names(rust_perf)
    c_fopen_callers = secure_fopen_callers(repo)
    removed_surface = "\n".join(
        [rust_perf, rust_sat, rust_eprover, rust_proofcontrol]
    )
    checks = {
        "c_prints_exact_counter_surface": c_printed == EXPECTED_COUNTERS,
        "rust_prints_exact_counter_surface": rust_printed == EXPECTED_COUNTERS,
        "rust_has_no_extra_legacy_counter_names": not any(
            name in removed_surface for name in REMOVED_RUST_COUNTERS
        ),
        "sat_timer_wraps_main_saturation_owner": (
            "PerfCounter::SatTimer" in rust_eprover
            and "run_main_saturation" in rust_eprover
            and "PerfCounter::SatTimer" not in rust_sat
        ),
        "perf_guards_use_process_cpu_clock_and_single_start_slots": (
            "use crate::basics::os_wrapper::get_usec_clock;" in rust_perf
            and "fn counter_start_cell" in rust_perf
            and ".swap(0, Ordering::Relaxed)" in rust_perf
            and "Instant" not in rust_perf
        ),
        "windows_job_object_is_memory_only": (
            "JOB_OBJECT_LIMIT_PROCESS_MEMORY" in rust_os
            and "JOB_OBJECT_LIMIT_PROCESS_TIME" not in rust_os
            and "set_hard_cpu_limit" not in rust_os
        ),
        "linux_proc_fallback_uses_runtime_clock_rate": (
            "SC_CLK_TCK_COMPAT" in rust_os
            and "clock_ticks_per_second()?" in rust_os
            and "linux_ticks_to_seconds(user_ticks, ticks_per_second)" in rust_os
        ),
        "linux_resource_usage_prefers_getrusage_with_proc_fallback": (
            "linux_getrusage_resource_usage()" in rust_os
            and 'read_to_string("/proc/self/stat")' in rust_os
            and 'read_to_string("/proc/self/status")' in rust_os
        ),
        "duplicated_rlimit_data_branch_is_retained": (
            rust_os.count("set_soft_rlimit(RLIMIT_DATA_COMPAT, mem_limit)") == 2
        ),
        "all_direct_c_secure_fopen_callers_use_write_mode": (
            len(c_fopen_callers) == 3
            and all(caller["mode"] == "w" for caller in c_fopen_callers)
            and "b'w' =>" in rust_os
        ),
    }
    return {
        "schema_version": 1,
        "expected_counter_names": EXPECTED_COUNTERS,
        "c_counter_names": c_printed,
        "rust_counter_names": rust_printed,
        "c_secure_fopen_callers": c_fopen_callers,
        "checks": checks,
        "accepted": all(checks.values()),
    }


def main() -> int:
    args = parse_args()
    repo = Path(__file__).resolve().parents[2]
    result = collect(repo)
    rendered = json.dumps(result, indent=2, sort_keys=True) + "\n"
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(rendered, encoding="utf-8")
    if args.expected is not None and rendered != args.expected.read_text(encoding="utf-8"):
        print(f"OS-wrapper audit mismatch: {args.output} != {args.expected}")
        return 1
    print(f"OS-wrapper audit: accepted={result['accepted']}")
    return 0 if result["accepted"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
