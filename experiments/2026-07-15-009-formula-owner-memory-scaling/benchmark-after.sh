#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
    echo "usage: benchmark-after.sh RUST_BINARY PROBLEM OUTPUT_CSV" >&2
    exit 2
fi

rust_binary=$1
problem=$2
output_csv=$3
: >"$output_csv"

for run in 1 2 3 4 5; do
    timing=$(mktemp)
    /usr/bin/time \
        -f "$run,%x,%e,%U,%S,%M" \
        -o "$timing" \
        "$rust_binary" --cnf --silent --output-file=/dev/null "$problem"
    cat "$timing" >>"$output_csv"
    rm -f "$timing"
done

cat "$output_csv"
