#!/usr/bin/env bash
set -euo pipefail

binary=${1:?usage: remote_gdb_swv.sh BINARY SOURCE_ROOT OUTPUT}
source_root=${2:?usage: remote_gdb_swv.sh BINARY SOURCE_ROOT OUTPUT}
output=${3:?usage: remote_gdb_swv.sh BINARY SOURCE_ROOT OUTPUT}
problem="$source_root/eprover/EXAMPLE_PROBLEMS/TPTP/SWV851-1.p"

mkdir -p "$(dirname "$output")"
gdb --quiet --batch \
  -ex "set pagination off" \
  -ex "set debuginfod enabled off" \
  -ex "handle SIGXCPU nostop noprint pass" \
  -ex "run" \
  -ex "thread apply all backtrace" \
  --args "$binary" "$problem" \
  --auto --silent --cpu-limit=60 --memory-limit=2048 \
  --detsort-rw --detsort-new --proof-object=1 \
  >"$output" 2>&1
