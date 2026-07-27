#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$repo_root"

experiment=2026-07-15-003-lfho-paramod-direct-mgu
artifact_dir=.artifacts/experiments/$experiment/trace
manifest=$HOME/.cache/e-rust-port/reference.json
mkdir -p "$artifact_dir"

mapfile -t binaries < <(python3 - "$manifest" <<'PY'
import json
import pathlib
import sys

manifest_path = pathlib.Path(sys.argv[1])
manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
commit = manifest["upstream_commit"]
print(manifest["builds"]["ho"]["binary"])
print(pathlib.Path.home() / ".cache" / "e-rust-port" / "rust-target" / commit / "release" / "eprover")
PY
)

c_binary=${binaries[0]}
rust_binary=${binaries[1]}
for binary in "$c_binary" "$rust_binary"; do
    if [[ ! -x "$binary" ]]; then
        printf 'missing executable: %s\n' "$binary" >&2
        exit 2
    fi
done

run_one() {
    local implementation=$1
    local executable=$2
    local ordering=$3
    local input=$4
    local case_name=$5
    local prefix=$artifact_dir/$case_name-$ordering-$implementation
    local status
    local arguments=(
        --term-ordering="$ordering"
        --literal-selection-strategy=NoSelection
        --pm-from-index=NoIndex
        --pm-into-index=NoIndex
        --processed-clauses-limit=2
        --output-level=2
        --print-statistics
    )

    set +e
    "$executable" "${arguments[@]}" "$input" >"$prefix.out" 2>"$prefix.err"
    status=$?
    set -e
    if [[ $status -ne 9 ]]; then
        printf '%s/%s/%s returned %d, expected processed-clause limit status 9\n' \
            "$case_name" "$ordering" "$implementation" "$status" >&2
        exit 1
    fi

    grep 'inference(' "$prefix.out" \
        | sed -E 's/c_0_[0-9]+/c_0_N/g' \
        >"$prefix-inferences.normalized"
    grep -E '^% (Processed clauses|Generated clauses|Paramodulations)' "$prefix.out" \
        >"$prefix-counts.txt"
}

orderings=(KBO KBO6 LPO LPOCopy LPO4 LPO4Copy)

for input in "$repo_root"/experiments/$experiment/*.p; do
    case_name=$(basename "$input" .p)
    for ordering in "${orderings[@]}"; do
        run_one c "$c_binary" "$ordering" "$input" "$case_name"
        run_one rust "$rust_binary" "$ordering" "$input" "$case_name"
        diff -u \
            "$artifact_dir/$case_name-$ordering-c-inferences.normalized" \
            "$artifact_dir/$case_name-$ordering-rust-inferences.normalized"
        diff -u \
            "$artifact_dir/$case_name-$ordering-c-counts.txt" \
            "$artifact_dir/$case_name-$ordering-rust-counts.txt"
        grep -Eq '^% Paramodulations[[:space:]]*:[[:space:]]*[1-9][0-9]*$' \
            "$artifact_dir/$case_name-$ordering-rust-counts.txt"
        printf '%s / %s: exact inference and counter match\n' \
            "$case_name" "$ordering"
    done
done

printf 'validated %d C/Rust LFHO paramodulation configurations\n' \
    "$(( ${#orderings[@]} * 3 ))"
