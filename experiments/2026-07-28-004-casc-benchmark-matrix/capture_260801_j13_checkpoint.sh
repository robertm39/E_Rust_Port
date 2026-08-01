#!/usr/bin/env bash
set -euo pipefail

readonly service="casc-j13-v2.service"
readonly old_invocation="88497424106049f9989fa26461cb298e"
readonly resumed_invocation="7eeb7794425849a19fca91b6d28fea12"
readonly run_parent="/opt/e-rust-port/casc-runs"
readonly frozen_binary="/opt/e-rust-port/artifacts/multicore-high-memory-003/umlaut-4e87dac3"
readonly interrupted_stdout="/root/interrupted-SYO326-vampire.stdout"
readonly interrupted_stderr="/root/interrupted-SYO326-vampire.stderr"
readonly evidence="/root/j13-checkpoint-260801"
readonly archive="/root/j13-checkpoint-260801.tar.gz"

if systemctl is-active --quiet "${service}"; then
    echo "error: ${service} is still active" >&2
    exit 1
fi
if pgrep -f '/tools/casc_benchmark/batch.py|/umlaut|/vampire' >/dev/null; then
    echo "error: prover or batch process is still active" >&2
    exit 1
fi
if find /sys/fs/cgroup -maxdepth 1 -type d -name 'umlaut-casc-*' -print -quit |
    grep -q .; then
    echo "error: CASC cgroup residue is still present" >&2
    exit 1
fi
if [[ -e "${evidence}" || -e "${archive}" ]]; then
    echo "error: checkpoint output already exists" >&2
    exit 1
fi

install -d -m 0755 "${evidence}"
journalctl -u "${service}" --no-pager -o short-iso >"${evidence}/service.log"
journalctl _SYSTEMD_INVOCATION_ID="${old_invocation}" --no-pager -o short-iso \
    >"${evidence}/service-original-invocation.log"
journalctl _SYSTEMD_INVOCATION_ID="${resumed_invocation}" --no-pager -o short-iso \
    >"${evidence}/service-resumed-invocation.log"
journalctl --since='2026-08-01 06:28:00 UTC' \
    --until='2026-08-01 06:31:00 UTC' --no-pager -o short-iso \
    >"${evidence}/systemd-reexecution-window.log"
journalctl -u apt-daily-upgrade.service --since='2026-08-01 06:20:00 UTC' \
    --no-pager -o short-iso >"${evidence}/apt-daily-upgrade.log"
systemctl list-timers --all --no-pager >"${evidence}/systemd-timers.txt"
ps -eo pid,ppid,etimes,stat,comm,args >"${evidence}/processes.txt"
find /sys/fs/cgroup -maxdepth 1 -type d -name 'umlaut-casc-*' -print \
    >"${evidence}/cgroup-residue.txt"
uname -a >"${evidence}/uname.txt"

cp -- "${frozen_binary}" "${evidence}/umlaut-4e87dac3"
cp -- "${interrupted_stdout}" "${evidence}/interrupted-SYO326-vampire.stdout"
cp -- "${interrupted_stderr}" "${evidence}/interrupted-SYO326-vampire.stderr"
tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner -czf \
    "${evidence}/casc-runs.tar.gz" -C /opt/e-rust-port casc-runs

(
    cd "${evidence}"
    sha256sum -- * >SHA256SUMS
)
tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner -czf \
    "${archive}" -C /root "$(basename "${evidence}")"
sha256sum "${archive}"
