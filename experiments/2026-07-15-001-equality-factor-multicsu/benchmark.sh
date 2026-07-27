#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$repo_root"

repetitions=${1:-200}
samples=${2:-7}
artifact_dir=.artifacts/experiments/2026-07-15-001-equality-factor-multicsu
manifest=$HOME/.cache/e-rust-port/reference.json
input=$repo_root/experiments/2026-07-15-001-equality-factor-multicsu/input.p
output=$artifact_dir/alternating-times.tsv
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
        printf 'run .\\e-interop.ps1 benchmark -Corpus experiments\\2026-07-15-001-equality-factor-multicsu -Runs 3 first\n' >&2
        exit 2
    fi
done

arguments=(
    --unif-mode=multi
    --pattern-oracle=false
    --fixpoint-oracle=false
    --func-proj-limit=1
    --imit-limit=1
    --max-unifiers=4
    --max-unif-steps=32
    --silent
    --processed-clauses-limit=1
)

printf 'implementation\tsample\trepetitions\twall_seconds\n' >"$output"

run_batch() {
    local label=$1
    local sample=$2
    local executable=$3
    local expected_status=$4
    local start_ns end_ns elapsed status
    start_ns=$(date +%s%N)
    for ((run = 1; run <= repetitions; run++)); do
        set +e
        "$executable" "${arguments[@]}" "$input" >/dev/null 2>&1
        status=$?
        set -e
        if [[ $status -ne $expected_status ]]; then
            printf '%s sample %d run %d returned %d, expected %d\n' \
                "$label" "$sample" "$run" "$status" "$expected_status" >&2
            exit 1
        fi
    done
    end_ns=$(date +%s%N)
    elapsed=$(python3 - "$start_ns" "$end_ns" <<'PY'
import sys

print(f"{(int(sys.argv[2]) - int(sys.argv[1])) / 1_000_000_000:.9f}")
PY
)
    printf '%s\t%d\t%d\t%s\n' "$label" "$sample" "$repetitions" "$elapsed" >>"$output"
}

for ((sample = 1; sample <= samples; sample++)); do
    if ((sample % 2 == 1)); then
        run_batch c "$sample" "$c_binary" 9
        run_batch rust "$sample" "$rust_binary" 9
    else
        run_batch rust "$sample" "$rust_binary" 9
        run_batch c "$sample" "$c_binary" 9
    fi
done

python3 - "$output" <<'PY'
import csv
import statistics
import sys

with open(sys.argv[1], encoding="utf-8", newline="") as handle:
    rows = list(csv.DictReader(handle, delimiter="\t"))
grouped = {
    implementation: [float(row["wall_seconds"]) for row in rows if row["implementation"] == implementation]
    for implementation in ("c", "rust")
}
c_median = statistics.median(grouped["c"])
rust_median = statistics.median(grouped["rust"])
print(f"C median batch wall: {c_median:.6f} s")
print(f"Rust median batch wall: {rust_median:.6f} s")
print(f"Rust/C batch ratio: {rust_median / c_median:.3f}x")
PY
