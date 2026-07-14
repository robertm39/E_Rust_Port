#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$repo_root"

baseline=${1:-.artifacts/experiments/2026-07-14-003-pdt-query-reuse/baseline-eprover}
candidate=${2:-target/release/eprover}
output=${3:-.artifacts/experiments/2026-07-14-003-pdt-query-reuse/alternating-times.txt}
problem=eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop
tmp_time=$(mktemp)
trap 'rm -f "$tmp_time"' EXIT

: > "$output"

run_one() {
    local label=$1
    local executable=$2
    /usr/bin/time -p -o "$tmp_time" "$executable" \
        --auto --silent --cpu-limit=600 --memory-limit=2048 \
        --detsort-rw --detsort-new "$problem" >/dev/null
    printf '%s\n' "$label" >> "$output"
    cat "$tmp_time" >> "$output"
}

for iteration in 1 2 3 4 5 6 7; do
    if (( iteration % 2 == 1 )); then
        run_one "B-$iteration" "$baseline"
        run_one "C-$iteration" "$candidate"
    else
        run_one "C-$iteration" "$candidate"
        run_one "B-$iteration" "$baseline"
    fi
done

cat "$output"
