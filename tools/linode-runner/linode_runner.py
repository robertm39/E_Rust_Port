#!/usr/bin/env python3
"""Provision short-lived Linode workers for Linux builds and profiling."""

from __future__ import annotations

import argparse
import contextlib
import hashlib
import ipaddress
import json
import math
import os
import re
import secrets
import shlex
import shutil
import subprocess
import sys
import tarfile
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid
from datetime import datetime, timedelta, timezone
from email.utils import parsedate_to_datetime
from pathlib import Path, PurePosixPath
from typing import Any, Iterable, NamedTuple, Sequence


API_BASE = "https://api.linode.com/v4"
DEFAULT_TYPE = "g8-dedicated-8-4"
HIGH_MEMORY_TYPE = "g7-highmem-8"
DEFAULT_REGION = "us-ord"
DEFAULT_IMAGE = "linode/ubuntu24.04"
LABEL_PREFIX = "e-rust-codex-"
FIXED_EST = timezone(timedelta(hours=-5), name="EST")
HIGH_MEMORY_DAILY_LIMIT = timedelta(hours=4)
REMOTE_ROOT = PurePosixPath("/opt/e-rust-port")
REPO_ROOT = Path(__file__).resolve().parents[2]
LOCAL_APP_DATA = Path(
    os.environ.get("LOCALAPPDATA", Path.home() / ".local" / "share")
)
LOCAL_ROOT = LOCAL_APP_DATA / "E-Rust-Port" / "linode-runner"
CURRENT_STATE = LOCAL_ROOT / "current.json"
PARKED_ROOT = LOCAL_ROOT / "parked"
RUN_HISTORY = LOCAL_ROOT / "runs"
SSH_KEY = LOCAL_ROOT / "linode-runner-ed25519"
LIFECYCLE_LOCK = LOCAL_ROOT / "lifecycle.lock"
PROVISION_CLAIM = LOCAL_ROOT / "provision-claim.json"
REAPER_CONFIG = LOCAL_ROOT / "reaper.json"
ARTIFACT_ROOT = REPO_ROOT / ".artifacts" / "linode"
BILLING_HOUR = timedelta(hours=1)
REAPER_MARGIN = timedelta(minutes=2)
REMOTE_REAPER_ROOT = PurePosixPath("/root/.local/share/umlaut-linode-reaper")
REMOTE_REAPER_SOURCE = REPO_ROOT / "tools" / "linode-runner" / "remote_reaper.py"
REUSE_DISARM_GUARD = timedelta(seconds=30)
PACKAGE_MAINTENANCE_RECORD = REMOTE_ROOT / "package-maintenance-quiescence.json"
PACKAGE_MAINTENANCE_UNITS = (
    "apt-daily.timer",
    "apt-daily-upgrade.timer",
    "apt-daily.service",
    "apt-daily-upgrade.service",
)


class PlanSpec(NamedTuple):
    """Expected catalog values for a supported Linode plan."""

    label: str
    memory: int
    vcpus: int
    disk: int
    plan_class: str


PLAN_SPECS = {
    DEFAULT_TYPE: PlanSpec(
        label="G8 Dedicated 8x4",
        memory=8192,
        vcpus=4,
        disk=83968,
        plan_class="dedicated",
    ),
    HIGH_MEMORY_TYPE: PlanSpec(
        label="Linode 150GB",
        memory=153600,
        vcpus=8,
        disk=204800,
        plan_class="highmem",
    ),
}


class HighMemoryUsage(NamedTuple):
    """Bank-adjusted high-memory usage within one fixed-EST accounting day."""

    actual: timedelta
    balance_at_start: timedelta
    day_start: datetime
    next_boundary: datetime

    @property
    def banked_at_start(self) -> timedelta:
        return max(self.balance_at_start, timedelta())

    @property
    def debt_at_start(self) -> timedelta:
        return max(-self.balance_at_start, timedelta())

    @property
    def capacity(self) -> timedelta:
        return max(HIGH_MEMORY_DAILY_LIMIT + self.balance_at_start, timedelta())

    @property
    def remaining(self) -> timedelta:
        return max(self.capacity - self.actual, timedelta())

    @property
    def next_balance(self) -> timedelta:
        return min(
            HIGH_MEMORY_DAILY_LIMIT,
            self.balance_at_start + HIGH_MEMORY_DAILY_LIMIT - self.actual,
        )

    @property
    def banked_at_next_boundary(self) -> timedelta:
        return max(self.next_balance, timedelta())

    @property
    def debt_at_next_boundary(self) -> timedelta:
        return max(-self.next_balance, timedelta())

    @property
    def exhausted(self) -> bool:
        return self.actual >= self.capacity

    @property
    def projected_eligible_at(self) -> datetime:
        """Earliest boundary allowing a start if no additional usage accrues."""

        balance = self.next_balance
        boundary = self.next_boundary
        while HIGH_MEMORY_DAILY_LIMIT + balance <= timedelta():
            balance = min(
                HIGH_MEMORY_DAILY_LIMIT,
                balance + HIGH_MEMORY_DAILY_LIMIT,
            )
            boundary += timedelta(days=1)
        return boundary


class RunnerError(RuntimeError):
    """A user-facing runner failure."""


class ApiError(RunnerError):
    """A Linode API request failure."""

    def __init__(self, method: str, path: str, status: int, detail: str):
        super().__init__(f"{method} {path} failed ({status}): {detail}")
        self.method = method
        self.path = path
        self.status = status
        self.detail = detail


def utc_now() -> datetime:
    return datetime.now(timezone.utc)


def iso_now() -> str:
    return utc_now().isoformat(timespec="seconds")


def parse_time(value: str) -> datetime:
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as error:
        raise RunnerError(f"Invalid timestamp {value!r}") from error
    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=timezone.utc)
    return parsed.astimezone(timezone.utc)


def parse_http_date(value: str | None) -> datetime:
    """Parse an HTTP Date header as an aware UTC timestamp."""

    if not value:
        raise RunnerError("Linode API response did not include a Date header")
    try:
        parsed = parsedate_to_datetime(value)
    except (TypeError, ValueError) as error:
        raise RunnerError(f"Invalid Linode API Date header: {value!r}") from error
    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=timezone.utc)
    return parsed.astimezone(timezone.utc)


def format_utc(value: datetime) -> str:
    """Format a normalized, second-resolution UTC timestamp."""

    return value.astimezone(timezone.utc).isoformat(timespec="seconds")


def fixed_est_day_bounds(now: datetime) -> tuple[datetime, datetime]:
    """Return fixed UTC-05:00 accounting-day bounds expressed in UTC."""

    fixed_now = now.astimezone(FIXED_EST)
    day_start = datetime.combine(
        fixed_now.date(),
        datetime.min.time(),
        tzinfo=FIXED_EST,
    )
    next_reset = day_start + timedelta(days=1)
    return (
        day_start.astimezone(timezone.utc),
        next_reset.astimezone(timezone.utc),
    )


def run_id() -> str:
    stamp = utc_now().strftime("%y%m%d-%H%M%S")
    return f"{stamp}-{secrets.token_hex(2)}"


def resource_label(identifier: str) -> str:
    label = f"{LABEL_PREFIX}{identifier}"
    if len(label) > 32:
        raise RunnerError(f"Generated resource label is too long: {label}")
    return label


def is_managed_label(label: object) -> bool:
    return isinstance(label, str) and label.startswith(LABEL_PREFIX)


def require_managed_label(actual: object, expected: str, resource: str) -> None:
    if not is_managed_label(expected):
        raise RunnerError(f"Refusing to manage unsafe saved {resource} label: {expected}")
    if actual != expected:
        raise RunnerError(
            f"Refusing to delete {resource}: live label {actual!r} "
            f"does not match saved label {expected!r}"
        )


class LinodeApi:
    """Small standard-library client for the Linode v4 API."""

    def __init__(self, token: str | None = None, base_url: str = API_BASE):
        self.token = token or os.environ.get("LINODE_TOKEN", "")
        if not self.token:
            raise RunnerError(
                "LINODE_TOKEN is missing. Use linode-runner.ps1 so the "
                "DPAPI-encrypted token is supplied safely."
            )
        self.base_url = base_url.rstrip("/")
        self.last_response_at: datetime | None = None

    def _capture_response_time(self, headers: Any) -> None:
        """Retain a server-controlled timestamp without breaking cleanup."""

        try:
            self.last_response_at = parse_http_date(headers.get("Date"))
        except (AttributeError, RunnerError):
            self.last_response_at = None

    def request(
        self,
        method: str,
        path: str,
        payload: dict[str, Any] | None = None,
        *,
        allow_404: bool = False,
    ) -> Any:
        body = None
        headers = {
            "Accept": "application/json",
            "Authorization": f"Bearer {self.token}",
            "User-Agent": "umlaut-linode-runner/1",
        }
        if payload is not None:
            body = json.dumps(payload).encode("utf-8")
            headers["Content-Type"] = "application/json"
        request = urllib.request.Request(
            f"{self.base_url}{path}",
            data=body,
            headers=headers,
            method=method,
        )
        try:
            with urllib.request.urlopen(request, timeout=60) as response:
                response_body = response.read()
                self._capture_response_time(response.headers)
        except urllib.error.HTTPError as error:
            response_body = error.read()
            self._capture_response_time(error.headers)
            if allow_404 and error.code == 404:
                return None
            detail = response_body.decode("utf-8", errors="replace")
            try:
                parsed = json.loads(detail)
                errors = parsed.get("errors", [])
                detail = "; ".join(
                    f"{item.get('field')}: {item.get('reason')}"
                    if item.get("field")
                    else str(item.get("reason"))
                    for item in errors
                ) or detail
            except (json.JSONDecodeError, AttributeError):
                pass
            raise ApiError(method, path, error.code, detail) from error
        except urllib.error.URLError as error:
            raise RunnerError(f"Could not reach the Linode API: {error.reason}") from error
        if not response_body:
            return None
        return json.loads(response_body.decode("utf-8"))

    def get(self, path: str, *, allow_404: bool = False) -> Any:
        return self.request("GET", path, allow_404=allow_404)

    def post(self, path: str, payload: dict[str, Any]) -> Any:
        return self.request("POST", path, payload)

    def put(self, path: str, payload: dict[str, Any]) -> Any:
        return self.request("PUT", path, payload)

    def delete(self, path: str, *, allow_404: bool = False) -> Any:
        return self.request("DELETE", path, allow_404=allow_404)

    def trusted_now(self) -> datetime:
        """Read current UTC time from Linode's HTTPS response headers."""

        self.request("HEAD", "/regions")
        if self.last_response_at is None:
            raise RunnerError(
                "Could not obtain trusted UTC time from the Linode API; "
                "high-memory starts fail closed"
            )
        return self.last_response_at

    def list_all(self, path: str) -> list[dict[str, Any]]:
        separator = "&" if "?" in path else "?"
        page = 1
        items: list[dict[str, Any]] = []
        while True:
            response = self.get(f"{path}{separator}page={page}&page_size=100")
            items.extend(response.get("data", []))
            if page >= int(response.get("pages", 1)):
                return items
            page += 1


