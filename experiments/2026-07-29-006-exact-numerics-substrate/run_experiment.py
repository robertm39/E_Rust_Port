#!/usr/bin/env python3
"""Generate, validate, and measure the exact-numerics backend candidates."""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import math
import os
import pathlib
import platform
import random
import re
import statistics
import subprocess
import sys
import time
from collections.abc import Iterable
from fractions import Fraction
from typing import Any

FNV_OFFSET = 0xCBF29CE484222325
FNV_PRIME = 0x100000001B3
WORKLOAD_ITERATIONS = {
    "paper": 500,
    "small": 80,
    "medium": 12,
    "large": 2,
}
WORKLOAD_SIZES = {
    "paper": 96,
    "small": 2_048,
    "medium": 512,
    "large": 64,
}
RATIONAL_PATTERN = re.compile(r"(?<![\w.])([+-]?\d+)\s*/\s*([+-]?\d+)")


def sha256_file(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def fnv_update(state: int, text: str) -> int:
    for byte in text.encode("ascii"):
        state ^= byte
        state = state * FNV_PRIME & ((1 << 64) - 1)
    return state


def canonical(value: Fraction) -> str:
    return f"{value.numerator}/{value.denominator}"


def digest_cases(cases: Iterable[tuple[Fraction, Fraction]]) -> str:
    state = FNV_OFFSET
    for left, right in cases:
        ordering = Fraction((left > right) - (left < right))
        values = (
            left,
            right,
            left + right,
            left - right,
            left * right,
            left / right,
            Fraction(math.floor(left)),
            Fraction(math.ceil(left)),
            ordering,
        )
        for value in values:
            state = fnv_update(state, canonical(value))
            state = fnv_update(state, "\n")
    return f"{state:016x}"


def signed_random(rng: random.Random, bits: int, *, nonzero: bool) -> int:
    while True:
        value = rng.getrandbits(bits)
        if rng.getrandbits(1):
            value = -value
        if value != 0 or not nonzero:
            return value


def generated_cases(
    rng: random.Random,
    *,
    count: int,
    numerator_bits: int,
    denominator_bits: int,
) -> list[tuple[Fraction, Fraction]]:
    cases = []
    for _ in range(count):
        left = Fraction(
            signed_random(rng, numerator_bits, nonzero=False),
            abs(signed_random(rng, denominator_bits, nonzero=True)),
        )
        right = Fraction(
            signed_random(rng, numerator_bits, nonzero=True),
            abs(signed_random(rng, denominator_bits, nonzero=True)),
        )
        cases.append((left, right))
    return cases


def paper_constants(viras_docs: pathlib.Path) -> tuple[list[Fraction], list[str]]:
    constants = {
        Fraction(-2),
        Fraction(-1),
        Fraction(-3, 4),
        Fraction(-2, 3),
        Fraction(-1, 2),
        Fraction(-1, 3),
        Fraction(0),
        Fraction(1, 3),
        Fraction(1, 2),
        Fraction(2, 3),
        Fraction(3, 4),
        Fraction(1),
        Fraction(2),
    }
    sources = []
    for path in sorted(viras_docs.glob("*.md")):
        sources.append(str(path))
        text = path.read_text(encoding="utf-8")
        for numerator, denominator in RATIONAL_PATTERN.findall(text):
            if int(denominator) != 0:
                constants.add(Fraction(int(numerator), int(denominator)))
    return sorted(constants), sources


def make_workloads(
    *,
    seed: int,
    viras_docs: pathlib.Path,
) -> tuple[dict[str, list[tuple[Fraction, Fraction]]], list[str]]:
    constants, sources = paper_constants(viras_docs)
    nonzero = [value for value in constants if value]
    paper = []
    for index in range(WORKLOAD_SIZES["paper"]):
        left = constants[index % len(constants)]
        right = nonzero[(index * 7 + 3) % len(nonzero)]
        paper.append((left, right))
    rng = random.Random(seed)
    return (
        {
            "paper": paper,
            "small": generated_cases(
                rng,
                count=WORKLOAD_SIZES["small"],
                numerator_bits=64,
                denominator_bits=48,
            ),
            "medium": generated_cases(
                rng,
                count=WORKLOAD_SIZES["medium"],
                numerator_bits=384,
                denominator_bits=256,
            ),
            "large": generated_cases(
                rng,
                count=WORKLOAD_SIZES["large"],
                numerator_bits=2_048,
                denominator_bits=1_536,
            ),
        },
        sources,
    )


def write_vectors(
    path: pathlib.Path,
    workloads: dict[str, list[tuple[Fraction, Fraction]]],
) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="ascii", newline="\n") as stream:
        stream.write(
            "# workload|left numerator|left denominator|"
            "right numerator|right denominator\n"
        )
        for workload in ("large", "medium", "paper", "small"):
            for left, right in workloads[workload]:
                stream.write(
                    f"{workload}|{left.numerator}|{left.denominator}|"
                    f"{right.numerator}|{right.denominator}\n"
                )


