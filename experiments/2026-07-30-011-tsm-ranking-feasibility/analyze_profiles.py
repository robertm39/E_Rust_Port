#!/usr/bin/env python3
"""Analyze phase-isolated TSM native summaries and Callgrind profiles."""

from __future__ import annotations

import argparse
import json
import re
from collections import defaultdict
from pathlib import Path
from typing import Any, Sequence


FUNCTION_RE = re.compile(r"^(?:c?fn)=\((\d+)\)(?: (.*))?$")
COST_RE = re.compile(r"^\S+(?:\s+\S+)?\s+(\d+)$")
INTERESTING_PARTS = (
    "compute_in_bank",
    "PatternSubst",
    "pattern_clause",
    "pattern_term_compare",
    "index_term_order",
    "TSMIndex",
    "find_tsa_for_term",
    "tsm_eval_term",
    "malloc",
    "free",
    "memcpy",
    "memmove",
)


class AnalysisError(RuntimeError):
    """Raised when a profile is incomplete or malformed."""


def parse_callgrind(path: Path) -> dict[str, Any]:
    names: dict[str, str] = {}
    self_costs: dict[str, int] = defaultdict(int)
    inclusive_arcs: dict[str, int] = defaultdict(int)
    current_function: str | None = None
    current_callee: str | None = None
    call_cost_pending = False
    summary: int | None = None

    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if line.startswith("summary:"):
            summary = int(line.removeprefix("summary:").strip().split()[0])
            continue
        if line.startswith("totals:"):
            continue
        function_match = FUNCTION_RE.match(line)
        if function_match:
            identifier, name = function_match.groups()
            if name:
                names[identifier] = name
            if line.startswith("fn="):
                current_function = identifier
                current_callee = None
                call_cost_pending = False
            else:
                current_callee = identifier
                call_cost_pending = False
            continue
        if line.startswith("calls="):
            if current_callee is None:
                raise AnalysisError(f"call record without callee in {path}")
            call_cost_pending = True
            continue
        cost_match = COST_RE.match(line)
        if not cost_match:
            continue
        cost = int(cost_match.group(1))
        if call_cost_pending:
            if current_callee is None:
                raise AnalysisError(f"call cost without callee in {path}")
            inclusive_arcs[current_callee] += cost
            call_cost_pending = False
        elif current_function is not None:
            self_costs[current_function] += cost

    if summary is None:
        raise AnalysisError(f"missing instruction summary: {path}")

    def resolve(costs: dict[str, int]) -> list[dict[str, Any]]:
        return [
            {
                "function": names.get(identifier, f"<function-{identifier}>"),
                "instructions": cost,
                "program_fraction": cost / summary,
            }
            for identifier, cost in sorted(
                costs.items(), key=lambda item: (-item[1], item[0])
            )
        ]

    self_ranked = resolve(self_costs)
    inclusive_ranked = resolve(inclusive_arcs)
    interesting = [
        entry
        for entry in inclusive_ranked
        if any(part.lower() in entry["function"].lower() for part in INTERESTING_PARTS)
    ]
    return {
        "path": str(path),
        "instructions": summary,
        "top_self": self_ranked[:30],
        "top_inclusive": inclusive_ranked[:30],
        "interesting_inclusive": interesting,
    }


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--native-summary", type=Path, required=True)
    parser.add_argument("--callgrind-summary", type=Path, required=True)
    parser.add_argument("--search-control-profile", type=Path, required=True)
    parser.add_argument("--search-learned-profile", type=Path, required=True)
    parser.add_argument("--classifier-empty-profile", type=Path, required=True)
    parser.add_argument("--classifier-full-profile", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    native = json.loads(arguments.native_summary.read_text(encoding="utf-8"))
    callgrind_summary = json.loads(
        arguments.callgrind_summary.read_text(encoding="utf-8")
    )
    profiles = {
        "search_control": parse_callgrind(arguments.search_control_profile),
        "search_learned": parse_callgrind(arguments.search_learned_profile),
        "classifier_empty": parse_callgrind(arguments.classifier_empty_profile),
        "classifier_full": parse_callgrind(arguments.classifier_full_profile),
    }
    callgrind = callgrind_summary["callgrind"]
    for name, profile in profiles.items():
        expected = callgrind[name]["instructions"]
        if profile["instructions"] != expected:
            raise AnalysisError(f"{name} instruction total differs from summary")

    learned_delta = (
        profiles["search_learned"]["instructions"]
        - profiles["search_control"]["instructions"]
    )
    classifier_delta = (
        profiles["classifier_full"]["instructions"]
        - profiles["classifier_empty"]["instructions"]
    )
    weighted = native["inputs"]["weighted_validation_occurrences"]
    result = {
        "schema_version": 1,
        "native": {
            "classifier_microseconds_per_weighted_occurrence": native[
                "native_classifier"
            ]["median_microseconds_per_weighted_occurrence"],
            "search_learned_control_cpu_ratio": native["native_search"][
                "learned_control_cpu_ratio"
            ],
        },
        "callgrind": {
            "search_control_instructions": profiles["search_control"][
                "instructions"
            ],
            "search_learned_instructions": profiles["search_learned"][
                "instructions"
            ],
            "search_learned_only_instructions": learned_delta,
            "search_learned_control_instruction_ratio": (
                profiles["search_learned"]["instructions"]
                / profiles["search_control"]["instructions"]
            ),
            "classifier_empty_instructions": profiles["classifier_empty"][
                "instructions"
            ],
            "classifier_full_instructions": profiles["classifier_full"][
                "instructions"
            ],
            "classifier_scoring_instructions": classifier_delta,
            "classifier_scoring_instructions_per_weighted_occurrence": (
                classifier_delta / weighted
            ),
        },
        "profiles": profiles,
    }
    arguments.output.write_text(
        json.dumps(result, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AnalysisError as error:
        raise SystemExit(f"error: {error}") from error
