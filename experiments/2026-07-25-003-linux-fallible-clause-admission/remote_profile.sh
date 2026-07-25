#!/usr/bin/env bash
set -Eeuo pipefail

if (($# != 2)); then
    echo "usage: remote_profile.sh SOURCE_ROOT ARTIFACT_ROOT" >&2
    exit 64
fi

source_root=$1
artifact_root=$2
binary="$source_root/target/release/eprover"
problem="$source_root/eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop"
profile="$artifact_root/callgrind-candidate.out"

mkdir -p "$artifact_root"
sha256sum "$binary" >"$artifact_root/binary-sha256.txt"
stat --printf='%n,%s\n' "$binary" >"$artifact_root/binary-size.csv"

/usr/bin/time -v \
    -o "$artifact_root/callgrind-time.txt" \
    valgrind --tool=callgrind \
    --callgrind-out-file="$profile" \
    "$binary" \
    "$problem" \
    --auto \
    --silent \
    --cpu-limit=600 \
    --memory-limit=2048 \
    --detsort-rw \
    --detsort-new \
    >"$artifact_root/callgrind-candidate.stdout" \
    2>"$artifact_root/callgrind-candidate.stderr"

awk '/^summary:/{print $2}' "$profile" \
    >"$artifact_root/callgrind-instructions.txt"
sha256sum "$artifact_root/callgrind-candidate.stdout" \
    >"$artifact_root/callgrind-proof-sha256.txt"
