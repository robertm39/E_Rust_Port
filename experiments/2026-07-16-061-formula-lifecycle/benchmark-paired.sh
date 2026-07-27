#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
    echo "usage: benchmark-paired.sh C_BINARY RUST_BINARY PROBLEM OUTPUT_CSV" >&2
    exit 2
fi

c_binary=$1
rust_binary=$2
problem=$3
output_csv=$4
: >"$output_csv"

for run in 1 2 3 4 5; do
    if ((run % 2 == 1)); then
        order=(c rust)
    else
        order=(rust c)
    fi
    for implementation in "${order[@]}"; do
        if [[ "$implementation" == c ]]; then
            binary=$c_binary
        else
            binary=$rust_binary
        fi
        timing=$(mktemp)
        /usr/bin/time \
            -f "$implementation,$run,%x,%e,%U,%S,%M" \
            -o "$timing" \
            "$binary" --cnf --silent --output-file=/dev/null "$problem"
        cat "$timing" >>"$output_csv"
        rm -f "$timing"
    done
done

cat "$output_csv"
