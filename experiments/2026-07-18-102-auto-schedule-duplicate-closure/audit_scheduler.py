#!/usr/bin/env python3
"""Audit scheduler state transfer, resource ownership, and prior decisions."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def load_issue(path: Path, issue_id: str) -> dict[str, object]:
    for line in path.read_text(encoding="utf-8").splitlines():
        issue = json.loads(line)
        if issue.get("_type") == "issue" and issue.get("id") == issue_id:
            return issue
    raise ValueError(f"missing issue {issue_id}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--reference", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()

    root = Path(__file__).resolve().parents[2]
    c_scheduler = (root / "eprover/CONTROL/cco_scheduling.c").read_text(
        encoding="utf-8"
    )
    c_driver = (root / "eprover/PROVER/eprover.c").read_text(encoding="utf-8")
    rust_driver = (root / "src/prover/eprover.rs").read_text(encoding="utf-8")
    rust_scheduler = (root / "src/control/scheduling.rs").read_text(encoding="utf-8")
    integration = (root / "tests/eprover_schedule.rs").read_text(encoding="utf-8")
    prior_findings = (
        root / "experiments/2026-07-16-034-multicore-fork-compatibility/FINDINGS.md"
    ).read_text(encoding="utf-8")
    prior_issue = load_issue(root / ".beads/issues.jsonl", "E_Rust_Port-j76.1.25")
    reference = json.loads(args.reference.read_text(encoding="utf-8"))

    checks = {
        "c_child_inherits_selected_schedule_state": all(
            marker in c_scheduler
            for marker in (
                "h_parms->heuristic_name = strats[i].heu_name;",
                "h_parms->order_params.ordertype = strats[i].ordering;",
                "SilentTimeOut = true;",
                "return i; // tells the other scheduling call what is the parent",
            )
        ),
        "c_nested_search_uses_parent_cell_budget_and_cores": all(
            marker in c_driver
            for marker in (
                "preproc_schedule[sched_idx].time_absolute",
                "preproc_schedule[sched_idx].cores",
            )
        ),
        "c_retry_uses_descendant_then_self_cpu_clocks": (
            "GetTotalCPUTimeIncludingChildren()" in c_driver
            and "double run_time = GetTotalCPUTime();" in c_scheduler
        ),
        "c_schedule_parent_prints_resource_footer": (
            "if(print_rusage)\n         {\n            PrintRusage(GlobalOut);" in c_scheduler
        ),
        "c_schedule_leaf_suppresses_final_resource_footer": (
            "if(print_rusage && !SilentTimeOut)" in c_driver
        ),
        "rust_private_protocol_transfers_preprocessing_budget": all(
            marker in rust_driver
            for marker in (
                "preprocessing_cpu_limit: u64",
                "preprocessing_cores: i32",
                "cell.time_absolute.to_string()",
                "cell.cores.to_string()",
            )
        ),
        "rust_rehydrates_parent_assigned_schedule_cell": all(
            marker in rust_driver
            for marker in (
                "selected.time_absolute = worker.preprocessing_cpu_limit;",
                "selected.cores = worker.preprocessing_cores.max(1);",
            )
        ),
        "rust_nested_search_uses_rehydrated_budget_and_cores": all(
            marker in rust_driver
            for marker in (
                "selected_preprocessing.time_absolute",
                "selected_preprocessing.cores.max(1)",
            )
        ),
        "rust_search_leaf_suppresses_resource_footer": (
            "schedule_worker_suppresses_resource_footer" in rust_driver
            and "matches!(worker.mode, InternalScheduleWorkerMode::Search { .. })"
            in rust_driver
        ),
        "rust_retry_preserves_two_cpu_clocks": all(
            marker in rust_scheduler
            for marker in (
                "config.wc_time_limit - usage.total_time",
                "time_used: usage.process_time",
            )
        ),
        "rust_parent_request_is_explicit": (
            "ScheduleExecutionOutcome::ParentRequest" in rust_scheduler
            and "parent_request_pending" in rust_scheduler
        ),
        "rust_snapshots_stdin_once_for_exec_workers": all(
            marker in rust_driver
            for marker in (
                "prepare_schedule_stdin_snapshot",
                "ScheduleStdinSnapshotGuard::install",
                "schedule_stdin_snapshot",
            )
        ),
        "integration_pins_full_search_schedule_budget": (
            "% Scheduled 6 strats onto 1 cores with 300 seconds (300 total)"
            in integration
        ),
        "integration_pins_two_resource_footers": (
            "assert_eq!(resource_positions.len(), 2" in integration
            and "assert_eq!(total_times.len(), 2" in integration
        ),
        "integration_pins_snapshotted_stdin": (
            "auto_schedule_replays_snapshotted_standard_input_in_nested_workers"
            in integration
        ),
        "integration_pins_cnf_nested_preprocessing_proof": (
            "auto_schedule_cnf_allows_nested_search_preprocessing_proof" in integration
            and "internal_search_selection(config).is_none()" in rust_driver
        ),
        "prior_safe_exec_state_transfer_decision_retained": all(
            marker in prior_findings
            for marker in (
                "Safe state-transfer decision",
                "portable implementation therefore keeps explicit exec workers",
                "named files are assumed stable",
            )
        ),
        "prior_duplicate_owner_is_closed": prior_issue.get("status") == "closed",
        "fresh_reference_projection_is_exact": reference.get("all_exact") is True,
    }
    report = {
        "schema_version": 1,
        "checks": checks,
        "passed": sum(checks.values()),
        "total": len(checks),
        "all_passed": all(checks.values()),
    }
    args.output.write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    if not report["all_passed"]:
        failed = [name for name, passed in checks.items() if not passed]
        print(f"scheduler audit failed: {', '.join(failed)}", file=sys.stderr)
        return 1
    print(f"validated {report['passed']}/{report['total']} scheduler checks")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
