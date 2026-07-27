#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$repo_root"

baseline=${1:-.artifacts/experiments/2026-07-14-010-pdt-variable-metadata-cache/baseline-eprover}
candidate=${2:-target/release/eprover}
artifact_dir=.artifacts/experiments/2026-07-14-010-pdt-variable-metadata-cache/proof-checks
mkdir -p "$artifact_dir"

run_one() {
    local label=$1
    local executable=$2
    local problem=$3
    /usr/bin/time -p -o "$artifact_dir/$label.time" "$executable" \
        --auto --print-statistics --cpu-limit=600 --memory-limit=2048 \
        --detsort-rw --detsort-new "$problem" >"$artifact_dir/$label.out"
    printf '%s\n' "$label"
    cat "$artifact_dir/$label.time"
}

hen=eprover/EXAMPLE_PROBLEMS/TPTP/HEN011-2.p
geo=eprover/EXAMPLE_PROBLEMS/TPTP/GEO288+1.p

run_one hen-baseline "$baseline" "$hen"
run_one hen-candidate "$candidate" "$hen"
run_one geo-baseline "$baseline" "$geo"
run_one geo-candidate "$candidate" "$geo"
