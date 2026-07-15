#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$repo_root"

baseline=${1:-.artifacts/baseline-eta-6b70ba92/target/release/eprover}
candidate=${2:-target/release/eprover}
artifact_dir=.artifacts/experiments/2026-07-14-011-pdt-eta-normalization
problem=eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop

mkdir -p "$artifact_dir"

run_one() {
    local label=$1
    local executable=$2
    valgrind --tool=callgrind \
        --callgrind-out-file="$artifact_dir/callgrind-$label.out" \
        "$executable" --auto --silent --cpu-limit=600 --memory-limit=2048 \
        --detsort-rw --detsort-new "$problem" >/dev/null \
        2>"$artifact_dir/callgrind-$label.log"
    callgrind_annotate --inclusive=yes --threshold=0.1 \
        "$artifact_dir/callgrind-$label.out" \
        >"$artifact_dir/callgrind-$label.txt"
}

run_one baseline "$baseline"
run_one candidate "$candidate"

printf 'baseline '
awk '/summary:/ { print $2 }' "$artifact_dir/callgrind-baseline.out"
printf 'candidate '
awk '/summary:/ { print $2 }' "$artifact_dir/callgrind-candidate.out"

grep -F 'normalize_pd_tree_term' \
    "$artifact_dir/callgrind-baseline.txt" \
    "$artifact_dir/callgrind-candidate.txt" || true
