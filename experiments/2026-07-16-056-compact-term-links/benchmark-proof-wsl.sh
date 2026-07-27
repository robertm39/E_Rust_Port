#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 6 ]]; then
    echo "usage: benchmark-proof-wsl.sh BASELINE CANDIDATE CORPUS_ROOT OUTPUT_CSV RUNS CASE..." >&2
    exit 2
fi

baseline=$1
candidate=$2
corpus_root=$3
output_csv=$4
runs=$5
shift 5

mkdir -p "$(dirname "$output_csv")"
: >"$output_csv"

for case_name in "$@"; do
    problem="$corpus_root/SMOKETEST/$case_name"
    for ((run = 1; run <= runs; run++)); do
        if ((run % 2 == 0)); then
            implementations=(candidate baseline)
        else
            implementations=(baseline candidate)
        fi
        for implementation in "${implementations[@]}"; do
            if [[ $implementation == baseline ]]; then
                binary=$baseline
            else
                binary=$candidate
            fi
            timing=$(mktemp)
            set +e
            TPTP="$corpus_root/TPTP" /usr/bin/time \
                -f "$implementation,$case_name,$run,%x,%e,%U,%S,%M" \
                -o "$timing" \
                "$binary" "$problem" \
                --auto --silent --cpu-limit=60 --memory-limit=2048 \
                --detsort-rw --detsort-new >/dev/null 2>/dev/null
            set -e
            cat "$timing" >>"$output_csv"
            rm -f "$timing"
        done
    done
done

cat "$output_csv"
