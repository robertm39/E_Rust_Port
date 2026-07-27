#!/usr/bin/env bash
set -Eeuo pipefail

if (($# != 2)); then
    echo "usage: remote_validate.sh SOURCE_ROOT ARTIFACT_ROOT" >&2
    exit 64
fi

source_root=$1
artifact_root=$2
problem="$source_root/eprover/EXAMPLE_PROBLEMS/TPTP/SWB008+1.p"
tptp_root="$source_root/eprover/EXAMPLE_PROBLEMS/TPTP"
candidate="$source_root/target/release/eprover"
reference=/root/.cache/e-rust-port/bin/worktree-snapshot/fol/eprover

mkdir -p "$artifact_root"
cd "$source_root"

/root/.cargo/bin/cargo fmt --all -- --check \
    >"$artifact_root/rust-fmt.stdout" \
    2>"$artifact_root/rust-fmt.stderr"
/root/.cargo/bin/cargo test --locked --test eprover_hard_limit \
    zero_cpu_limit_reports_hard_timeout_once \
    >"$artifact_root/integration-test.stdout" \
    2>"$artifact_root/integration-test.stderr"
/root/.cargo/bin/cargo test --locked --lib \
    run_hard_time_limit_uses_cpu_limit_exit_status \
    >"$artifact_root/unit-test.stdout" \
    2>"$artifact_root/unit-test.stderr"
/root/.cargo/bin/cargo clippy --locked --all-targets --all-features -- \
    -D warnings -D clippy::pedantic \
    >"$artifact_root/rust-clippy.stdout" \
    2>"$artifact_root/rust-clippy.stderr"
/root/.cargo/bin/cargo build --locked --release --bin eprover \
    >"$artifact_root/rust-build.stdout" \
    2>"$artifact_root/rust-build.stderr"

python3 tools/linode-runner/linux_compat.py build-reference \
    --repo-root "$source_root" \
    --eprover-commit worktree-snapshot \
    >"$artifact_root/c-build.stdout" \
    2>"$artifact_root/c-build.stderr"

run_case()
{
    local label=$1
    local binary=$2
    local status

    set +e
    TPTP="$tptp_root" "$binary" "$problem" \
        --auto --silent --cpu-limit=60 --memory-limit=2048 \
        --detsort-rw --detsort-new --proof-object=1 \
        >"$artifact_root/$label.stdout" \
        2>"$artifact_root/$label.stderr"
    status=$?
    set -e
    printf '%s=%s\n' "$label" "$status" |
        tee -a "$artifact_root/exit-status.txt"
}

run_case reference "$reference"
run_case candidate "$candidate"

test "$(sed -n '/^%% Failure: Resource limit exceeded (time)$/p' \
    "$artifact_root/reference.stdout" | wc -l)" -eq 1
test "$(sed -n '/^%% Failure: Resource limit exceeded (time)$/p' \
    "$artifact_root/candidate.stdout" | wc -l)" -eq 1
test "$(grep -Fxc 'eprover: CPU time limit exceeded, terminating' \
    "$artifact_root/reference.stderr")" -eq 1
test "$(grep -Fxc 'eprover: CPU time limit exceeded, terminating' \
    "$artifact_root/candidate.stderr")" -eq 1
grep -Fqx 'reference=8' "$artifact_root/exit-status.txt"
grep -Fqx 'candidate=8' "$artifact_root/exit-status.txt"
cmp "$artifact_root/reference.stdout" "$artifact_root/candidate.stdout"
cmp "$artifact_root/reference.stderr" "$artifact_root/candidate.stderr"
sha256sum "$artifact_root"/reference.std* "$artifact_root"/candidate.std* |
    tee "$artifact_root/output-sha256.txt"
