#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$repo_root"

artifact_dir=.artifacts/experiments/2026-07-16-066-main-eprover-profile
baseline=${1:-$artifact_dir/baseline-eprover}
candidate=${2:-target/release/eprover}
output=${3:-$artifact_dir/hen-prefix-times.txt}
problem=eprover/EXAMPLE_PROBLEMS/TPTP/HEN011-2.p
tmp_time=$(mktemp)
trap 'rm -f "$tmp_time"' EXIT

: > "$output"

run_one() {
    local label=$1
    local executable=$2
    local status=0
    /usr/bin/time -f 'user %U\nwall %e\nrss_kib %M' -o "$tmp_time" \
        "$executable" --auto --silent --cpu-limit=600 --memory-limit=2048 \
        --processed-clauses-limit=50000 --detsort-rw --detsort-new \
        "$problem" >/dev/null || status=$?
    if [[ $status -ne 9 ]]; then
        printf 'unexpected exit status %d for %s\n' "$status" "$label" >&2
        exit 1
    fi
    printf '%s\n' "$label" >> "$output"
    sed '/^$/d' "$tmp_time" >> "$output"
}

for iteration in 1 2 3 4 5; do
    if (( iteration % 2 == 1 )); then
        run_one "B-$iteration" "$baseline"
        run_one "C-$iteration" "$candidate"
    else
        run_one "C-$iteration" "$candidate"
        run_one "B-$iteration" "$baseline"
    fi
done

cat "$output"
