#!/usr/bin/env bash
set -euo pipefail

repo=/mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port
artifact="$repo/.artifacts/experiments/2026-07-12-007-geo288-search"
problem="$repo/eprover/EXAMPLE_PROBLEMS/TPTP/GEO288+1.p"
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
/usr/bin/time -f 'C wall=%e user=%U system=%S rss_kb=%M' \
  "$c_ref" "${args[@]}" "$problem" >"$artifact/c-trace.txt" || c_status=$?
rust_status=0
/usr/bin/time -f 'Rust wall=%e user=%U system=%S rss_kb=%M' \
  "$rust" "${args[@]}" "$problem" >"$artifact/rust-trace.txt" || rust_status=$?

printf 'C status=%d Rust status=%d\n' "$c_status" "$rust_status"
python3 "$repo/experiments/2026-07-12-005-swc078-selection/compare_selected.py" \
  "$artifact/rust-trace.txt" "$artifact/c-trace.txt"
