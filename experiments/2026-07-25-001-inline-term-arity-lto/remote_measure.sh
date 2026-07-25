#!/usr/bin/env bash
set -Eeuo pipefail

if (($# != 2)); then
    echo "usage: remote_measure.sh SOURCE_ROOT ARTIFACT_ROOT" >&2
    exit 64
fi

source_root=$1
artifact_root=$2
experiment_root="$source_root/experiments/2026-07-25-001-inline-term-arity-lto"
termtypes="$source_root/src/terms/termtypes.rs"
problem="$source_root/eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop"
target_root="$source_root/target"
candidate_bin="$artifact_root/eprover-candidate"
parent_bin="$artifact_root/eprover-parent"
original_termtypes="$artifact_root/termtypes.candidate.rs"

mkdir -p "$artifact_root"
cp "$termtypes" "$original_termtypes"
trap 'cp "$original_termtypes" "$termtypes"' EXIT

cd "$source_root"
echo "== Build candidate =="
cargo build --locked --release --bin eprover
cp "$target_root/release/eprover" "$candidate_bin"

echo "== Restore exact parent accessor =="
python3 - "$termtypes" <<'PY'
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
source = path.read_text(encoding="utf-8")
candidate = """    #[allow(
        clippy::inline_always,
        reason = "measured experiment tests this hot C-style arity accessor boundary"
    )]
    #[inline(always)]
"""
if source.count(candidate) != 1:
    raise SystemExit("candidate arity attribute block was not unique")
path.write_text(source.replace(candidate, ""), encoding="utf-8")
PY

echo "== Build parent =="
cargo build --locked --release --bin eprover
cp "$target_root/release/eprover" "$parent_bin"
cp "$original_termtypes" "$termtypes"

sha256sum "$parent_bin" "$candidate_bin" | tee "$artifact_root/binary-sha256.txt"
stat --printf='%n,%s\n' "$parent_bin" "$candidate_bin" |
    tee "$artifact_root/binary-size.csv"

common_args=(
    "$problem"
    --auto
    --silent
    --cpu-limit=600
    --memory-limit=2048
    --detsort-rw
    --detsort-new
)

echo "== Exact parent Callgrind =="
valgrind --tool=callgrind \
    --callgrind-out-file="$artifact_root/callgrind-parent.out" \
    "$parent_bin" "${common_args[@]}" \
    >"$artifact_root/callgrind-parent.stdout" \
    2>"$artifact_root/callgrind-parent.stderr"

echo "== Exact candidate Callgrind =="
valgrind --tool=callgrind \
    --callgrind-out-file="$artifact_root/callgrind-candidate.out" \
    "$candidate_bin" "${common_args[@]}" \
    >"$artifact_root/callgrind-candidate.stdout" \
    2>"$artifact_root/callgrind-candidate.stderr"

{
    printf 'parent='
    awk '/^summary:/{print $2}' "$artifact_root/callgrind-parent.out"
    printf 'candidate='
    awk '/^summary:/{print $2}' "$artifact_root/callgrind-candidate.out"
} | tee "$artifact_root/callgrind-instructions.txt"
sha256sum \
    "$artifact_root/callgrind-parent.stdout" \
    "$artifact_root/callgrind-candidate.stdout" |
    tee "$artifact_root/callgrind-proof-sha256.txt"

echo "== Alternating native measurements =="
python3 "$experiment_root/measure_pairs.py" \
    --parent "$parent_bin" \
    --candidate "$candidate_bin" \
    --problem "$problem" \
    --warmups 4 \
    --pairs 64 \
    --warmup-csv "$artifact_root/native-warmup.csv" \
    --measurement-csv "$artifact_root/native-lusk.csv"
python3 "$experiment_root/analyze_pairs.py" \
    "$artifact_root/native-lusk.csv" \
    --output "$artifact_root/native-summary.json"
cat "$artifact_root/native-summary.json"