def firewall_rules(allow_cidr: str) -> dict[str, Any]:
    return {
        "inbound": [
            {
                "label": "ssh-controller",
                "description": "SSH from the controller public IPv4 only",
                "action": "ACCEPT",
                "protocol": "TCP",
                "ports": "22",
                "addresses": {"ipv4": [allow_cidr]},
            }
        ],
        "outbound": [],
        "inbound_policy": "DROP",
        "outbound_policy": "ACCEPT",
    }


def firewall_payload(label: str, allow_cidr: str) -> dict[str, Any]:
    return {"label": label, "rules": firewall_rules(allow_cidr)}


def linode_payload(
    label: str,
    firewall_id: int,
    public_key: str,
    *,
    linode_type: str = DEFAULT_TYPE,
    region: str = DEFAULT_REGION,
    image: str = DEFAULT_IMAGE,
) -> dict[str, Any]:
    return {
        "label": label,
        "type": linode_type,
        "region": region,
        "image": image,
        "booted": True,
        "backups_enabled": False,
        "disk_encryption": "enabled",
        "interface_generation": "legacy_config",
        "firewall_id": firewall_id,
        "authorized_keys": [public_key],
    }


def atomic_write_json(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(f"{path.suffix}.{os.getpid()}.tmp")
    temporary.write_text(
        json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    os.replace(temporary, path)


@contextlib.contextmanager
def lifecycle_lock(path: Path | None = None) -> Iterable[None]:
    """Serialize active/parked state transitions and local deletion."""

    lock_path = LIFECYCLE_LOCK if path is None else path
    lock_path.parent.mkdir(parents=True, exist_ok=True)
    with lock_path.open("a+b") as lock:
        lock.seek(0, os.SEEK_END)
        if lock.tell() == 0:
            lock.write(b"\0")
            lock.flush()
        lock.seek(0)
        if os.name == "nt":
            import msvcrt

            msvcrt.locking(lock.fileno(), msvcrt.LK_LOCK, 1)
            try:
                yield
            finally:
                lock.seek(0)
                msvcrt.locking(lock.fileno(), msvcrt.LK_UNLCK, 1)
        else:
            import fcntl

            fcntl.flock(lock.fileno(), fcntl.LOCK_EX)
            try:
                yield
            finally:
                fcntl.flock(lock.fileno(), fcntl.LOCK_UN)


def save_current(state: dict[str, Any]) -> None:
    state["updated_at"] = iso_now()
    atomic_write_json(CURRENT_STATE, state)


def archive_state(state: dict[str, Any]) -> None:
    RUN_HISTORY.mkdir(parents=True, exist_ok=True)
    atomic_write_json(RUN_HISTORY / f"{state['run_id']}.json", state)


def load_current(*, required: bool = True) -> dict[str, Any] | None:
    if not CURRENT_STATE.is_file():
        if required:
            raise RunnerError("No active runner state exists. Run 'up' first.")
        return None
    try:
        return json.loads(CURRENT_STATE.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise RunnerError(f"Could not read runner state {CURRENT_STATE}: {error}") from error


def plan_spec(linode_type: str) -> PlanSpec:
    try:
        return PLAN_SPECS[linode_type]
    except KeyError as error:
        supported = ", ".join(sorted(PLAN_SPECS))
        raise RunnerError(
            f"Unsupported Linode type {linode_type!r}; supported types: {supported}"
        ) from error


def read_state_file(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise RunnerError(
            f"Could not verify high-memory usage from {path}: {error}"
        ) from error
    if not isinstance(value, dict):
        raise RunnerError(
            f"Could not verify high-memory usage from {path}: expected an object"
        )
    return value


def parked_state_path(linode_id: object) -> Path:
    try:
        identifier = int(linode_id)
    except (TypeError, ValueError) as error:
        raise RunnerError(f"Invalid parked Linode ID: {linode_id!r}") from error
    if identifier <= 0:
        raise RunnerError(f"Invalid parked Linode ID: {linode_id!r}")
    return PARKED_ROOT / f"{identifier}.json"


def list_parked_states() -> list[tuple[Path, dict[str, Any]]]:
    if not PARKED_ROOT.exists():
        return []
    if not PARKED_ROOT.is_dir():
        raise RunnerError(f"Could not inspect parked runners: {PARKED_ROOT} is not a directory")
    result: list[tuple[Path, dict[str, Any]]] = []
    for path in sorted(PARKED_ROOT.glob("*.json")):
        state = read_state_file(path)
        expected = parked_state_path(state.get("linode_id"))
        if path != expected:
            raise RunnerError(
                f"Parked state filename does not match its Linode ID: {path}"
            )
        if state.get("lifecycle") != "parked":
            raise RunnerError(f"Parked state has invalid lifecycle: {path}")
        result.append((path, state))
    return result


def save_parked(state: dict[str, Any]) -> Path:
    state["updated_at"] = iso_now()
    path = parked_state_path(state.get("linode_id"))
    atomic_write_json(path, state)
    return path


def high_memory_state_interval(
    state: dict[str, Any],
    *,
    now: datetime,
    active: bool,
    source: str,
) -> tuple[object, datetime, datetime] | None:
    """Extract a trusted high-memory billing interval from runner state."""

    if state.get("type") != HIGH_MEMORY_TYPE:
        return None
    linode_id = state.get("linode_id")
    if linode_id is None:
        return None
    created = state.get("linode_created_at")
    if not isinstance(created, str):
        raise RunnerError(
            f"Could not verify high-memory usage from {source}: "
            "missing trusted Linode creation time"
        )
    started_at = parse_time(created)
    if active:
        ended_at = now
    else:
        deleted = state.get("linode_deleted_at")
        if not isinstance(deleted, str):
            raise RunnerError(
                f"Could not verify high-memory usage from {source}: "
                "missing trusted Linode deletion time"
            )
        ended_at = parse_time(deleted)
    if ended_at < started_at:
        raise RunnerError(
            f"Could not verify high-memory usage from {source}: "
            "deletion precedes creation"
        )
    if started_at > now or ended_at > now:
        raise RunnerError(
            f"Could not verify high-memory usage from {source}: "
            "trusted interval extends into the future"
        )
    return linode_id, started_at, ended_at


def high_memory_usage(
    now: datetime,
    *,
    active_linodes: Iterable[dict[str, Any]] = (),
    history_root: Path | None = None,
    current_state_path: Path | None = None,
    parked_root: Path | None = None,
) -> HighMemoryUsage:
    """Replay trusted history and return current bank-adjusted usage."""

    history = RUN_HISTORY if history_root is None else history_root
    current = CURRENT_STATE if current_state_path is None else current_state_path
    parked = PARKED_ROOT if parked_root is None else parked_root
    intervals: dict[object, tuple[datetime, datetime]] = {}
    if history.exists():
        if not history.is_dir():
            raise RunnerError(
                f"Could not inspect high-memory runner history {history}: "
                "expected a directory"
            )
        try:
            history_files = sorted(history.glob("*.json"))
        except OSError as error:
            raise RunnerError(
                f"Could not inspect high-memory runner history {history}: {error}"
            ) from error
        for path in history_files:
            interval = high_memory_state_interval(
                read_state_file(path),
                now=now,
                active=False,
                source=str(path),
            )
            if interval is not None:
                key, started_at, ended_at = interval
                intervals[key] = (started_at, ended_at)
    if current.exists() and not current.is_file():
        raise RunnerError(
            f"Could not verify high-memory usage from {current}: "
            "expected a state file"
        )
    if current.is_file():
        interval = high_memory_state_interval(
            read_state_file(current),
            now=now,
            active=True,
            source=str(current),
        )
        if interval is not None:
            key, started_at, ended_at = interval
            intervals[key] = (started_at, ended_at)
    if parked.exists():
        if not parked.is_dir():
            raise RunnerError(
                f"Could not verify high-memory usage from {parked}: "
                "expected a directory"
            )
        for path in sorted(parked.glob("*.json")):
            interval = high_memory_state_interval(
                read_state_file(path),
                now=now,
                active=True,
                source=str(path),
            )
            if interval is not None:
                key, started_at, ended_at = interval
                intervals[key] = (started_at, ended_at)
    for linode in active_linodes:
        if (
            linode.get("type") != HIGH_MEMORY_TYPE
            or not is_managed_label(linode.get("label"))
        ):
            continue
        linode_id = linode.get("id")
        created = linode.get("created")
        if linode_id is None or not isinstance(created, str):
            raise RunnerError(
                "Could not verify high-memory usage from a live managed Linode: "
                "missing ID or trusted creation time"
            )
        started_at = parse_time(created)
        if now < started_at:
            raise RunnerError(
                "Could not verify high-memory usage from a live managed Linode: "
                "creation time is in the future"
            )
        intervals[linode_id] = (started_at, now)

    day_start, next_boundary = fixed_est_day_bounds(now)
    if intervals:
        first_day, _ = fixed_est_day_bounds(
            min(started_at for started_at, _ended_at in intervals.values())
        )
    else:
        first_day = day_start
    balance_at_start = HIGH_MEMORY_DAILY_LIMIT
    actual = timedelta()
    accounting_day = first_day
    while accounting_day <= day_start:
        following_day = accounting_day + timedelta(days=1)
        actual = timedelta()
        for started_at, ended_at in intervals.values():
            overlap_start = max(started_at, accounting_day)
            overlap_end = min(ended_at, following_day)
            if overlap_end > overlap_start:
                actual += overlap_end - overlap_start
        if accounting_day == day_start:
            break
        balance_at_start = min(
            HIGH_MEMORY_DAILY_LIMIT,
            balance_at_start + HIGH_MEMORY_DAILY_LIMIT - actual,
        )
        accounting_day = following_day
    return HighMemoryUsage(
        actual=actual,
        balance_at_start=balance_at_start,
        day_start=day_start,
        next_boundary=next_boundary,
    )


def format_duration(value: timedelta) -> str:
    total_seconds = max(0, int(value.total_seconds()))
    hours, remainder = divmod(total_seconds, 3600)
    minutes, seconds = divmod(remainder, 60)
    return f"{hours:02d}:{minutes:02d}:{seconds:02d}"


def report_high_memory_usage(usage: HighMemoryUsage) -> None:
    day = usage.day_start.astimezone(FIXED_EST).date().isoformat()
    boundary_est = usage.next_boundary.astimezone(FIXED_EST).isoformat()
    boundary_utc = format_utc(usage.next_boundary)
    print(f"High-memory usage for {day} fixed EST (UTC-05:00)")
    print(f"Daily base allowance: {format_duration(HIGH_MEMORY_DAILY_LIMIT)}")
    print(f"Banked usage at start of day: {format_duration(usage.banked_at_start)}")
    print(f"Usage debt at start of day: {format_duration(usage.debt_at_start)}")
    print(f"Adjusted daily capacity: {format_duration(usage.capacity)}")
    print(f"Actual Linode lifetime today: {format_duration(usage.actual)}")
    print(
        "Remaining before new starts are blocked: "
        f"{format_duration(usage.remaining)}"
    )
    print(f"Next accounting boundary: {boundary_est} ({boundary_utc} UTC)")
    print(
        "Projected bank at next boundary if no further usage accrues: "
        f"{format_duration(usage.banked_at_next_boundary)}"
    )
    print(
        "Projected debt at next boundary if no further usage accrues: "
        f"{format_duration(usage.debt_at_next_boundary)}"
    )
    if usage.exhausted:
        eligible_est = usage.projected_eligible_at.astimezone(FIXED_EST).isoformat()
        eligible_utc = format_utc(usage.projected_eligible_at)
        print(
            "Projected earliest new start if no further usage accrues: "
            f"{eligible_est} ({eligible_utc} UTC)"
        )


def require_high_memory_allowance(usage: HighMemoryUsage) -> None:
    if usage.exhausted:
        raise RunnerError(
            "High-memory usage has reached today's bank-adjusted capacity "
            f"of {format_duration(usage.capacity)}; "
            "no new high-memory run may start now. If no further usage accrues, "
            f"the projected earliest eligible boundary is "
            f"{usage.projected_eligible_at.astimezone(FIXED_EST).isoformat()} "
            "fixed EST"
        )


def inspect_high_memory_allowance(
    api: LinodeApi,
    *,
    active_linodes: Iterable[dict[str, Any]] | None = None,
) -> HighMemoryUsage:
    trusted_now = api.trusted_now()
    live = (
        api.list_all("/linode/instances")
        if active_linodes is None
        else active_linodes
    )
    usage = high_memory_usage(trusted_now, active_linodes=live)
    report_high_memory_usage(usage)
    return usage


SAFE_REAPER_ACCOUNT_ROLES = {
    "account_event_viewer",
    "account_maintenance_viewer",
    "account_notification_viewer",
    "account_oauth_client_admin",
}


def billing_delete_at(created_at: datetime, now: datetime) -> datetime:
    """Return the guarded end of the current rounded billing hour."""

    created = created_at.astimezone(timezone.utc)
    trusted_now = now.astimezone(timezone.utc)
    elapsed = max(0.0, (trusted_now - created).total_seconds())
    bucket = math.floor(elapsed / BILLING_HOUR.total_seconds()) + 1
    return created + bucket * BILLING_HOUR - REAPER_MARGIN


def load_reaper_config(*, required: bool = True) -> dict[str, Any] | None:
    if not REAPER_CONFIG.is_file():
        if required:
            raise RunnerError(
                "Restricted reaper configuration is missing. Run "
                "'linode-runner.ps1 init-reaper --username NAME'."
            )
        return None
    config = read_state_file(REAPER_CONFIG)
    username = config.get("username")
    valid_username = (
        isinstance(username, str)
        and re.fullmatch(
            r"[A-Za-z0-9][A-Za-z0-9_-]{1,30}[A-Za-z0-9]",
            username,
        )
        is not None
    )
    if not valid_username:
        raise RunnerError(f"Invalid restricted reaper username in {REAPER_CONFIG}")
    return config


def reaper_token(*, required: bool = True) -> str | None:
    token = os.environ.get("LINODE_REAPER_TOKEN", "")
    if not token:
        if required:
            raise RunnerError(
                "Restricted reaper token is unavailable. Run "
                "'linode-runner.ps1 init-reaper --username NAME'."
            )
        return None
    return token


def desired_reaper_access(states: Iterable[dict[str, Any]]) -> list[dict[str, Any]]:
    access: list[dict[str, Any]] = []
    for state in states:
        access.extend(
            [
                {
                    "id": int(state["firewall_id"]),
                    "type": "firewall",
                    "roles": ["firewall_admin"],
                },
                {
                    "id": int(state["linode_id"]),
                    "type": "linode",
                    "roles": ["linode_admin"],
                },
            ]
        )
    return sorted(access, key=lambda item: (str(item["type"]), int(item["id"])))


def validate_reaper_access(
    current: dict[str, Any],
    *,
    allowed_states: Iterable[dict[str, Any]],
) -> list[str]:
    account_access = current.get("account_access", [])
    entity_access = current.get("entity_access", [])
    if not isinstance(account_access, list) or not all(
        isinstance(value, str) for value in account_access
    ):
        raise RunnerError("Restricted reaper user returned invalid account roles")
    unexpected_accounts = sorted(set(account_access) - SAFE_REAPER_ACCOUNT_ROLES)
    if unexpected_accounts:
        raise RunnerError(
            "Restricted reaper user has unsafe account roles: "
            + ", ".join(unexpected_accounts)
        )
    allowed = {
        (str(item["type"]), int(item["id"])): tuple(item["roles"])
        for item in desired_reaper_access(allowed_states)
    }
    if not isinstance(entity_access, list):
        raise RunnerError("Restricted reaper user returned invalid entity roles")
    for item in entity_access:
        if not isinstance(item, dict):
            raise RunnerError("Restricted reaper user returned invalid entity role")
        try:
            key = (str(item["type"]), int(item["id"]))
            roles = tuple(sorted(str(value) for value in item["roles"]))
        except (KeyError, TypeError, ValueError) as error:
            raise RunnerError(
                "Restricted reaper user returned malformed entity access"
            ) from error
        expected = allowed.get(key)
        if expected is None or roles != tuple(sorted(expected)):
            raise RunnerError(
                "Restricted reaper user has unexpected entity access: "
                f"{key[0]} {key[1]} roles={list(roles)}"
            )
    return account_access


def reaper_access_path() -> str:
    config = load_reaper_config()
    assert config is not None
    username = urllib.parse.quote(str(config["username"]), safe="")
    return f"/iam/users/{username}/role-permissions"


def sync_reaper_access(
    api: LinodeApi,
    desired_states: Iterable[dict[str, Any]],
    *,
    allowed_existing_states: Iterable[dict[str, Any]] = (),
) -> None:
    desired = list(desired_states)
    allowed = [*desired, *allowed_existing_states]
    path = reaper_access_path()
    current = api.get(path)
    account_access = validate_reaper_access(current, allowed_states=allowed)
    payload = {
        "account_access": account_access,
        "entity_access": desired_reaper_access(desired),
    }
    api.put(path, payload)
    restricted = LinodeApi(token=reaper_token())
    for state in desired:
        label = str(state["label"])
        for kind, resource_path in (
            ("Linode", f"/linode/instances/{int(state['linode_id'])}"),
            (
                "firewall",
                f"/networking/firewalls/{int(state['firewall_id'])}",
            ),
        ):
            live = restricted.get(resource_path)
            require_managed_label(live.get("label"), label, kind)


def validate_reaper_setup(api: LinodeApi) -> None:
    """Validate, without changing, the optional restricted reaper setup."""

    config = load_reaper_config(required=False)
    token = reaper_token(required=False)
    if config is None and token is None:
        print("Restricted reaper: not configured; down/run will delete immediately")
        return
    if config is None or token is None:
        raise RunnerError(
            "Restricted reaper setup is incomplete; rerun 'init-reaper'"
        )
    parked = [state for _path, state in list_parked_states()]
    current = api.get(reaper_access_path())
    validate_reaper_access(current, allowed_states=parked)
    restricted = LinodeApi(token=token)
    restricted.get("/profile")
    for state in parked:
        for resource_path in (
            f"/linode/instances/{int(state['linode_id'])}",
            f"/networking/firewalls/{int(state['firewall_id'])}",
        ):
            restricted.get(resource_path)
    print(
        f"Restricted reaper: OK ({config['username']}; "
        f"{len(parked)} parked runner(s))"
    )


def remote_reaper_names(state: dict[str, Any]) -> tuple[str, str, PurePosixPath]:
    linode_id = int(state["linode_id"])
    base = f"umlaut-linode-reaper-{linode_id}"
    return f"{base}.service", f"{base}.timer", REMOTE_REAPER_ROOT / str(linode_id)


def remote_reaper_unit_files(state: dict[str, Any]) -> tuple[str, str]:
    service, timer, root = remote_reaper_names(state)
    delete_at = parse_time(str(state["delete_at"]))
    on_calendar = delete_at.strftime("%Y-%m-%d %H:%M:%S UTC")
    service_text = f"""[Unit]
Description=Delete parked Umlaut Linode {int(state['linode_id'])}

[Service]
Type=oneshot
ExecStart=/usr/bin/flock -n /run/{service}.lock /usr/bin/python3 {root}/remote_reaper.py --state {root}/state.json --token-file {root}/token
Restart=on-failure
RestartSec=60s
"""
    timer_text = f"""[Unit]
Description=Billing-boundary cleanup for parked Umlaut Linode {int(state['linode_id'])}

[Timer]
OnCalendar={on_calendar}
Persistent=true
AccuracySec=1s
Unit={service}

[Install]
WantedBy=timers.target
"""
    return service_text, timer_text


def arm_remote_reaper(state: dict[str, Any]) -> None:
    token = reaper_token()
    assert token is not None
    service, timer, root = remote_reaper_names(state)
    service_text, timer_text = remote_reaper_unit_files(state)
    remote_stage = PurePosixPath("/root") / f".{state['label']}-reaper-stage"
    ssh_command(
        state,
        f"rm -rf {shlex.quote(str(remote_stage))} && "
        f"install -d -m 0700 {shlex.quote(str(remote_stage))}",
        timeout=60,
    )
    with tempfile.TemporaryDirectory(prefix="umlaut-reaper-") as temporary:
        staging = Path(temporary)
        files = {
            "remote_reaper.py": REMOTE_REAPER_SOURCE.read_text(encoding="utf-8"),
            "state.json": json.dumps(
                {
                    key: state[key]
                    for key in (
                        "linode_id",
                        "firewall_id",
                        "label",
                        "lease_id",
                        "delete_at",
                    )
                },
                indent=2,
                sort_keys=True,
            )
            + "\n",
            "token": token + "\n",
            service: service_text,
            timer: timer_text,
        }
        for name, contents in files.items():
            local_path = staging / name
            local_path.write_text(contents, encoding="utf-8")
            if name == "token":
                local_path.chmod(0o600)
            scp_to(state, local_path, f"{remote_stage}/{name}")
    install = " && ".join(
        [
            f"install -d -m 0700 {shlex.quote(str(root))}",
            f"install -m 0700 {remote_stage}/remote_reaper.py {root}/remote_reaper.py",
            f"install -m 0600 {remote_stage}/state.json {root}/state.json",
            f"install -m 0600 {remote_stage}/token {root}/token",
            f"install -m 0644 {remote_stage}/{service} /etc/systemd/system/{service}",
            f"install -m 0644 {remote_stage}/{timer} /etc/systemd/system/{timer}",
            f"rm -rf {shlex.quote(str(remote_stage))}",
            "systemctl daemon-reload",
            f"systemctl enable --now {timer}",
        ]
    )
    ssh_command(state, install, timeout=120)
    state["remote_reaper"] = {
        "service": service,
        "timer": timer,
        "root": str(root),
    }


def disarm_remote_reaper(state: dict[str, Any]) -> None:
    service, timer, root = remote_reaper_names(state)
    command = " ; ".join(
        [
            f"systemctl disable --now {timer} >/dev/null 2>&1 || true",
            f"systemctl stop {service} >/dev/null 2>&1 || true",
            f"rm -f /etc/systemd/system/{timer} /etc/systemd/system/{service}",
            f"rm -rf {shlex.quote(str(root))}",
            "systemctl daemon-reload",
        ]
    )
    ssh_command(state, command, timeout=120)
    state.pop("remote_reaper", None)


def cleanup_remote_workspace(state: dict[str, Any]) -> None:
    uploaded = state.get("uploaded_files", [])
    uploaded_paths: list[str] = []
    paths = [
        str(REMOTE_ROOT / name)
        for name in (
            "source",
            "incoming",
            "previous",
            "artifacts",
            "compat-cache",
        )
    ]
    label = str(state.get("label", ""))
    if is_managed_label(label):
        paths.append(f"/root/{label}-source.tar.gz")
    workload_id = state.get("workload_id")
    if isinstance(workload_id, str):
        paths.append(f"/root/{resource_label(workload_id)}-artifacts.tar.gz")
    if isinstance(uploaded, list):
        uploaded_paths = [
            validate_remote_upload_path(value)
            for value in uploaded
            if isinstance(value, str)
        ]
        paths.extend(uploaded_paths)
    quoted = " ".join(shlex.quote(value) for value in paths)
    process_patterns = [f"*{REMOTE_ROOT}/*"]
    process_patterns.extend(f"*:{value}*" for value in uploaded_paths)
    joined_patterns = "|".join(process_patterns)
    script = f"""
set -Eeuo pipefail
terminate_workspace_processes() {{
    signal="$1"
    for process in /proc/[0-9]*; do
        pid="${{process#/proc/}}"
        if test "$pid" = "$$" -o "$pid" = "$PPID"; then
            continue
        fi
        cwd="$(readlink "$process/cwd" 2>/dev/null || true)"
        executable="$(readlink "$process/exe" 2>/dev/null || true)"
        case "$cwd:$executable" in
            {joined_patterns})
                kill "-$signal" "$pid" 2>/dev/null || true
                ;;
        esac
    done
}}
terminate_workspace_processes TERM
sleep 2
terminate_workspace_processes KILL
rm -rf -- {quoted}
"""
    ssh_command(state, script, timeout=180)
    state.pop("uploaded_files", None)


def command_path(name: str) -> str:
    found = shutil.which(name)
    if not found:
        raise RunnerError(f"Required local command is unavailable: {name}")
    return found


def run_local(
    command: Sequence[str],
    *,
    timeout: int | None = None,
    capture: bool = False,
) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        list(command),
        text=True,
        encoding="utf-8",
        errors="replace",
        stdout=subprocess.PIPE if capture else None,
        stderr=subprocess.PIPE if capture else None,
        timeout=timeout,
        check=False,
    )
    if result.returncode != 0:
        detail = ""
        if capture:
            detail = f"\n{result.stdout}{result.stderr}".rstrip()
        raise RunnerError(
            f"Command failed with exit code {result.returncode}: {command[0]}{detail}"
        )
    return result


def ensure_ssh_key() -> Path:
    private_key = SSH_KEY
    public_key = Path(f"{SSH_KEY}.pub")
    if private_key.is_file() and public_key.is_file():
        return private_key
    if private_key.exists() or public_key.exists():
        raise RunnerError(
            f"Incomplete SSH key pair at {private_key}; move both remnants aside and retry"
        )
    private_key.parent.mkdir(parents=True, exist_ok=True)
    run_local(
        [
            command_path("ssh-keygen"),
            "-q",
            "-t",
            "ed25519",
            "-N",
            "",
            "-C",
            "umlaut-linode-runner",
            "-f",
            str(private_key),
        ]
    )
    return private_key


def read_public_key() -> str:
    ensure_ssh_key()
    value = Path(f"{SSH_KEY}.pub").read_text(encoding="utf-8").strip()
    if not value.startswith("ssh-ed25519 "):
        raise RunnerError(f"Unexpected public key format in {SSH_KEY}.pub")
    return value


def detect_public_ipv4(override: str | None = None) -> str:
    if override:
        candidate = override.removesuffix("/32")
    else:
        try:
            with urllib.request.urlopen("https://api.ipify.org", timeout=20) as response:
                candidate = response.read().decode("ascii").strip()
        except (urllib.error.URLError, UnicodeDecodeError) as error:
            raise RunnerError(
                "Could not detect the controller public IPv4; pass --allow-ip"
            ) from error
    try:
        address = ipaddress.ip_address(candidate)
    except ValueError as error:
        raise RunnerError(f"Invalid controller IP address: {candidate}") from error
    if address.version != 4:
        raise RunnerError("The runner currently requires an IPv4 address for SSH")
    return f"{address}/32"


def ssh_options(state: dict[str, Any], *, connect_timeout: int = 10) -> list[str]:
    known_hosts = LOCAL_ROOT / f"known-hosts-{state['run_id']}"
    return [
        "-i",
        str(SSH_KEY),
        "-o",
        "BatchMode=yes",
        "-o",
        "IdentitiesOnly=yes",
        "-o",
        "StrictHostKeyChecking=accept-new",
        "-o",
        f"UserKnownHostsFile={known_hosts}",
        "-o",
        f"ConnectTimeout={connect_timeout}",
        "-o",
        "ServerAliveInterval=15",
        "-o",
        "ServerAliveCountMax=4",
    ]


def ssh_command(
    state: dict[str, Any],
    command: str,
    *,
    timeout: int | None = None,
    capture: bool = False,
) -> subprocess.CompletedProcess[str]:
    remote = f"bash -lc {shlex.quote(command)}"
    return run_local(
        [
            command_path("ssh"),
            *ssh_options(state),
            f"root@{state['ipv4']}",
            remote,
        ],
        timeout=timeout,
        capture=capture,
    )


def scp_to(state: dict[str, Any], source: Path, destination: str) -> None:
    run_local(
        [
            command_path("scp"),
            *ssh_options(state),
            str(source),
            f"root@{state['ipv4']}:{destination}",
        ],
        timeout=1800,
    )


def scp_from(state: dict[str, Any], source: str, destination: Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    run_local(
        [
            command_path("scp"),
            *ssh_options(state),
            f"root@{state['ipv4']}:{source}",
            str(destination),
        ],
        timeout=1800,
    )


def validate_remote_file_path(value: str) -> str:
    """Accept a conservative absolute POSIX file path for SCP transfers."""

    path = PurePosixPath(value)
    if (
        not path.is_absolute()
        or ".." in path.parts
        or not re.fullmatch(r"/[A-Za-z0-9._/-]+", value)
    ):
        raise RunnerError(
            "Remote transfer paths must be absolute and contain only "
            "letters, digits, '.', '_', '-', and '/'"
        )
    return value


def validate_remote_upload_path(value: str) -> str:
    remote_path = validate_remote_file_path(value)
    remote_parts = PurePosixPath(remote_path).parts
    if (
        len(remote_parts) < 3
        or remote_parts[1] != "root"
        or remote_parts[2].startswith(".")
    ):
        raise RunnerError(
            "Upload destinations must be non-hidden paths beneath /root so "
            "parking can sanitize them safely"
        )
    return remote_path


def upload_file(state: dict[str, Any], source: Path, destination: str) -> None:
    source = source.resolve()
    if not source.is_file():
        raise RunnerError(f"Upload source is not a file: {source}")
    remote_path = validate_remote_upload_path(destination)
    scp_to(state, source, remote_path)
    uploaded = state.setdefault("uploaded_files", [])
    if not isinstance(uploaded, list):
        raise RunnerError("Active runner state has invalid uploaded-files metadata")
    if remote_path not in uploaded:
        uploaded.append(remote_path)
    save_current(state)


def download_file(
    state: dict[str, Any], source: str, destination: Path, *, overwrite: bool
) -> None:
    source = validate_remote_file_path(source)
    destination = destination.resolve()
    if destination.exists() and not overwrite:
        raise RunnerError(
            f"Download destination already exists: {destination}; pass --overwrite"
        )
    scp_from(state, source, destination)


def snapshot_metadata(repo_root: Path) -> dict[str, Any]:
    def git_output(repository: Path, *arguments: str) -> str | None:
        try:
            return run_local(
                [
                    command_path("git"),
                    "-c",
                    f"safe.directory={repository}",
                    "-C",
                    str(repository),
                    *arguments,
                ],
                timeout=60,
                capture=True,
            ).stdout.strip()
        except RunnerError:
            return None

    return {
        "created_at": iso_now(),
        "root_commit": git_output(repo_root, "rev-parse", "HEAD"),
        "eprover_commit": git_output(repo_root / "eprover", "rev-parse", "HEAD"),
    }


def create_snapshot(repo_root: Path, destination: Path) -> dict[str, Any]:
    """Archive current files, including uncommitted and ignored eprover sources."""
    metadata = snapshot_metadata(repo_root)
    destination.parent.mkdir(parents=True, exist_ok=True)
    exclude_patterns = [
        ".git",
        "*/.git",
        "*/.git/*",
        ".dolt",
        "*/.dolt",
        "*/.dolt/*",
        ".venv",
        "*/.venv",
        "*/.venv/*",
        "target",
        "*/target",
        "*/target/*",
        "debug",
        "*/debug",
        "*/debug/*",
        ".artifacts",
        "*/.artifacts",
        "*/.artifacts/*",
        ".agents",
        "*/.agents",
        "*/.agents/*",
        ".claude",
        "*/.claude",
        "*/.claude/*",
        ".codex",
        "*/.codex",
        "*/.codex/*",
        ".beads",
        "*/.beads",
        "*/.beads/*",
        "cadical",
        "cadical/*",
        "gmp-6.3.0",
        "gmp-6.3.0/*",
        "minisat",
        "minisat/*",
        "problems",
        "problems/*",
        "vampire",
        "vampire/*",
        "z3",
        "z3/*",
        "__pycache__",
        "*/__pycache__",
        "*/__pycache__/*",
        ".beads-credential-key",
        "*.pyc",
        "*.pyo",
        "*.pdb",
    ]
    with tempfile.TemporaryDirectory(prefix="e-rust-manifest-") as manifest_temp:
        manifest_dir = Path(manifest_temp)
        (manifest_dir / ".linode-snapshot.json").write_text(
            json.dumps(metadata, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        archive_command = [
            command_path("tar"),
            "-czf",
            destination.as_posix(),
            *(argument for pattern in exclude_patterns for argument in ("--exclude", pattern)),
            "-C",
            repo_root.as_posix(),
            ".",
            "-C",
            manifest_dir.as_posix(),
            ".linode-snapshot.json",
        ]
        run_local(archive_command, timeout=1800, capture=True)
    listing = run_local(
        [command_path("tar"), "-tzf", destination.as_posix()],
        timeout=600,
        capture=True,
    ).stdout.splitlines()
    file_count = sum(
        1
        for item in listing
        if item
        and not item.endswith("/")
        and item.removeprefix("./") != ".linode-snapshot.json"
    )
    digest = hashlib.sha256()
    with destination.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    metadata["archive_sha256"] = digest.hexdigest()
    metadata["file_count"] = file_count
    metadata["archive_bytes"] = destination.stat().st_size
    return metadata


def safe_extract(archive_path: Path, destination: Path) -> None:
    destination.mkdir(parents=True, exist_ok=True)
    root = destination.resolve()
    with tarfile.open(archive_path, "r:gz") as archive:
        members = archive.getmembers()
        for member in members:
            if member.issym() or member.islnk():
                raise RunnerError(f"Refusing link in downloaded archive: {member.name}")
            target = (destination / member.name).resolve()
            try:
                target.relative_to(root)
            except ValueError as error:
                raise RunnerError(
                    f"Refusing unsafe downloaded archive path: {member.name}"
                ) from error
        if sys.version_info >= (3, 12):
            archive.extractall(destination, members=members, filter="data")
        else:
            archive.extractall(destination, members=members)


def validate_catalog(
    api: LinodeApi, linode_type: str, region: str, image: str
) -> None:
    expected = plan_spec(linode_type)
    type_info = api.get(f"/linode/types/{linode_type}")
    actual = (
        type_info.get("memory"),
        type_info.get("vcpus"),
        type_info.get("disk"),
        type_info.get("class"),
    )
    wanted = (
        expected.memory,
        expected.vcpus,
        expected.disk,
        expected.plan_class,
    )
    if actual != wanted:
        raise RunnerError(
            f"{linode_type} no longer matches the expected {expected.label} "
            f"catalog values (memory={expected.memory}, vcpus={expected.vcpus}, "
            f"disk={expected.disk}, class={expected.plan_class})"
        )
    api.get(f"/regions/{region}")
    api.get(f"/images/{image}")
    availability = api.get(f"/regions/{region}/availability")
    availability_items = (
        availability
        if isinstance(availability, list)
        else availability.get("data", [])
    )
    available_types = {
        item.get("plan")
        for item in availability_items
        if item.get("available") is True
    }
    if linode_type not in available_types:
        raise RunnerError(f"{linode_type} is not currently available in {region}")


def wait_for_linode(
    api: LinodeApi,
    linode_id: int,
    *,
    desired: str = "running",
    timeout: int = 900,
) -> dict[str, Any]:
    deadline = time.monotonic() + timeout
    last_status = "unknown"
    while time.monotonic() < deadline:
        linode = api.get(f"/linode/instances/{linode_id}")
        last_status = str(linode.get("status"))
        if last_status == desired:
            return linode
        if last_status == "billing_suspension":
            raise RunnerError("Linode entered billing_suspension")
        time.sleep(5)
    raise RunnerError(
        f"Timed out waiting for Linode {linode_id} to become {desired}; "
        f"last status was {last_status}"
    )


def wait_for_ssh(state: dict[str, Any], timeout: int = 600) -> None:
    deadline = time.monotonic() + timeout
    last_error = ""
    while time.monotonic() < deadline:
        try:
            result = ssh_command(state, "true", timeout=20, capture=True)
            if result.returncode == 0:
                return
        except (RunnerError, subprocess.TimeoutExpired) as error:
            last_error = str(error)
        time.sleep(5)
    raise RunnerError(f"Timed out waiting for SSH: {last_error}")


def bootstrap_script() -> str:
    return r"""
set -Eeuo pipefail
export DEBIAN_FRONTEND=noninteractive
command -v cloud-init >/dev/null
cloud-init status --wait
install -d -m 0755 /opt/e-rust-port
maintenance_units=(
    apt-daily.timer
    apt-daily-upgrade.timer
    apt-daily.service
    apt-daily-upgrade.service
)
systemctl stop "${maintenance_units[@]}"
systemctl mask "${maintenance_units[@]}"
for unit in "${maintenance_units[@]}"; do
    systemctl reset-failed "$unit" >/dev/null 2>&1 || true
    active_state="$(systemctl show --property=ActiveState --value "$unit")"
    unit_file_state="$(systemctl show --property=UnitFileState --value "$unit")"
    printf '%s active=%s unit-file=%s\n' "$unit" "$active_state" "$unit_file_state"
    test "$active_state" = inactive
    test "$unit_file_state" = masked
done
apt-get update
apt-get install -y --no-install-recommends \
    build-essential ca-certificates curl file gawk gcc-mingw-w64-x86-64 \
    g++-mingw-w64-x86-64 git pkg-config python3 time valgrind
cadical_source=/opt/e-rust-port/cadical-3.0.1
cadical_commit=c60730422e758ef1cebe7aeddf2dda31c996bf04
git clone --quiet --depth=1 --branch rel-3.0.1 \
    https://github.com/arminbiere/cadical.git "$cadical_source"
test "$(git -C "$cadical_source" rev-parse HEAD)" = "$cadical_commit"
git -C "$cadical_source" fsck --strict
test "$(cat "$cadical_source/VERSION")" = 3.0.1
if ! command -v cargo >/dev/null 2>&1; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs |
        sh -s -- -y --profile minimal --default-toolchain stable
fi
/root/.cargo/bin/rustup component add rustfmt clippy
/root/.cargo/bin/rustup target add x86_64-pc-windows-gnu
/root/.cargo/bin/rustc --version
/root/.cargo/bin/cargo --version
/root/.cargo/bin/cargo fmt --version
/root/.cargo/bin/cargo clippy --version
gcc --version | head -n 1
x86_64-w64-mingw32-gcc --version | head -n 1
x86_64-w64-mingw32-g++-posix --version | head -n 1
valgrind --version
python3 - /opt/e-rust-port/package-maintenance-quiescence.json \
    "${maintenance_units[@]}" <<'PY'
import json
import os
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path


def property_value(unit: str, name: str) -> str:
    result = subprocess.run(
        ["systemctl", "show", f"--property={name}", "--value", unit],
        check=True,
        stdout=subprocess.PIPE,
        text=True,
    )
    return result.stdout.strip()


record_path = Path(sys.argv[1])
units = sys.argv[2:]
record = {
    "schema_version": 1,
    "captured_at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
    "cloud_init_wait_completed": True,
    "units": {
        unit: {
            "active_state": property_value(unit, "ActiveState"),
            "unit_file_state": property_value(unit, "UnitFileState"),
        }
        for unit in units
    },
}
temporary = record_path.with_suffix(".json.tmp")
temporary.write_text(
    json.dumps(record, indent=2, sort_keys=True) + "\n",
    encoding="utf-8",
)
os.replace(temporary, record_path)
PY
"""


def validate_package_maintenance_record(value: object) -> dict[str, Any]:
    """Validate the fail-closed package-maintenance quiescence record."""

    if not isinstance(value, dict) or value.get("schema_version") != 1:
        raise RunnerError("Invalid package-maintenance quiescence record schema")
    if value.get("cloud_init_wait_completed") is not True:
        raise RunnerError("Package-maintenance record did not confirm cloud-init wait")
    units = value.get("units")
    if not isinstance(units, dict) or set(units) != set(PACKAGE_MAINTENANCE_UNITS):
        raise RunnerError("Package-maintenance record has an unexpected unit set")
    for unit in PACKAGE_MAINTENANCE_UNITS:
        status = units.get(unit)
        if not isinstance(status, dict):
            raise RunnerError(f"Package-maintenance record is missing {unit} state")
        if status.get("active_state") != "inactive":
            raise RunnerError(f"Package-maintenance unit {unit} is not inactive")
        if status.get("unit_file_state") != "masked":
            raise RunnerError(f"Package-maintenance unit {unit} is not masked")
    return value


def bootstrap(state: dict[str, Any]) -> dict[str, Any]:
    ssh_command(state, bootstrap_script(), timeout=1800)
    result = ssh_command(
        state,
        f"cat {shlex.quote(str(PACKAGE_MAINTENANCE_RECORD))}",
        timeout=30,
        capture=True,
    )
    try:
        record = json.loads(result.stdout)
    except json.JSONDecodeError as error:
        raise RunnerError(
            "Could not parse package-maintenance quiescence record"
        ) from error
    validated = validate_package_maintenance_record(record)
    return {
        "path": str(PACKAGE_MAINTENANCE_RECORD),
        "sha256": hashlib.sha256(result.stdout.encode("utf-8")).hexdigest(),
        "record": validated,
    }


def provision(
    api: LinodeApi,
    *,
    allow_ip: str | None = None,
    linode_type: str = DEFAULT_TYPE,
    region: str = DEFAULT_REGION,
    image: str = DEFAULT_IMAGE,
    prevalidated: bool = False,
) -> dict[str, Any]:
    if CURRENT_STATE.exists():
        existing = load_current()
        raise RunnerError(
            f"Active state already exists for {existing['label']}; "
            "run status/down before creating another runner"
        )
    if not prevalidated:
        validate_catalog(api, linode_type, region, image)
        if linode_type == HIGH_MEMORY_TYPE:
            usage = inspect_high_memory_allowance(api)
            require_high_memory_allowance(usage)
    command_path("ssh")
    command_path("scp")
    ensure_ssh_key()
    allow_cidr = detect_public_ipv4(allow_ip)
    print(
        f"Provisioning {linode_type} in {region}; SSH source is {allow_cidr}",
        flush=True,
    )
    identifier = run_id()
    label = resource_label(identifier)
    state: dict[str, Any] = {
        "run_id": identifier,
        "label": label,
        "created_at": iso_now(),
        "lifecycle": "active",
        "phase": "creating-firewall",
        "allow_cidr": allow_cidr,
        "type": linode_type,
        "region": region,
        "image": image,
    }
    save_current(state)
    try:
        print(f"Creating firewall {label}", flush=True)
        firewall = api.post(
            "/networking/firewalls", firewall_payload(label, allow_cidr)
        )
        state["firewall_id"] = int(firewall["id"])
        state["phase"] = "creating-linode"
        save_current(state)
        print(f"Creating Linode {label}", flush=True)
        linode = api.post(
            "/linode/instances",
            linode_payload(
                label,
                state["firewall_id"],
                read_public_key(),
                linode_type=linode_type,
                region=region,
                image=image,
            ),
        )
        state["linode_id"] = int(linode["id"])
        created = linode.get("created")
        if not isinstance(created, str):
            raise RunnerError(
                "Created Linode did not include a trusted creation timestamp"
            )
        state["linode_created_at"] = format_utc(parse_time(created))
        state["phase"] = "provisioning"
        addresses = linode.get("ipv4", [])
        if addresses:
            state["ipv4"] = addresses[0]
        save_current(state)
        print(f"Waiting for Linode {state['linode_id']} to boot", flush=True)
        linode = wait_for_linode(api, state["linode_id"])
        addresses = linode.get("ipv4", [])
        if not addresses:
            raise RunnerError("Provisioned Linode has no public IPv4 address")
        state["ipv4"] = addresses[0]
        state["phase"] = "waiting-for-ssh"
        save_current(state)
        print(f"Waiting for SSH at {state['ipv4']}", flush=True)
        wait_for_ssh(state)
        state["phase"] = "bootstrapping"
        save_current(state)
        print("Installing the Linux build and Callgrind toolchain", flush=True)
        state["package_maintenance"] = bootstrap(state)
        state["phase"] = "ready"
        save_current(state)
        return state
    except BaseException:
        state["phase"] = "provision-failed"
        save_current(state)
        try:
            delete_state_resources(api, state)
        except Exception as cleanup_error:
            print(
                f"URGENT: automatic cleanup also failed: {cleanup_error}",
                file=sys.stderr,
            )
        raise


def compatible_runner(
    state: dict[str, Any],
    *,
    linode_type: str,
    region: str,
    image: str,
) -> bool:
    return (
        state.get("type") == linode_type
        and state.get("region") == region
        and state.get("image") == image
    )


def parked_delete_at(state: dict[str, Any]) -> datetime:
    value = state.get("delete_at")
    if not isinstance(value, str):
        raise RunnerError(
            f"Parked runner {state.get('label', '<unknown>')} has no deletion deadline"
        )
    return parse_time(value)


def remaining_parked_states(*, exclude_linode_id: int | None = None) -> list[dict[str, Any]]:
    return [
        state
        for _path, state in list_parked_states()
        if exclude_linode_id is None or int(state["linode_id"]) != exclude_linode_id
    ]


def process_is_alive(process_id: int) -> bool:
    if process_id <= 0:
        return False
    try:
        os.kill(process_id, 0)
    except OSError:
        return False
    return True


def claim_provision() -> str:
    """Claim the single active slot while a fresh runner is being created."""

    if PROVISION_CLAIM.is_file():
        existing = read_state_file(PROVISION_CLAIM)
        process_id = existing.get("pid")
        if isinstance(process_id, int) and process_is_alive(process_id):
            raise RunnerError(
                f"Another runner acquisition is in progress (PID {process_id})"
            )
        PROVISION_CLAIM.unlink()
    owner = uuid.uuid4().hex
    atomic_write_json(
        PROVISION_CLAIM,
        {"owner": owner, "pid": os.getpid(), "created_at": iso_now()},
    )
    return owner


def release_provision_claim(owner: str) -> None:
    if not PROVISION_CLAIM.is_file():
        return
    existing = read_state_file(PROVISION_CLAIM)
    if existing.get("owner") == owner:
        PROVISION_CLAIM.unlink()


def reap_parked_state(
    api: LinodeApi,
    path: Path,
    state: dict[str, Any],
    *,
    lease_id: str | None = None,
    force: bool = False,
) -> bool:
    if lease_id is not None and state.get("lease_id") != lease_id:
        return False
    if not force and api.trusted_now() < parked_delete_at(state):
        return False
    state["phase"] = "deleting"
    atomic_write_json(path, state)
    delete_state_resources(api, state, state_path=path)
    if load_reaper_config(required=False) is not None and reaper_token(required=False):
        sync_reaper_access(
            api,
            remaining_parked_states(exclude_linode_id=int(state["linode_id"])),
            allowed_existing_states=[state],
        )
    return True


def activate_parked_runner(
    api: LinodeApi,
    path: Path,
    state: dict[str, Any],
    *,
    allow_ip: str | None,
) -> dict[str, Any] | None:
    linode_id = int(state["linode_id"])
    label = str(state["label"])
    live = api.get(f"/linode/instances/{linode_id}", allow_404=True)
    if live is None:
        delete_state_resources(api, state, state_path=path)
        return None
    require_managed_label(live.get("label"), label, "Linode")
    firewall_id = int(state["firewall_id"])
    firewall = api.get(f"/networking/firewalls/{firewall_id}", allow_404=True)
    if firewall is None:
        delete_state_resources(api, state, state_path=path)
        return None
    require_managed_label(firewall.get("label"), label, "firewall")
    if live.get("status") in {"offline", "stopped"}:
        api.post(f"/linode/instances/{linode_id}/boot", {})
        live = wait_for_linode(api, linode_id)
    elif live.get("status") != "running":
        live = wait_for_linode(api, linode_id)
    addresses = live.get("ipv4", state.get("ipv4", []))
    if isinstance(addresses, list) and addresses:
        state["ipv4"] = addresses[0]
    state["allow_cidr"] = replace_firewall_rules(api, state, allow_ip)
    atomic_write_json(path, state)
    wait_for_ssh(state)
    disarm_remote_reaper(state)
    state["lifecycle"] = "active"
    state["phase"] = "ready"
    state.pop("delete_at", None)
    state.pop("lease_id", None)
    state.pop("parked_at", None)
    state["reuse_count"] = int(state.get("reuse_count", 0)) + 1
    atomic_write_json(path, state)
    os.replace(path, CURRENT_STATE)
    try:
        sync_reaper_access(
            api,
            remaining_parked_states(exclude_linode_id=linode_id),
            allowed_existing_states=[state],
        )
        state["phase"] = "ready"
        save_current(state)
        return state
    except BaseException:
        try:
            delete_state_resources(api, state)
        except Exception as cleanup_error:
            print(
                f"URGENT: failed reused-runner cleanup: {cleanup_error}",
                file=sys.stderr,
            )
        raise


def acquire_runner(
    api: LinodeApi,
    *,
    allow_ip: str | None = None,
    linode_type: str = DEFAULT_TYPE,
    region: str = DEFAULT_REGION,
    image: str = DEFAULT_IMAGE,
) -> tuple[dict[str, Any], bool]:
    validate_catalog(api, linode_type, region, image)
    if linode_type == HIGH_MEMORY_TYPE:
        require_high_memory_allowance(inspect_high_memory_allowance(api))
    trusted_now = api.trusted_now()
    claim_owner: str | None = None
    with lifecycle_lock():
        if CURRENT_STATE.exists():
            existing = load_current()
            raise RunnerError(
                f"Active state already exists for {existing['label']}; "
                "run status/down before acquiring another runner"
            )
        candidates: list[tuple[Path, dict[str, Any]]] = []
        for path, state in list_parked_states():
            if trusted_now + REUSE_DISARM_GUARD >= parked_delete_at(state):
                reap_parked_state(api, path, state, force=True)
                continue
            if compatible_runner(
                state,
                linode_type=linode_type,
                region=region,
                image=image,
            ):
                candidates.append((path, state))
        candidates.sort(
            key=lambda item: (parked_delete_at(item[1]), int(item[1]["linode_id"]))
        )
        while candidates:
            path, state = candidates.pop(0)
            reused = activate_parked_runner(
                api,
                path,
                state,
                allow_ip=allow_ip,
            )
            if reused is not None:
                print(
                    f"Reusing parked Linode {reused['linode_id']} "
                    f"({reused['label']})",
                    flush=True,
                )
                return reused, True
        claim_owner = claim_provision()
    try:
        state = provision(
            api,
            allow_ip=allow_ip,
            linode_type=linode_type,
            region=region,
            image=image,
            prevalidated=True,
        )
        return state, False
    finally:
        assert claim_owner is not None
        with lifecycle_lock():
            release_provision_claim(claim_owner)


def park_runner(api: LinodeApi, state: dict[str, Any]) -> bool:
    """Park an active runner; return False when it was deleted immediately."""

    linode_id = int(state["linode_id"])
    with lifecycle_lock():
        current = load_current()
        if int(current.get("linode_id", -1)) != linode_id:
            raise RunnerError("Active runner changed before it could be parked")
        if current.get("lifecycle", "active") != "active":
            raise RunnerError(
                f"Runner cannot be parked from lifecycle "
                f"{current.get('lifecycle')!r}"
            )
        state = current
        state["lifecycle"] = "parking"
        state["phase"] = "cleaning-for-park"
        save_current(state)
    path = f"/linode/instances/{linode_id}"
    live = api.get(path, allow_404=True)
    if live is None:
        delete_state_resources(api, state)
        return False
    require_managed_label(live.get("label"), state["label"], "Linode")
    created = live.get("created")
    if not isinstance(created, str):
        created = state.get("linode_created_at")
    if not isinstance(created, str):
        raise RunnerError("Live Linode has no trusted creation timestamp")
    state["linode_created_at"] = format_utc(parse_time(created))
    trusted_now = api.trusted_now()
    delete_at = billing_delete_at(parse_time(created), trusted_now)
    try:
        cleanup_remote_workspace(state)
    except BaseException:
        delete_state_resources(api, state)
        raise
    trusted_now = api.trusted_now()
    if trusted_now >= delete_at:
        print("Billing reuse cutoff has passed; deleting immediately.", flush=True)
        delete_state_resources(api, state)
        return False
    if load_reaper_config(required=False) is None or not reaper_token(required=False):
        print(
            "Restricted remote reaper is not configured; deleting immediately.",
            file=sys.stderr,
        )
        delete_state_resources(api, state)
        return False
    state["lease_id"] = uuid.uuid4().hex
    state["delete_at"] = format_utc(delete_at)
    state["parked_at"] = format_utc(trusted_now)
    state["lifecycle"] = "parked"
    state["phase"] = "arming-reapers"
    save_current(state)
    with lifecycle_lock():
        existing = remaining_parked_states()
    try:
        sync_reaper_access(api, [*existing, state])
        arm_remote_reaper(state)
    except BaseException:
        try:
            sync_reaper_access(
                api,
                existing,
                allowed_existing_states=[state],
            )
        except Exception as access_error:
            print(
                f"URGENT: could not roll back reaper access: {access_error}",
                file=sys.stderr,
            )
        delete_state_resources(api, state)
        raise
    state["phase"] = "parked"
    save_current(state)
    destination = parked_state_path(linode_id)
    with lifecycle_lock():
        current = load_current()
        if (
            int(current.get("linode_id", -1)) != linode_id
            or current.get("lease_id") != state["lease_id"]
        ):
            raise RunnerError("Active runner changed while reapers were armed")
        destination.parent.mkdir(parents=True, exist_ok=True)
        os.replace(CURRENT_STATE, destination)
    print(
        f"Runner parked until {state['delete_at']} UTC: "
        f"Linode {linode_id} ({state['label']})",
        flush=True,
    )
    return True


def sync_source(state: dict[str, Any], repo_root: Path = REPO_ROOT) -> dict[str, Any]:
    state["phase"] = "packaging-source"
    save_current(state)
    with tempfile.TemporaryDirectory(prefix="e-rust-linode-") as temporary:
        archive = Path(temporary) / "source.tar.gz"
        metadata = create_snapshot(repo_root, archive)
        remote_archive = f"/root/{state['label']}-source.tar.gz"
        print(
            f"Uploading {metadata['file_count']} files "
            f"({metadata['archive_bytes'] / (1024 * 1024):.1f} MiB)"
        )
        state["phase"] = "uploading-source"
        save_current(state)
        scp_to(state, archive, remote_archive)
        install_script = f"""
set -Eeuo pipefail
test -f {shlex.quote(remote_archive)}
rm -rf {REMOTE_ROOT}/incoming {REMOTE_ROOT}/previous
install -d -m 0755 {REMOTE_ROOT}/incoming
tar -xzf {shlex.quote(remote_archive)} -C {REMOTE_ROOT}/incoming
test -f {REMOTE_ROOT}/incoming/Cargo.toml
test -d {REMOTE_ROOT}/incoming/eprover
if test -d {REMOTE_ROOT}/source; then
    mv {REMOTE_ROOT}/source {REMOTE_ROOT}/previous
fi
mv {REMOTE_ROOT}/incoming {REMOTE_ROOT}/source
rm -rf {REMOTE_ROOT}/previous
rm -f {shlex.quote(remote_archive)}
"""
        ssh_command(state, install_script, timeout=900)
    state["snapshot"] = metadata
    state["phase"] = "synced"
    save_current(state)
    return metadata


def begin_workload(state: dict[str, Any]) -> str:
    identifier = run_id()
    state["workload_id"] = identifier
    state.pop("remote_artifact_path", None)
    state.pop("local_artifact_path", None)
    state["phase"] = "workload-starting"
    save_current(state)
    return identifier


def run_remote_workload(state: dict[str, Any]) -> None:
    commit = state.get("snapshot", {}).get("eprover_commit") or "worktree-snapshot"
    script_path = (
        f"{REMOTE_ROOT}/source/tools/linode-runner/remote_run.sh"
    )
    workload_id = state.get("workload_id")
    if not isinstance(workload_id, str):
        raise RunnerError("Active runner has no workload ID")
    workload_label = resource_label(workload_id)
    artifact_path = f"{REMOTE_ROOT}/artifacts/{workload_label}"
    command = " ".join(
        shlex.quote(value)
        for value in [
            "bash",
            script_path,
            str(REMOTE_ROOT / "source"),
            artifact_path,
            "4",
            commit,
        ]
    )
    state["phase"] = "running-workload"
    save_current(state)
    ssh_command(state, command, timeout=14400)
    state["remote_artifact_path"] = artifact_path
    state["phase"] = "workload-complete"
    save_current(state)


def collect_artifacts(state: dict[str, Any]) -> Path:
    workload_id = state.get("workload_id")
    if not isinstance(workload_id, str):
        raise RunnerError("Active runner has no workload ID")
    workload_label = resource_label(workload_id)
    remote_artifacts = state.get(
        "remote_artifact_path", f"{REMOTE_ROOT}/artifacts/{workload_label}"
    )
    remote_archive = f"/root/{workload_label}-artifacts.tar.gz"
    pack = (
        f"test -d {shlex.quote(remote_artifacts)} && "
        f"tar -C {shlex.quote(remote_artifacts)} -czf "
        f"{shlex.quote(remote_archive)} ."
    )
    ssh_command(state, pack, timeout=900)
    local_dir = ARTIFACT_ROOT / workload_id
    local_archive = local_dir / "artifacts.tar.gz"
    scp_from(state, remote_archive, local_archive)
    safe_extract(local_archive, local_dir)
    local_archive.unlink()
    ssh_command(
        state,
        f"rm -f {shlex.quote(remote_archive)} && "
        f"rm -rf {shlex.quote(str(remote_artifacts))}",
        timeout=60,
    )
    state["local_artifact_path"] = str(local_dir)
    state["phase"] = "artifacts-collected"
    save_current(state)
    return local_dir


def wait_until_absent(
    api: LinodeApi, path: str, *, timeout: int = 600
) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if api.get(path, allow_404=True) is None:
            return
        time.sleep(5)
    raise RunnerError(f"Timed out waiting for deletion: {path}")


def delete_state_resources(
    api: LinodeApi,
    state: dict[str, Any],
    *,
    state_path: Path | None = None,
) -> None:
    label = str(state["label"])
    if not is_managed_label(label):
        raise RunnerError(f"Refusing cleanup for unmanaged label: {label}")
    linode_id = state.get("linode_id")
    linode_deleted_at: datetime | None = None
    if linode_id is not None:
        path = f"/linode/instances/{int(linode_id)}"
        live = api.get(path, allow_404=True)
        if live is not None:
            require_managed_label(live.get("label"), label, "Linode")
            print(f"Deleting Linode {linode_id} ({label})")
            api.delete(path)
            wait_until_absent(api, path)
        linode_deleted_at = getattr(api, "last_response_at", None)
        if (
            state.get("type") == HIGH_MEMORY_TYPE
            and isinstance(linode_deleted_at, datetime)
        ):
            state["linode_deleted_at"] = format_utc(linode_deleted_at)
    firewall_id = state.get("firewall_id")
    if firewall_id is not None:
        path = f"/networking/firewalls/{int(firewall_id)}"
        live = api.get(path, allow_404=True)
        if live is not None:
            require_managed_label(live.get("label"), label, "firewall")
            print(f"Deleting firewall {firewall_id} ({label})")
            deadline = time.monotonic() + 300
            while True:
                try:
                    api.delete(path)
                    break
                except ApiError as error:
                    if time.monotonic() >= deadline:
                        raise
                    if error.status not in {400, 409}:
                        raise
                    time.sleep(5)
            wait_until_absent(api, path)
    state["phase"] = "deleted"
    state["lifecycle"] = "deleted"
    state["deleted_at"] = (
        format_utc(linode_deleted_at)
        if isinstance(linode_deleted_at, datetime)
        else iso_now()
    )
    archive_state(state)
    path = CURRENT_STATE if state_path is None else state_path
    if path.is_file():
        saved = read_state_file(path)
        if (
            saved.get("linode_id") == state.get("linode_id")
            and saved.get("label") == state.get("label")
        ):
            path.unlink()


def replace_firewall_rules(
    api: LinodeApi,
    state: dict[str, Any],
    allow_ip: str | None,
) -> str:
    allow_cidr = detect_public_ipv4(allow_ip)
    firewall_id = int(state["firewall_id"])
    live = api.get(f"/networking/firewalls/{firewall_id}")
    require_managed_label(live.get("label"), state["label"], "firewall")
    api.put(
        f"/networking/firewalls/{firewall_id}/rules",
        firewall_rules(allow_cidr),
    )
    return allow_cidr


def refresh_firewall(api: LinodeApi, state: dict[str, Any], allow_ip: str | None) -> None:
    allow_cidr = replace_firewall_rules(api, state, allow_ip)
    state["allow_cidr"] = allow_cidr
    save_current(state)
    print(f"Firewall now allows SSH from {allow_cidr}")


def status(api: LinodeApi) -> None:
    state = load_current(required=False)

    def with_live_status(value: dict[str, Any]) -> dict[str, Any]:
        result = dict(value)
        linode_id = value.get("linode_id")
        firewall_id = value.get("firewall_id")
        if linode_id is not None:
            live = api.get(f"/linode/instances/{linode_id}", allow_404=True)
            result["live_linode_status"] = (
                None if live is None else live.get("status")
            )
        if firewall_id is not None:
            live = api.get(
                f"/networking/firewalls/{firewall_id}", allow_404=True
            )
            result["live_firewall_status"] = (
                None if live is None else live.get("status")
            )
        return result

    result = {
        "active": None if state is None else with_live_status(state),
        "parked": [
            with_live_status(parked)
            for _path, parked in list_parked_states()
        ],
    }
    print(json.dumps(result, indent=2, sort_keys=True))


def managed_older_than(
    resources: Iterable[dict[str, Any]], cutoff: datetime
) -> list[dict[str, Any]]:
    candidates = []
    for resource in resources:
        created = resource.get("created")
        if not is_managed_label(resource.get("label")) or not isinstance(created, str):
            continue
        try:
            if parse_time(created) < cutoff:
                candidates.append(resource)
        except ValueError:
            continue
    return candidates


def garbage_collect(
    api: LinodeApi, *, older_than_hours: float, confirm: bool
) -> None:
    if older_than_hours < 1:
        raise RunnerError("--older-than-hours must be at least 1")
    cutoff = utc_now() - timedelta(hours=older_than_hours)
    protected_states = [load_current(required=False) or {}]
    protected_states.extend(state for _path, state in list_parked_states())
    protected_linodes = {state.get("linode_id") for state in protected_states}
    protected_firewalls = {state.get("firewall_id") for state in protected_states}
    linodes = [
        item
        for item in managed_older_than(
            api.list_all("/linode/instances"), cutoff
        )
        if item.get("id") not in protected_linodes
    ]
    firewalls = [
        item
        for item in managed_older_than(
            api.list_all("/networking/firewalls"), cutoff
        )
        if item.get("id") not in protected_firewalls
    ]
    if not linodes and not firewalls:
        print("No stale managed resources found.")
        return
    for item in linodes:
        print(f"stale Linode: {item['id']} {item['label']} {item['created']}")
    for item in firewalls:
        print(f"stale firewall: {item['id']} {item['label']} {item['created']}")
    if not confirm:
        print("Dry run only. Pass --yes to delete these resources.")
        return
    for item in linodes:
        path = f"/linode/instances/{int(item['id'])}"
        live = api.get(path, allow_404=True)
        if live is not None:
            require_managed_label(live.get("label"), item["label"], "Linode")
            api.delete(path)
            wait_until_absent(api, path)
    for item in firewalls:
        path = f"/networking/firewalls/{int(item['id'])}"
        live = api.get(path, allow_404=True)
        if live is not None:
            require_managed_label(live.get("label"), item["label"], "firewall")
            api.delete(path)
            wait_until_absent(api, path)


def initialize() -> None:
    ensure_ssh_key()
    command_path("ssh")
    command_path("scp")
    LOCAL_ROOT.mkdir(parents=True, exist_ok=True)
    RUN_HISTORY.mkdir(parents=True, exist_ok=True)
    PARKED_ROOT.mkdir(parents=True, exist_ok=True)
    print(f"Dedicated SSH key ready: {SSH_KEY}")
    print(f"Local runner state: {LOCAL_ROOT}")


def preflight(
    api: LinodeApi,
    *,
    allow_ip: str | None,
    linode_type: str,
    region: str,
    image: str,
) -> None:
    command_path("ssh")
    command_path("scp")
    command_path("tar")
    ensure_ssh_key()
    allow_cidr = detect_public_ipv4(allow_ip)
    validate_catalog(api, linode_type, region, image)
    active_linodes = api.list_all("/linode/instances")
    api.list_all("/networking/firewalls")
    print(f"Linode API and catalog: OK")
    print(f"Plan: {linode_type} in {region} using {image}")
    print(f"SSH firewall source: {allow_cidr}")
    if linode_type == HIGH_MEMORY_TYPE:
        usage = inspect_high_memory_allowance(
            api,
            active_linodes=active_linodes,
        )
        require_high_memory_allowance(usage)
    validate_reaper_setup(api)


def add_provision_arguments(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--allow-ip",
        help="public controller IPv4; detected automatically when omitted",
    )
    plan = parser.add_mutually_exclusive_group()
    plan.add_argument(
        "--type",
        choices=tuple(PLAN_SPECS),
        default=DEFAULT_TYPE,
        dest="linode_type",
        help="supported Linode type (advanced compatibility option)",
    )
    plan.add_argument(
        "--high-memory",
        action="store_const",
        const=HIGH_MEMORY_TYPE,
        dest="linode_type",
        help="use the guarded 150 GB high-memory profile",
    )
    parser.add_argument("--region", default=DEFAULT_REGION)
    parser.add_argument("--image", default=DEFAULT_IMAGE)


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(description=__doc__)
    commands = root.add_subparsers(dest="command", required=True)
    commands.add_parser("init", help="create the dedicated local SSH key")
    check = commands.add_parser(
        "check", help="validate local tools, API permissions, plan, and capacity"
    )
    add_provision_arguments(check)
    up = commands.add_parser("up", help="reuse or provision a compatible runner")
    add_provision_arguments(up)
    commands.add_parser("sync", help="upload a fresh current-worktree snapshot")
    upload = commands.add_parser(
        "upload", help="upload one explicit local artifact to the active runner"
    )
    upload.add_argument("local_path", type=Path)
    upload.add_argument("remote_path")
    download = commands.add_parser(
        "download", help="download one explicit artifact from the active runner"
    )
    download.add_argument("remote_path")
    download.add_argument("local_path", type=Path)
    download.add_argument("--overwrite", action="store_true")
    execute = commands.add_parser("exec", help="run a command on the active runner")
    execute.add_argument("remote_command", nargs=argparse.REMAINDER)
    refresh = commands.add_parser(
        "refresh-ip", help="replace the firewall SSH source address"
    )
    refresh.add_argument("--allow-ip")
    run = commands.add_parser(
        "run", help="acquire, sync, build/profile, collect, and park"
    )
    add_provision_arguments(run)
    run.add_argument(
        "--keep-on-failure",
        action="store_true",
        help="retain paid resources after a workload failure for debugging",
    )
    down = commands.add_parser("down", help="park the active runner for reuse")
    down.add_argument(
        "--now",
        action="store_true",
        help="delete the active runner immediately instead of parking it",
    )
    down.add_argument(
        "--all",
        action="store_true",
        help="delete the active runner and every parked runner immediately",
    )
    commands.add_parser("status", help="show local and live runner state")
    reap = commands.add_parser("reap", help=argparse.SUPPRESS)
    reap.add_argument("--linode-id", type=int, required=True)
    reap.add_argument("--lease-id", required=True)
    gc = commands.add_parser("gc", help="find stale resources with the managed prefix")
    gc.add_argument("--older-than-hours", type=float, default=6)
    gc.add_argument("--yes", action="store_true", help="delete listed resources")
    return root


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parser().parse_args(argv)
    try:
        if arguments.command == "init":
            initialize()
            return 0
        api = LinodeApi()
        if arguments.command == "check":
            preflight(
                api,
                allow_ip=arguments.allow_ip,
                linode_type=arguments.linode_type,
                region=arguments.region,
                image=arguments.image,
            )
        elif arguments.command == "up":
            state, reused = acquire_runner(
                api,
                allow_ip=arguments.allow_ip,
                linode_type=arguments.linode_type,
                region=arguments.region,
                image=arguments.image,
            )
            print(
                f"Runner ready ({'reused' if reused else 'new'}): {state['label']} "
                f"(Linode {state['linode_id']}, {state['ipv4']})"
            )
        elif arguments.command == "sync":
            state = load_current()
            metadata = sync_source(state)
            print(f"Snapshot uploaded: {metadata['archive_sha256']}")
        elif arguments.command == "upload":
            upload_file(
                load_current(), arguments.local_path, arguments.remote_path
            )
            print(f"Uploaded {arguments.local_path} to {arguments.remote_path}")
        elif arguments.command == "download":
            download_file(
                load_current(),
                arguments.remote_path,
                arguments.local_path,
                overwrite=arguments.overwrite,
            )
            print(f"Downloaded {arguments.remote_path} to {arguments.local_path}")
        elif arguments.command == "exec":
            state = load_current()
            remote_arguments = list(arguments.remote_command)
            if remote_arguments and remote_arguments[0] == "--":
                remote_arguments.pop(0)
            if not remote_arguments:
                raise RunnerError("Provide a remote command after 'exec --'")
            ssh_command(state, " ".join(remote_arguments))
        elif arguments.command == "refresh-ip":
            refresh_firewall(api, load_current(), arguments.allow_ip)
        elif arguments.command == "down":
            if arguments.all:
                current = load_current(required=False)
                if current is not None:
                    delete_state_resources(api, current)
                for path, parked in list_parked_states():
                    reap_parked_state(api, path, parked, force=True)
                if load_reaper_config(required=False) is not None and reaper_token(
                    required=False
                ):
                    sync_reaper_access(api, [])
                print("Active and parked runners deleted.")
            elif arguments.now:
                delete_state_resources(api, load_current())
                print("Runner and firewall deleted.")
            elif park_runner(api, load_current()):
                print("Runner is available for billing-aware reuse.")
            else:
                print("Runner and firewall deleted.")
        elif arguments.command == "status":
            status(api)
        elif arguments.command == "reap":
            path = parked_state_path(arguments.linode_id)
            with lifecycle_lock():
                if not path.is_file():
                    print("Parked runner is already absent.")
                else:
                    parked = read_state_file(path)
                    if reap_parked_state(
                        api,
                        path,
                        parked,
                        lease_id=arguments.lease_id,
                    ):
                        print("Parked runner and firewall deleted.")
                    else:
                        print("Parked runner lease is stale or not yet due.")
        elif arguments.command == "gc":
            garbage_collect(
                api,
                older_than_hours=arguments.older_than_hours,
                confirm=arguments.yes,
            )
        elif arguments.command == "run":
            state: dict[str, Any] | None = None
            workload_error: BaseException | None = None
            try:
                state, reused = acquire_runner(
                    api,
                    allow_ip=arguments.allow_ip,
                    linode_type=arguments.linode_type,
                    region=arguments.region,
                    image=arguments.image,
                )
                print(f"Using {'reused' if reused else 'new'} Linode {state['linode_id']}")
                sync_source(state)
                begin_workload(state)
                run_remote_workload(state)
                local_artifacts = collect_artifacts(state)
                print(f"Artifacts collected at {local_artifacts}")
            except BaseException as error:
                workload_error = error
                if state is not None:
                    try:
                        collect_artifacts(state)
                    except Exception as collect_error:
                        print(
                            f"Could not collect partial artifacts: {collect_error}",
                            file=sys.stderr,
                        )
                raise
            finally:
                if state is not None and not (
                    workload_error is not None and arguments.keep_on_failure
                ):
                    try:
                        if park_runner(api, state):
                            print("Runner parked for billing-aware reuse.")
                        else:
                            print("Runner and firewall deleted.")
                    except Exception as cleanup_error:
                        print(
                            "URGENT: cleanup failed; run 'down --now': "
                            f"{cleanup_error}",
                            file=sys.stderr,
                        )
                        if workload_error is None:
                            raise
        return 0
    except KeyboardInterrupt:
        print("Interrupted.", file=sys.stderr)
        return 130
    except (RunnerError, subprocess.TimeoutExpired) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
