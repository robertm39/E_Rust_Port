#!/usr/bin/env bash
set -euo pipefail

repo=/mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port
artifact="$repo/.artifacts/experiments/2026-07-12-006-swc078-evaluation"
mkdir -p "$artifact"
export TPTP="$repo/eprover/EXAMPLE_PROBLEMS/TPTP"

status=0
valgrind --tool=callgrind \
  --callgrind-out-file="$artifact/callgrind.out" \
  "$repo/target/release/eprover" \
  --auto --output-level=1 --processed-clauses-limit=6367 \
  --cpu-limit=600 --memory-limit=2048 --detsort-rw --detsort-new \
  "$repo/eprover/EXAMPLE_PROBLEMS/TPTP/SWC078-1.p" \
  >/dev/null || status=$?

callgrind_annotate --inclusive=no --threshold=99 --auto=no \
  "$artifact/callgrind.out" >"$artifact/callgrind-self.txt"
printf 'status=%d\n' "$status"
