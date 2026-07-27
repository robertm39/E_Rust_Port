#!/usr/bin/env bash
set -Eeuo pipefail

if (($# != 2)); then
    echo "usage: remote_repeat.sh SOURCE_ROOT ARTIFACT_ROOT" >&2
    exit 64
fi

source_root=$1
artifact_root=$2
measure_root=/opt/e-rust-port/measure
parent_bin="$measure_root/parent-eprover"
candidate_bin="$measure_root/candidate-eprover"
problem="$source_root/eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop"
measure_pairs="$source_root/experiments/2026-07-25-001-inline-term-arity-lto/measure_pairs.py"
analyze_pairs="$source_root/experiments/2026-07-25-001-inline-term-arity-lto/analyze_pairs.py"

test -x "$parent_bin"
test -x "$candidate_bin"

python3 "$measure_pairs" \
    --parent "$parent_bin" \
    --candidate "$candidate_bin" \
    --problem "$problem" \
    --warmups 4 \
    --pairs 64 \
    --warmup-csv "$artifact_root/native-warmup-2.csv" \
    --measurement-csv "$artifact_root/native-lusk-2.csv"
python3 "$analyze_pairs" \
    "$artifact_root/native-lusk-2.csv" \
    --output "$artifact_root/native-summary-2.json"
python3 "$analyze_pairs" \
    "$artifact_root/native-lusk.csv" \
    "$artifact_root/native-lusk-2.csv" \
    --output "$artifact_root/native-summary-combined.json"
cat "$artifact_root/native-summary-2.json"
cat "$artifact_root/native-summary-combined.json"
