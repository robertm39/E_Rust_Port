#!/usr/bin/env bash
set -euo pipefail

repo=/mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port
artifact="$repo/.artifacts/experiments/2026-07-12-006-swc078-evaluation"
problem="$repo/eprover/EXAMPLE_PROBLEMS/TPTP/SWC078-1.p"
c_ref=/home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover
rust="$repo/target/release/eprover"
args=(
  --auto
  --output-level=1
  --cpu-limit=60
  --memory-limit=2048
  --detsort-rw
  --detsort-new
)

mkdir -p "$artifact"
export TPTP="$repo/eprover/EXAMPLE_PROBLEMS/TPTP"

c_status=0
"$c_ref" "${args[@]}" "$problem" >"$artifact/c-prefix.txt" || c_status=$?
rust_status=0
"$rust" "${args[@]}" "$problem" >"$artifact/rust-prefix.txt" || rust_status=$?

if [[ $c_status -ne $rust_status ]]; then
  printf 'status mismatch: C=%d Rust=%d\n' "$c_status" "$rust_status" >&2
  exit 1
fi

printf 'C status=%d Rust status=%d\n' "$c_status" "$rust_status"

python3 "$repo/experiments/2026-07-12-005-swc078-selection/compare_selected.py" \
  "$artifact/rust-prefix.txt" "$artifact/c-prefix.txt"
