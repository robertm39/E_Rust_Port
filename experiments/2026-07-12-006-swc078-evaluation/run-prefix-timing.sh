#!/usr/bin/env bash
set -euo pipefail

repo=/mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port
problem="$repo/eprover/EXAMPLE_PROBLEMS/TPTP/SWC078-1.p"
c_ref=/home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover
rust="$repo/target/release/eprover"
args=(
  --auto
  --output-level=1
  --processed-clauses-limit=6367
  --cpu-limit=600
  --memory-limit=2048
  --detsort-rw
  --detsort-new
)

export TPTP="$repo/eprover/EXAMPLE_PROBLEMS/TPTP"
c_status=0
/usr/bin/time -f 'C wall=%e user=%U system=%S rss_kb=%M' \
  "$c_ref" "${args[@]}" "$problem" >/dev/null || c_status=$?
rust_status=0
/usr/bin/time -f 'Rust wall=%e user=%U system=%S rss_kb=%M' \
  "$rust" "${args[@]}" "$problem" >/dev/null || rust_status=$?
printf 'C status=%d Rust status=%d\n' "$c_status" "$rust_status"
