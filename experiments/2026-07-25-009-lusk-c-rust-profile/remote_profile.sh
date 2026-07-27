#!/usr/bin/env bash
set -Eeuo pipefail

if (($# != 2)); then
    echo "usage: remote_profile.sh SOURCE_ROOT ARTIFACT_ROOT" >&2
    exit 64
fi

source_root=$1
artifact_root=$2
problem="$source_root/eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop"
candidate="$source_root/target/release/eprover"
reference=/root/.cache/e-rust-port/bin/worktree-snapshot/fol/eprover

mkdir -p "$artifact_root"
cd "$source_root"

/root/.cargo/bin/cargo build --locked --release --bin eprover \
    >"$artifact_root/rust-build.stdout" \
    2>"$artifact_root/rust-build.stderr"
python3 tools/linode-runner/linux_compat.py build-reference \
    --repo-root "$source_root" \
    --eprover-commit worktree-snapshot \
    >"$artifact_root/c-build.stdout" \
    2>"$artifact_root/c-build.stderr"

profile()
{
    local label=$1
    local binary=$2
    local status

    set +e
    valgrind --tool=callgrind \
        --log-file="$artifact_root/$label.callgrind.log" \
        --callgrind-out-file="$artifact_root/callgrind-$label.out" \
        "$binary" "$problem" \
        --auto --silent --cpu-limit=600 --memory-limit=2048 \
        --detsort-rw --detsort-new \
        >"$artifact_root/$label.stdout" \
        2>"$artifact_root/$label.stderr"
    status=$?
    set -e
    printf '%s=%s\n' "$label" "$status" |
        tee -a "$artifact_root/exit-status.txt"

    callgrind_annotate --threshold=95 \
        "$artifact_root/callgrind-$label.out" \
        >"$artifact_root/callgrind-$label-self.txt"
    callgrind_annotate --inclusive=yes --threshold=95 \
        "$artifact_root/callgrind-$label.out" \
        >"$artifact_root/callgrind-$label-inclusive.txt"
}

profile reference "$reference"
profile candidate "$candidate"

grep -Fqx 'reference=0' "$artifact_root/exit-status.txt"
grep -Fqx 'candidate=0' "$artifact_root/exit-status.txt"
cmp "$artifact_root/reference.stdout" "$artifact_root/candidate.stdout"
test ! -s "$artifact_root/reference.stderr"
test ! -s "$artifact_root/candidate.stderr"
sha256sum "$artifact_root"/reference.stdout "$artifact_root"/candidate.stdout |
    tee "$artifact_root/output-sha256.txt"
{
    for label in reference candidate; do
        printf '%s=' "$label"
        awk '/^summary:/{print $2}' "$artifact_root/callgrind-$label.out"
    done
} | tee "$artifact_root/instruction-totals.txt"
