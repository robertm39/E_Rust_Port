#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
    echo "usage: benchmark-final-scaling.sh RUST_BINARY CORPUS_DIR OUTPUT_CSV" >&2
    exit 2
fi

rust_binary=$1
corpus_dir=$2
output_csv=$3
: >"$output_csv"

for owners in 00100 01000 05000 10000 20000; do
    owner_count=$((10#$owners))
    problem="$corpus_dir/repeated-$owners.p"
    for run in 1 2 3 4 5; do
        timing=$(mktemp)
        /usr/bin/time \
            -f "$owner_count,$run,%x,%e,%U,%S,%M" \
            -o "$timing" \
            "$rust_binary" --cnf --silent --output-file=/dev/null "$problem"
        cat "$timing" >>"$output_csv"
        rm -f "$timing"
    done
done

cat "$output_csv"
