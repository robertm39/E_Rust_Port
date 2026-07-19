#!/usr/bin/env python3
"""Retain the stable reference archive and interop command inventory."""

from __future__ import annotations

import argparse
from collections import Counter
import importlib
import json
from pathlib import Path
import re
import sys
from typing import Any


EXPECTED_UPSTREAM_COMMIT = "17026b1bfe61aaf223cfaae54947c8d2679c31a0"
REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT / "tools" / "e-interop"))
e_interop = importlib.import_module("e_interop")


def driver_commands() -> list[str]:
    parser = e_interop.parser()
    for action in parser._actions:
        choices = getattr(action, "choices", None)
        if isinstance(choices, dict):
            return sorted(choices)
    raise SystemExit("could not find e-interop driver subcommands")


def wrapper_commands() -> list[str]:
    source = (REPO_ROOT / "e-interop.ps1").read_text(encoding="utf-8")
    match = re.search(r"\[ValidateSet\(([^)]+)\)\]", source)
    if match is None:
        raise SystemExit("could not find e-interop.ps1 command ValidateSet")
    return sorted(re.findall(r"'([^']+)'", match.group(1)))


def reference_build(build: dict[str, Any]) -> dict[str, Any]:
    return {
        "mode": build["mode"],
        "configure": build["configure"],
        "version": build["version"],
        "smoke_status": build["smoke_status"],
        "sha256": build["sha256"],
    }


def summarize(
    main_report: dict[str, Any], tool_report: dict[str, Any]
) -> dict[str, Any]:
    main_manifest = main_report["reference_manifest"]
    tool_manifest = tool_report["reference_manifest"]
    if main_manifest != tool_manifest:
        raise SystemExit("main and support-tool reports used different manifests")
    builds = main_manifest["builds"]
    archived_tools = builds["fol"]["tools"]
    return {
        "schema_version": 1,
        "upstream_commit": main_manifest["upstream_commit"],
        "distribution": main_manifest["distribution"],
        "reference_builds": {
            mode: reference_build(builds[mode]) for mode in ("fol", "ho")
        },
        "wrapper_commands": wrapper_commands(),
        "driver_commands": driver_commands(),
        "configured_tool_paths": dict(sorted(e_interop.REFERENCE_TOOL_BINARIES.items())),
        "archived_tool_names": sorted(archived_tools),
        "source_linked_tools": sorted(e_interop.ARCHIVED_REFERENCE_TOOL_LINKS),
        "source_patched_tools": sorted(
            e_interop.ARCHIVED_REFERENCE_TOOL_SOURCE_PATCHES
        ),
        "versioned_tool_count": len(e_interop.VERSIONED_REFERENCE_TOOLS),
        "functional_tool_names": sorted(e_interop.TOOL_FUNCTIONAL_CASES),
        "main_case_count": main_report["case_count"],
        "main_mode_counts": dict(
            sorted(Counter(case["mode"] for case in main_report["cases"]).items())
        ),
        "main_scenario_counts": dict(
            sorted(
                Counter(case["scenario"] for case in main_report["cases"]).items()
            )
        ),
        "support_tool_case_count": tool_report["case_count"],
        "support_tool_count": len({case["tool"] for case in tool_report["cases"]}),
    }


def validate(summary: dict[str, Any]) -> None:
    if summary["upstream_commit"] != EXPECTED_UPSTREAM_COMMIT:
        raise SystemExit("interop reports used the wrong archived C commit")
    if set(summary["reference_builds"]) != {"fol", "ho"}:
        raise SystemExit("FOL/HO reference inventory changed")
    if any(
        build["smoke_status"] != "Theorem"
        for build in summary["reference_builds"].values()
    ):
        raise SystemExit("an archived main reference failed its smoke test")
    configured_tools = set(summary["configured_tool_paths"])
    if len(configured_tools) != 25:
        raise SystemExit("configured support-tool inventory changed")
    if configured_tools != set(summary["archived_tool_names"]):
        raise SystemExit("reference manifest does not archive every configured tool")
    if configured_tools != set(summary["functional_tool_names"]):
        raise SystemExit("functional comparison does not cover every archived tool")
    if summary["source_linked_tools"] != ["termprops", "tsm_classify"]:
        raise SystemExit("commented-target source-link inventory changed")
    if summary["source_patched_tools"] != summary["source_linked_tools"]:
        raise SystemExit("source-linked tool patch inventory changed")
    if summary["wrapper_commands"] != [
        "benchmark",
        "build-reference",
        "compare",
        "compare-tools",
        "setup",
    ]:
        raise SystemExit("PowerShell interop command surface changed")
    if summary["driver_commands"] != [
        "benchmark",
        "build-reference",
        "compare",
        "compare-tools",
        "doctor",
    ]:
        raise SystemExit("Python interop command surface changed")
    if summary["main_case_count"] != 50:
        raise SystemExit("main comparison inventory changed")
    if summary["support_tool_case_count"] != 216:
        raise SystemExit("support-tool comparison inventory changed")
    if summary["support_tool_count"] != 25:
        raise SystemExit("support-tool report coverage changed")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--main-report", type=Path, required=True)
    parser.add_argument("--tool-report", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--expected", type=Path)
    args = parser.parse_args()

    main_report = json.loads(args.main_report.read_text(encoding="utf-8"))
    tool_report = json.loads(args.tool_report.read_text(encoding="utf-8"))
    summary = summarize(main_report, tool_report)
    validate(summary)

    if args.expected is not None:
        expected = json.loads(args.expected.read_text(encoding="utf-8"))
        if summary != expected:
            raise SystemExit("stable interop harness summary differs from retained evidence")

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )


if __name__ == "__main__":
    main()
