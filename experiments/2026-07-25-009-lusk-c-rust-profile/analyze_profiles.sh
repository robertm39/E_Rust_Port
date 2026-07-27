#!/usr/bin/env bash
set -Eeuo pipefail

if (($# != 1)); then
    echo "usage: analyze_profiles.sh ARTIFACT_ROOT" >&2
    exit 64
fi

artifact_root=$1

for label in reference candidate; do
    callgrind_annotate --threshold=95 \
        "$artifact_root/callgrind-$label.out" \
        >"$artifact_root/callgrind-$label-self.txt"
    callgrind_annotate --tree=both --threshold=95 \
        "$artifact_root/callgrind-$label.out" \
        >"$artifact_root/callgrind-$label-tree.txt"
    callgrind_annotate --inclusive=yes --threshold=95 \
        "$artifact_root/callgrind-$label.out" \
        >"$artifact_root/callgrind-$label-inclusive.txt"
done

sha256sum "$artifact_root"/reference.stdout "$artifact_root"/candidate.stdout |
    tee "$artifact_root/output-sha256.txt"
{
    for label in reference candidate; do
        printf '%s=' "$label"
        awk '/^summary:/{print $2}' "$artifact_root/callgrind-$label.out"
    done
} | tee "$artifact_root/instruction-totals.txt"
