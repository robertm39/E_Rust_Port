#!/usr/bin/env bash
set -Eeuo pipefail

if (($# != 2)); then
    echo "usage: remote_measure.sh SOURCE_ROOT ARTIFACT_ROOT" >&2
    exit 64
fi

source_root=$1
artifact_root=$2
measure_root=/opt/e-rust-port/measure
parent_bin="$measure_root/parent-eprover"
candidate_bin="$measure_root/candidate-eprover"
smoke_problem="$source_root/eprover/EXAMPLE_PROBLEMS/SMOKETEST/socrates.p"
lusk_problem="$source_root/eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop"
measure_pairs="$source_root/experiments/2026-07-25-001-inline-term-arity-lto/measure_pairs.py"
analyze_pairs="$source_root/experiments/2026-07-25-001-inline-term-arity-lto/analyze_pairs.py"

mkdir -p "$artifact_root"
test -x "$parent_bin"
test -x "$candidate_bin"
test -f "$smoke_problem"
test -f "$lusk_problem"

sha256sum "$parent_bin" "$candidate_bin" |
    tee "$artifact_root/binary-sha256.txt"
stat --printf='%n,%s\n' "$parent_bin" "$candidate_bin" |
    tee "$artifact_root/binary-size.csv"

profile()
{
    local label=$1
    local binary=$2
    local problem=$3
    shift 3
    valgrind --tool=callgrind \
        --callgrind-out-file="$artifact_root/callgrind-$label.out" \
        "$binary" "$problem" "$@" \
        >"$artifact_root/callgrind-$label.stdout" \
        2>"$artifact_root/callgrind-$label.stderr"
    callgrind_annotate --inclusive=yes --threshold=0.1 \
        "$artifact_root/callgrind-$label.out" \
        >"$artifact_root/callgrind-$label.txt"
}

profile smoke-parent "$parent_bin" "$smoke_problem" \
    --auto --silent --cpu-limit=10
profile smoke-candidate "$candidate_bin" "$smoke_problem" \
    --auto --silent --cpu-limit=10
profile lusk-parent "$parent_bin" "$lusk_problem" \
    --auto --silent --cpu-limit=600 --memory-limit=2048 \
    --detsort-rw --detsort-new
profile lusk-candidate "$candidate_bin" "$lusk_problem" \
    --auto --silent --cpu-limit=600 --memory-limit=2048 \
    --detsort-rw --detsort-new

{
    for label in smoke-parent smoke-candidate lusk-parent lusk-candidate; do
        printf '%s=' "$label"
        awk '/^summary:/{print $2}' "$artifact_root/callgrind-$label.out"
    done
} | tee "$artifact_root/callgrind-instructions.txt"
sha256sum "$artifact_root"/callgrind-*.stdout |
    tee "$artifact_root/callgrind-proof-sha256.txt"
wc -c "$artifact_root"/callgrind-*.stderr |
    tee "$artifact_root/callgrind-stderr-size.txt"

python3 "$measure_pairs" \
    --parent "$parent_bin" \
    --candidate "$candidate_bin" \
    --problem "$smoke_problem" \
    --warmups 8 \
    --pairs 256 \
    --warmup-csv "$artifact_root/native-smoke-warmup.csv" \
    --measurement-csv "$artifact_root/native-smoke.csv"
python3 "$analyze_pairs" \
    "$artifact_root/native-smoke.csv" \
    --output "$artifact_root/native-smoke-summary.json"

python3 "$measure_pairs" \
    --parent "$parent_bin" \
    --candidate "$candidate_bin" \
    --problem "$lusk_problem" \
    --warmups 4 \
    --pairs 32 \
    --warmup-csv "$artifact_root/native-lusk-warmup.csv" \
    --measurement-csv "$artifact_root/native-lusk.csv"
python3 "$analyze_pairs" \
    "$artifact_root/native-lusk.csv" \
    --output "$artifact_root/native-lusk-summary.json"

cat "$artifact_root/native-smoke-summary.json"
cat "$artifact_root/native-lusk-summary.json"
