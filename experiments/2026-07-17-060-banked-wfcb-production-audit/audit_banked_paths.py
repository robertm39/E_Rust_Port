#!/usr/bin/env python3
"""Audit immutable and banked WFCB/HCB production call sites."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


def production_lines(path: Path) -> list[tuple[int, str]]:
    lines = path.read_text(encoding="utf-8").splitlines()
    for index, line in enumerate(lines):
        if line.strip() == "#[cfg(test)]":
            lines = lines[:index]
            break
    return [
        (number, line.strip())
        for number, line in enumerate(lines, start=1)
        if not line.strip().startswith("//")
    ]


def find_calls(
    sources: dict[Path, list[tuple[int, str]]],
    needle: str,
    excluded: set[Path],
) -> list[dict[str, object]]:
    return [
        {"file": path.as_posix(), "line": number, "source": line}
        for path, lines in sources.items()
        if path not in excluded
        for number, line in lines
        if needle in line
    ]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=Path(__file__).resolve().parents[2], type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--quiet", action="store_true")
    args = parser.parse_args()

    root = args.root.resolve()
    source_root = root / "src"
    sources = {
        path.relative_to(root): production_lines(path)
        for path in sorted(source_root.rglob("*.rs"))
    }
    wfcb = Path("src/heuristics/wfcb.rs")
    hcb = Path("src/heuristics/hcb.rs")
    proofcontrol = Path("src/heuristics/proofcontrol.rs")

    forbidden = {
        "direct_compute_eval_outside_wfcb_adapter": find_calls(
            sources, ".compute_eval(", {wfcb}
        ),
        "immutable_add_evaluation_outside_wfcb_hcb_adapters": find_calls(
            sources, ".add_evaluation(", {wfcb, hcb}
        ),
        "immutable_hcb_clause_evaluate_outside_hcb_adapter": find_calls(
            sources, "hcb_clause_evaluate(", {hcb}
        ),
        "immutable_hcb_set_reweight_outside_hcb_proofcontrol_adapters": find_calls(
            sources, "hcb_clause_set_reweight(", {hcb, proofcontrol}
        ),
        "immutable_proof_control_reweight_outside_its_adapter_module": find_calls(
            sources, "proof_control_clause_set_reweight(", {proofcontrol}
        ),
    }
    proofcontrol_only = {proofcontrol: sources[proofcontrol]}
    banked = {
        "hcb_clause_evaluate_with_bank": find_calls(
            proofcontrol_only, "hcb_clause_evaluate_with_bank(", set()
        ),
        "hcb_clause_set_reweight_with_bank": find_calls(
            proofcontrol_only, "hcb_clause_set_reweight_with_bank(", set()
        ),
        "proof_control_clause_set_reweight_with_bank": find_calls(
            proofcontrol_only,
            "proof_control_clause_set_reweight_with_bank(",
            set(),
        ),
    }
    forbidden_count = sum(len(calls) for calls in forbidden.values())
    banked_call_count = sum(
        sum(not call["source"].startswith("pub fn ") for call in calls)
        for calls in banked.values()
    )
    result = {
        "rust_source_files": len(sources),
        "forbidden_immutable_call_count": forbidden_count,
        "forbidden_immutable_calls": forbidden,
        "proof_control_banked_call_count": banked_call_count,
        "proof_control_banked_calls": banked,
    }
    args.output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    if not args.quiet:
        print(json.dumps(result, indent=2))
    if forbidden_count != 0 or banked_call_count < 8:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