def percentile(values: list[int], fraction: float) -> int:
    ordered = sorted(values)
    index = max(0, math.ceil(fraction * len(ordered)) - 1)
    return ordered[index]


def parse_maximum_rss(path: pathlib.Path) -> int:
    for line in path.read_text(encoding="utf-8").splitlines():
        if "Maximum resident set size (kbytes):" in line:
            return int(line.rsplit(":", 1)[1].strip())
    raise RuntimeError(f"maximum RSS missing from {path}")


def run_backend(
    *,
    name: str,
    executable: pathlib.Path,
    vectors: pathlib.Path,
    expected: dict[str, str],
    runs: int,
    raw_dir: pathlib.Path,
) -> dict[str, Any]:
    samples: dict[str, list[int]] = {workload: [] for workload in expected}
    rss_samples = []
    outputs = []
    for run_index in range(runs):
        time_path = raw_dir / f"{name}-run-{run_index + 1}.time.txt"
        command = [
            "/usr/bin/time",
            "-v",
            "-o",
            str(time_path),
            str(executable),
            str(vectors),
        ]
        started = time.monotonic_ns()
        completed = subprocess.run(
            command,
            check=False,
            capture_output=True,
            text=True,
            timeout=900,
        )
        wall_ns = time.monotonic_ns() - started
        (raw_dir / f"{name}-run-{run_index + 1}.stdout").write_text(
            completed.stdout,
            encoding="utf-8",
        )
        (raw_dir / f"{name}-run-{run_index + 1}.stderr").write_text(
            completed.stderr,
            encoding="utf-8",
        )
        if completed.returncode != 0:
            raise RuntimeError(
                f"{name} run {run_index + 1} exited {completed.returncode}"
            )
        payload = json.loads(completed.stdout)
        if payload["backend"] != name:
            raise RuntimeError(
                f"{name} binary identified itself as {payload['backend']}"
            )
        rows = {row["name"]: row for row in payload["workloads"]}
        if set(rows) != set(expected):
            raise RuntimeError(f"{name} returned unexpected workload set")
        for workload, digest in expected.items():
            if rows[workload]["digest"] != digest:
                mismatch = {
                    "backend": name,
                    "run": run_index + 1,
                    "workload": workload,
                    "actual_digest": rows[workload]["digest"],
                    "expected_digest": digest,
                }
                (
                    raw_dir
                    / f"{name}-run-{run_index + 1}-mismatch.json"
                ).write_text(
                    json.dumps(mismatch, indent=2, sort_keys=True) + "\n",
                    encoding="utf-8",
                )
                raise RuntimeError(
                    f"{name}/{workload} digest {rows[workload]['digest']} "
                    f"does not match oracle {digest}"
                )
            if rows[workload]["cases"] != WORKLOAD_SIZES[workload]:
                raise RuntimeError(f"{name}/{workload} case count changed")
            if rows[workload]["iterations"] != WORKLOAD_ITERATIONS[workload]:
                raise RuntimeError(f"{name}/{workload} iteration count changed")
            samples[workload].append(int(rows[workload]["elapsed_ns"]))
        rss_samples.append(parse_maximum_rss(time_path))
        outputs.append(
            {
                "run": run_index + 1,
                "wall_ns": wall_ns,
                "payload": payload,
            }
        )
    summary = {}
    for workload, timings in samples.items():
        operations = (
            WORKLOAD_SIZES[workload]
            * WORKLOAD_ITERATIONS[workload]
            * 7
        )
        median_ns = int(statistics.median(timings))
        summary[workload] = {
            "cases": WORKLOAD_SIZES[workload],
            "iterations": WORKLOAD_ITERATIONS[workload],
            "operations": operations,
            "samples_ns": timings,
            "minimum_ns": min(timings),
            "median_ns": median_ns,
            "p95_ns": percentile(timings, 0.95),
            "median_ns_per_operation": median_ns / operations,
            "digest": expected[workload],
        }
    return {
        "name": name,
        "executable": str(executable),
        "executable_bytes": executable.stat().st_size,
        "executable_sha256": sha256_file(executable),
        "runs": runs,
        "maximum_rss_kib_samples": rss_samples,
        "median_maximum_rss_kib": int(statistics.median(rss_samples)),
        "workloads": summary,
        "raw_runs": outputs,
    }


