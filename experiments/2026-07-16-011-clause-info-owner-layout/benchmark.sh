#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 6 ]]; then
    echo "usage: benchmark.sh C_BINARY BASELINE_RUST CANDIDATE_RUST REPEATED_CORPUS UNIQUE_CORPUS OUTPUT_CSV" >&2
    exit 2
fi

c_binary=$1
baseline_binary=$2
candidate_binary=$3
repeated_corpus=$4
unique_corpus=$5
output_csv=$6

mkdir -p "$(dirname "$output_csv")"
: >"$output_csv"

for shape in repeated unique; do
    case "$shape" in
        repeated) corpus=$repeated_corpus; prefix=repeated ;;
        unique) corpus=$unique_corpus; prefix=unique-atom ;;
    esac
    for owners in 00100 01000 05000 10000 20000; do
        owner_count=$((10#$owners))
        problem="$corpus/$prefix-$owners.p"
        for run in 1 2 3 4 5; do
            case $((run % 3)) in
                0) implementations=(c baseline candidate) ;;
                1) implementations=(baseline candidate c) ;;
                2) implementations=(candidate c baseline) ;;
            esac
            for implementation in "${implementations[@]}"; do
                case "$implementation" in
                    c) binary=$c_binary ;;
                    baseline) binary=$baseline_binary ;;
                    candidate) binary=$candidate_binary ;;
                esac
                timing=$(mktemp)
                /usr/bin/time \
                    -f "$implementation,$shape,$owner_count,$run,%x,%e,%U,%S,%M" \
                    -o "$timing" \
                    "$binary" --cnf --silent --output-file=/dev/null "$problem"
                cat "$timing" >>"$output_csv"
                rm -f "$timing"
            done
        done
    done
done

cat "$output_csv"
