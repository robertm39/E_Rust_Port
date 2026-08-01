#!/usr/bin/env bash
set -Eeuo pipefail

if (($# != 1)); then
    echo "usage: maintenance_lifecycle_test.sh ARTIFACT_ROOT" >&2
    exit 2
fi

artifact_root="$(realpath -m "$1")"
if [[ "$artifact_root" != /opt/e-rust-port/artifacts/e-rust-codex-* ]]; then
    echo "refusing unexpected artifact root: $artifact_root" >&2
    exit 2
fi

maintenance_record=/opt/e-rust-port/package-maintenance-quiescence.json
test -f "$maintenance_record"
mkdir -p "$artifact_root"
cp "$maintenance_record" "$artifact_root/package-maintenance-quiescence.json"
(
    cd "$(dirname "$maintenance_record")"
    sha256sum "$(basename "$maintenance_record")"
) >"$artifact_root/package-maintenance-quiescence.sha256"
(
    cd "$artifact_root"
    sha256sum -c package-maintenance-quiescence.sha256
)

maintenance_units=(
    apt-daily.timer
    apt-daily-upgrade.timer
    apt-daily.service
    apt-daily-upgrade.service
)
for maintenance_unit in "${maintenance_units[@]}"; do
    test "$(systemctl show --property=ActiveState --value "$maintenance_unit")" = inactive
    test "$(systemctl show --property=UnitFileState --value "$maintenance_unit")" = masked
done

work_root="$(mktemp -d /opt/e-rust-port/package-maintenance-probe.XXXXXX)"
unit="e-rust-port-maintenance-probe-$$.service"
worker="$work_root/worker.sh"
result_root="$work_root/results"
ready="$work_root/ready"
mkdir -p "$result_root"

cleanup() {
    systemctl stop "$unit" >/dev/null 2>&1 || true
    systemctl reset-failed "$unit" >/dev/null 2>&1 || true
    if [[ "$work_root" == /opt/e-rust-port/package-maintenance-probe.* ]]; then
        rm -rf -- "$work_root"
    fi
}
trap cleanup EXIT

cat >"$worker" <<'WORKER'
#!/usr/bin/env bash
set -Eeuo pipefail

result_root="$1"
ready="$2"
result="$result_root/result-0001.json"
hashes="$result_root/results.sha256"
result_temporary="$result.tmp"
hash_temporary="$hashes.tmp"

cleanup_partial() {
    rm -f -- "$result_temporary" "$hash_temporary"
}
trap 'cleanup_partial; exit 0' INT TERM
trap cleanup_partial EXIT

if [[ -f "$result" || -f "$hashes" ]]; then
    test -f "$result"
    test -f "$hashes"
    (cd "$result_root" && sha256sum -c results.sha256)
    printf 'hash-valid coordinate skipped\n' >"$result_root/resume-verified"
    exit 0
fi

printf '{"coordinate":"maintenance-probe","schema_version":1}\n' \
    >"$result_temporary"
mv "$result_temporary" "$result"
(
    cd "$result_root"
    sha256sum "$(basename "$result")" >"$(basename "$hash_temporary")"
)
mv "$hash_temporary" "$hashes"
touch "$ready"

while true; do
    sleep 60 &
    wait "$!" || true
done
WORKER
chmod 0755 "$worker"

systemd-run \
    --unit="$unit" \
    --collect \
    --property=Type=exec \
    --property=KillMode=control-group \
    --property=Restart=no \
    "$worker" "$result_root" "$ready"

for _ in $(seq 1 100); do
    [[ -f "$ready" ]] && break
    sleep 0.1
done
test -f "$ready"
test "$(systemctl show --property=ActiveState --value "$unit")" = active

pid_before="$(systemctl show --property=MainPID --value "$unit")"
invocation_before="$(systemctl show --property=InvocationID --value "$unit")"
control_group="$(systemctl show --property=ControlGroup --value "$unit")"
[[ "$pid_before" =~ ^[1-9][0-9]*$ ]]
[[ "$invocation_before" =~ ^[0-9a-f]{32}$ ]]
[[ "$control_group" == /system.slice/e-rust-port-maintenance-probe-*.service ]]
kill -0 "$pid_before"

systemctl daemon-reexec
for _ in $(seq 1 100); do
    if systemctl show "$unit" >/dev/null 2>&1; then
        break
    fi
    sleep 0.1
done

pid_after="$(systemctl show --property=MainPID --value "$unit")"
invocation_after="$(systemctl show --property=InvocationID --value "$unit")"
test "$pid_after" = "$pid_before"
test "$invocation_after" = "$invocation_before"
test "$(systemctl show --property=ActiveState --value "$unit")" = active
kill -0 "$pid_after"
for maintenance_unit in "${maintenance_units[@]}"; do
    test "$(systemctl show --property=ActiveState --value "$maintenance_unit")" = inactive
    test "$(systemctl show --property=UnitFileState --value "$maintenance_unit")" = masked
done

systemctl kill --kill-whom=main --signal=SIGINT "$unit"
for _ in $(seq 1 100); do
    if ! systemctl is-active --quiet "$unit"; then
        break
    fi
    sleep 0.1
done
if systemctl is-active --quiet "$unit"; then
    echo "transient maintenance probe remained active after SIGINT" >&2
    exit 1
fi
systemctl stop "$unit" >/dev/null 2>&1 || true

cgroup_procs="/sys/fs/cgroup${control_group}/cgroup.procs"
if [[ -s "$cgroup_procs" ]]; then
    echo "transient maintenance probe left processes in $control_group" >&2
    cat "$cgroup_procs" >&2
    exit 1
fi
if pgrep -f -- "$worker" >/dev/null; then
    echo "transient maintenance probe left a worker process" >&2
    exit 1
fi
if find "$result_root" -maxdepth 1 -type f -name '*.tmp' -print -quit |
    grep -q .; then
    echo "transient maintenance probe left incomplete output" >&2
    exit 1
fi

(cd "$result_root" && sha256sum -c results.sha256)
"$worker" "$result_root" "$ready"
test -f "$result_root/resume-verified"
test "$(find "$result_root" -maxdepth 1 -type f -name 'result-*.json' | wc -l)" = 1
(cd "$result_root" && sha256sum -c results.sha256)

journalctl --unit "$unit" --no-pager \
    >"$artifact_root/package-maintenance-lifecycle-journal.txt"
record_sha256="$(sha256sum "$maintenance_record" | awk '{print $1}')"
result_sha256="$(sha256sum "$result_root/result-0001.json" | awk '{print $1}')"
python3 - "$artifact_root/package-maintenance-lifecycle.json" \
    "$unit" "$pid_before" "$pid_after" "$invocation_before" \
    "$invocation_after" "$control_group" "$record_sha256" \
    "$result_sha256" <<'PY'
import json
import os
import sys
from datetime import datetime, timezone
from pathlib import Path

(
    output,
    unit,
    pid_before,
    pid_after,
    invocation_before,
    invocation_after,
    control_group,
    record_sha256,
    result_sha256,
) = sys.argv[1:]
record = {
    "schema_version": 1,
    "captured_at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
    "unit": unit,
    "pid_before_daemon_reexec": int(pid_before),
    "pid_after_daemon_reexec": int(pid_after),
    "invocation_before_daemon_reexec": invocation_before,
    "invocation_after_daemon_reexec": invocation_after,
    "control_group": control_group,
    "maintenance_record_sha256": record_sha256,
    "result_sha256": result_sha256,
    "daemon_reexec_preserved_process": True,
    "inactive_after_sigint": True,
    "cgroup_empty_after_sigint": True,
    "no_incomplete_output": True,
    "hash_valid_resume": True,
}
path = Path(output)
temporary = path.with_suffix(".json.tmp")
temporary.write_text(
    json.dumps(record, indent=2, sort_keys=True) + "\n",
    encoding="utf-8",
)
os.replace(temporary, path)
PY

printf 'ok\n' >"$artifact_root/PACKAGE_MAINTENANCE_LIFECYCLE_COMPLETE"
