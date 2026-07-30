#!/usr/bin/env python3
"""Run the experiment's fail-closed and extraction test suite as JSON."""

from __future__ import annotations

import argparse
import io
import json
import time
import unittest
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    arguments = parser.parse_args()
    loader = unittest.TestLoader()
    suite = loader.loadTestsFromNames(["test_fm", "test_production"])
    names: list[str] = []

    def visit(value: unittest.TestSuite | unittest.TestCase) -> None:
        if isinstance(value, unittest.TestSuite):
            for child in value:
                visit(child)
        else:
            names.append(value.id())

    visit(suite)
    stream = io.StringIO()
    started = time.perf_counter_ns()
    result = unittest.TextTestRunner(
        stream=stream,
        verbosity=2,
    ).run(suite)
    report = {
        "success": result.wasSuccessful(),
        "tests_run": result.testsRun,
        "elapsed_ms": (time.perf_counter_ns() - started) / 1_000_000,
        "failures": [
            {"test": test.id(), "traceback": traceback}
            for test, traceback in result.failures
        ],
        "errors": [
            {"test": test.id(), "traceback": traceback}
            for test, traceback in result.errors
        ],
        "skipped": [
            {"test": test.id(), "reason": reason}
            for test, reason in result.skipped
        ],
        "test_ids": sorted(names),
        "categories": {
            "mutation_rejections": sum(".MutationTests." in name for name in names),
            "cancellation": sum("cancellation" in name for name in names),
            "bounds": sum("bound" in name for name in names),
            "timeout": sum("timeout" in name for name in names),
            "malformed": sum("malformed" in name for name in names),
            "production_extraction_and_rendering": sum(
                name.startswith("test_production.") for name in names
            ),
        },
        "text_output": stream.getvalue(),
    }
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    arguments.output.write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(
        json.dumps(
            {
                "success": report["success"],
                "tests_run": report["tests_run"],
                "categories": report["categories"],
            },
            indent=2,
            sort_keys=True,
        )
    )
    return 0 if result.wasSuccessful() else 1


if __name__ == "__main__":
    raise SystemExit(main())