def cargo_metadata(
    *,
    cargo: pathlib.Path,
    manifest: pathlib.Path,
    raw_dir: pathlib.Path,
) -> dict[str, Any]:
    command = [
        str(cargo),
        "metadata",
        "--manifest-path",
        str(manifest),
        "--locked",
        "--format-version",
        "1",
    ]
    completed = subprocess.run(
        command,
        check=False,
        capture_output=True,
        text=True,
        timeout=300,
    )
    (raw_dir / "cargo-metadata.stderr").write_text(
        completed.stderr,
        encoding="utf-8",
    )
    if completed.returncode != 0:
        raise RuntimeError(
            f"cargo metadata exited {completed.returncode}"
        )
    metadata = json.loads(completed.stdout)
    (raw_dir / "cargo-metadata.json").write_text(
        json.dumps(metadata, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return metadata


def cargo_license_inventory(metadata: dict[str, Any]) -> list[dict[str, Any]]:
    packages = []
    for package in metadata["packages"]:
        if package["name"] == "umlaut-exact-numerics-study":
            continue
        packages.append(
            {
                "name": package["name"],
                "version": package["version"],
                "license": package.get("license"),
                "license_file": package.get("license_file"),
                "source": package.get("source"),
                "repository": package.get("repository"),
            }
        )
    return sorted(packages, key=lambda item: (item["name"], item["version"]))


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--viras-docs", type=pathlib.Path, required=True)
    parser.add_argument("--cargo", type=pathlib.Path, default="cargo")
    parser.add_argument("--cargo-manifest", type=pathlib.Path, required=True)
    parser.add_argument("--num-bin", type=pathlib.Path, required=True)
    parser.add_argument("--dashu-bin", type=pathlib.Path, required=True)
    parser.add_argument("--rug-bin", type=pathlib.Path, required=True)
    parser.add_argument("--mini-bin", type=pathlib.Path, required=True)
    parser.add_argument("--mini-gmp-c", type=pathlib.Path, required=True)
    parser.add_argument("--mini-mpq-c", type=pathlib.Path, required=True)
    parser.add_argument("--work-dir", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    parser.add_argument("--seed", type=lambda value: int(value, 0), default=0x5A172027)
    parser.add_argument("--runs", type=int, default=7)
    parser.add_argument(
        "--corrupt-oracle",
        choices=sorted(WORKLOAD_SIZES),
        help="flip one oracle digest bit to test fail-closed behavior",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.runs < 3:
        raise SystemExit("--runs must be at least 3")
    args.work_dir.mkdir(parents=True, exist_ok=True)
    raw_dir = args.work_dir / "raw"
    raw_dir.mkdir(parents=True, exist_ok=True)
    workloads, sources = make_workloads(
        seed=args.seed,
        viras_docs=args.viras_docs,
    )
    vectors = args.work_dir / "vectors.txt"
    write_vectors(vectors, workloads)
    expected = {
        name: digest_cases(cases) for name, cases in workloads.items()
    }
    if args.corrupt_oracle:
        original = expected[args.corrupt_oracle]
        expected[args.corrupt_oracle] = (
            f"{int(original, 16) ^ 1:016x}"
        )
        print(
            f"intentionally corrupted {args.corrupt_oracle} oracle "
            f"digest {original} -> {expected[args.corrupt_oracle]}",
            file=sys.stderr,
        )
    metadata = cargo_metadata(
        cargo=args.cargo,
        manifest=args.cargo_manifest,
        raw_dir=raw_dir,
    )
    backends = (
        (
            "num-rational-0.4.2",
            args.num_bin,
        ),
        (
            "dashu-ratio-0.5.1",
            args.dashu_bin,
        ),
        (
            "rug-1.30.0-full-gmp-ffi",
            args.rug_bin,
        ),
        (
            "mini-gmp-6.3.0",
            args.mini_bin,
        ),
    )
    results = []
    for name, executable in backends:
        results.append(
            run_backend(
                name=name,
                executable=executable,
                vectors=vectors,
                expected=expected,
                runs=args.runs,
                raw_dir=raw_dir,
            )
        )
    report = {
        "schema_version": 1,
        "bead": "E_Rust_Port-9jt.5.1",
        "created_at": dt.datetime.now(dt.timezone.utc).isoformat(),
        "passed": True,
        "configuration": {
            "seed": args.seed,
            "seed_hex": hex(args.seed),
            "runs": args.runs,
            "workload_sizes": WORKLOAD_SIZES,
            "workload_iterations": WORKLOAD_ITERATIONS,
            "operations_per_case": 7,
        },
        "environment": {
            "platform": platform.platform(),
            "python": sys.version,
            "processor": platform.processor(),
        },
        "independence": {
            "oracle": "Python fractions.Fraction",
            "imports_umlaut": False,
            "backend_package_is_repository_root_package": False,
        },
        "vectors": {
            "path": str(vectors),
            "bytes": vectors.stat().st_size,
            "sha256": sha256_file(vectors),
            "paper_sources": sources,
            "expected_digests": expected,
        },
        "mini_gmp_sources": [
            {
                "path": str(path),
                "bytes": path.stat().st_size,
                "sha256": sha256_file(path),
            }
            for path in (args.mini_gmp_c, args.mini_mpq_c)
        ],
        "cargo_license_inventory": cargo_license_inventory(metadata),
        "backends": results,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(
        json.dumps(
            {
                "output": str(args.output),
                "passed": True,
                "backends": [result["name"] for result in results],
                "digests": expected,
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
