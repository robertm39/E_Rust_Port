#!/usr/bin/env bash
set -eu

root=${ROOT:-/mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port}
baseline=${BASELINE_EXE:-/mnt/c/tmp/e-rust-port-baseline/target/release/eprover}
candidate=${CANDIDATE_EXE:-$root/target/release/eprover}
output=${OUTPUT:-$root/.artifacts/experiments/2026-07-11-006-post-cache-callgrind/candidate-timings.txt}
problem=$root/eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop

: > "$output"

run_one() {
    label=$1
    executable=$2
    /usr/bin/time -a -o "$output" -f "$label %U %S %e" \
        "$executable" --auto --silent --cpu-limit=600 \
        --detsort-rw --detsort-new "$problem" >/dev/null
}

for iteration in $(seq 1 10); do
    if ((iteration % 2)); then
        run_one "baseline-$iteration" "$baseline"
        run_one "candidate-$iteration" "$candidate"
    else
        run_one "candidate-$iteration" "$candidate"
        run_one "baseline-$iteration" "$baseline"
    fi
done

cat "$output"
