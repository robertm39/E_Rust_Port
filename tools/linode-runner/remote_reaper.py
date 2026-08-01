#!/usr/bin/env python3
"""Delete one parked Umlaut Linode from inside the instance."""

from __future__ import annotations

import argparse
import json
import re
import sys
import urllib.error
import urllib.request
from datetime import datetime, timezone
from email.utils import parsedate_to_datetime
from pathlib import Path
from typing import Any, Sequence


API_BASE = "https://api.linode.com/v4"
LABEL_PREFIX = "e-rust-codex-"
MARKER_PREFIX = "umlaut-reaper-"


class ReaperError(RuntimeError):
    """A safety or API failure while reaping one parked runner."""


class LinodeApi:
    """Minimal Linode API client used by the remote timer."""

    def __init__(self, token: str, base_url: str = API_BASE):
        if not token:
            raise ReaperError("The restricted reaper token is empty")
        self.token = token
        self.base_url = base_url.rstrip("/")
        self.last_response_at: datetime | None = None

    def request(
        self,
        method: str,
        path: str,
        payload: dict[str, Any] | None = None,
        *,
        allow_404: bool = False,
    ) -> Any:
        body = None if payload is None else json.dumps(payload).encode("utf-8")
        headers = {
            "Accept": "application/json",
            "Authorization": f"Bearer {self.token}",
            "User-Agent": "umlaut-remote-reaper/1",
        }
        if body is not None:
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
                self.last_response_at = parse_http_date(response.headers.get("Date"))
        except urllib.error.HTTPError as error:
            try:
                self.last_response_at = parse_http_date(error.headers.get("Date"))
            except (AttributeError, ReaperError):
                self.last_response_at = None
            if allow_404 and error.code == 404:
                return None
            detail = error.read().decode("utf-8", errors="replace")
            raise ReaperError(
                f"{method} {path} failed ({error.code}): {detail}"
            ) from error
        except urllib.error.URLError as error:
            raise ReaperError(f"Could not reach the Linode API: {error.reason}") from error
        if not response_body:
            return None
        return json.loads(response_body.decode("utf-8"))

    def get(self, path: str, *, allow_404: bool = False) -> Any:
        return self.request("GET", path, allow_404=allow_404)

    def put(self, path: str, payload: dict[str, Any]) -> Any:
        return self.request("PUT", path, payload)

    def delete(self, path: str, *, allow_404: bool = False) -> Any:
        return self.request("DELETE", path, allow_404=allow_404)

    def trusted_now(self) -> datetime:
        self.request("HEAD", "/regions")
        if self.last_response_at is None:
            raise ReaperError("Linode API response did not provide trusted time")
        return self.last_response_at


def parse_http_date(value: str | None) -> datetime:
    if not value:
        raise ReaperError("Linode API response did not include a Date header")
    try:
        parsed = parsedate_to_datetime(value)
    except (TypeError, ValueError) as error:
        raise ReaperError(f"Invalid Linode API Date header: {value!r}") from error
    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=timezone.utc)
    return parsed.astimezone(timezone.utc)


def parse_time(value: str) -> datetime:
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as error:
        raise ReaperError(f"Invalid reaper deletion time: {value!r}") from error
    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=timezone.utc)
    return parsed.astimezone(timezone.utc)


def read_secret(path: Path) -> str:
    try:
        mode = path.stat().st_mode & 0o777
        if mode & 0o077:
            raise ReaperError(f"Restricted token file is not private: {path}")
        token = path.read_text(encoding="utf-8").strip()
    except OSError as error:
        raise ReaperError(f"Could not read restricted token file {path}: {error}") from error
    if not token:
        raise ReaperError(f"Restricted token file is empty: {path}")
    return token


def read_state(path: Path) -> dict[str, Any]:
    try:
        state = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ReaperError(f"Could not read reaper state {path}: {error}") from error
    if not isinstance(state, dict):
        raise ReaperError(f"Reaper state is not an object: {path}")
    required = ("linode_id", "firewall_id", "label", "lease_id", "delete_at")
    if any(field not in state for field in required):
        raise ReaperError(f"Reaper state is missing required fields: {path}")
    label = state["label"]
    lease_id = state["lease_id"]
    if not isinstance(label, str) or not label.startswith(LABEL_PREFIX):
        raise ReaperError(f"Refusing unmanaged reaper label: {label!r}")
    if not isinstance(lease_id, str) or re.fullmatch(r"[0-9a-f]{32}", lease_id) is None:
        raise ReaperError(f"Invalid reaper lease ID: {lease_id!r}")
    for field in ("linode_id", "firewall_id"):
        try:
            identifier = int(state[field])
        except (TypeError, ValueError) as error:
            raise ReaperError(f"Invalid reaper {field}: {state[field]!r}") from error
        if identifier <= 0:
            raise ReaperError(f"Invalid reaper {field}: {state[field]!r}")
        state[field] = identifier
    if not isinstance(state["delete_at"], str):
        raise ReaperError(f"Invalid reaper deletion time: {state['delete_at']!r}")
    parse_time(state["delete_at"])
    return state


def require_label(resource: Any, expected: str, kind: str) -> dict[str, Any]:
    if not isinstance(resource, dict):
        raise ReaperError(f"Invalid live {kind} response")
    actual = resource.get("label")
    if actual != expected:
        raise ReaperError(
            f"Refusing to manage {kind}: live label {actual!r} "
            f"does not match {expected!r}"
        )
    return resource


def marker(lease_id: str, stage: str) -> str:
    return f"{MARKER_PREFIX}{lease_id}-{stage}"


def mark_firewall(
    api: LinodeApi,
    firewall_id: int,
    firewall: dict[str, Any],
    lease_id: str,
    stage: str,
) -> None:
    tags = [
        value
        for value in firewall.get("tags", [])
        if isinstance(value, str) and not value.startswith(MARKER_PREFIX)
    ]
    tags.append(marker(lease_id, stage))
    api.put(f"/networking/firewalls/{firewall_id}", {"tags": tags})


def reap(api: LinodeApi, state: dict[str, Any]) -> bool:
    """Delete the assigned Linode after exact live-resource verification."""

    if api.trusted_now() < parse_time(str(state["delete_at"])):
        raise ReaperError("Refusing to reap before the trusted deletion deadline")
    linode_id = int(state["linode_id"])
    firewall_id = int(state["firewall_id"])
    label = str(state["label"])
    lease_id = str(state["lease_id"])
    linode_path = f"/linode/instances/{linode_id}"
    linode = api.get(linode_path, allow_404=True)
    if linode is None:
        return False
    require_label(linode, label, "Linode")
    firewall_path = f"/networking/firewalls/{firewall_id}"
    firewall = require_label(api.get(firewall_path), label, "firewall")
    mark_firewall(api, firewall_id, firewall, lease_id, "attempt")
    api.delete(linode_path)
    try:
        refreshed = require_label(api.get(firewall_path), label, "firewall")
        mark_firewall(api, firewall_id, refreshed, lease_id, "accepted")
    except ReaperError:
        # The deletion request is the bill-stopping operation.  The controller
        # can conservatively reconcile an attempt marker if this VM disappears
        # before the best-effort completion marker reaches the free firewall.
        pass
    return True


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    result.add_argument("--state", type=Path, required=True)
    result.add_argument("--token-file", type=Path, required=True)
    return result


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parser().parse_args(argv)
    try:
        state = read_state(arguments.state)
        api = LinodeApi(read_secret(arguments.token_file))
        reap(api, state)
        return 0
    except ReaperError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
