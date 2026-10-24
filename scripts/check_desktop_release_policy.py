#!/usr/bin/env python3
"""Fail-closed, dependency-free checks for the unsigned AutoTrace preview build.

Static checks are a defense in depth, not a substitute for cargo audit, real
Windows acceptance, or tests with a real Google account.
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FILES = (
    "desktop/src-tauri/tauri.conf.json",
    "desktop/src-tauri/capabilities/default.json",
    "desktop/src-tauri/src/drive_oauth.rs",
    "desktop/src-tauri/Cargo.lock",
    ".github/workflows/desktop-installer.yml",
    ".github/workflows/desktop-foundation.yml",
)

def failures(root: Path = ROOT) -> list[str]:
    content = {}
    failures_found = []
    for rel in FILES:
        try:
            content[rel] = (root / rel).read_text(encoding="utf-8")
        except OSError:
            failures_found.append(f"Missing release input: {rel}")
    if failures_found:
        return failures_found

    config = json.loads(content["desktop/src-tauri/tauri.conf.json"])
    cap = json.loads(content["desktop/src-tauri/capabilities/default.json"])
    security = config.get("app", {}).get("security", {})
    csp = security.get("csp", "")
    if "unsafe-eval" in csp or "script-src 'self'" not in csp:
        failures_found.append("Tauri scripts must be self-only without unsafe-eval")
    if "connect-src ipc: http://ipc.localhost" not in csp:
        failures_found.append("Desktop IPC must be explicitly constrained")
    if cap.get("windows") != ["main"] or cap.get("permissions") != ["core:default"]:
        failures_found.append("Native Tauri capability permissions expanded: security review required")
    if config.get("bundle", {}).get("active") is not True or "nsis" not in config.get("bundle", {}).get("targets", []):
        failures_found.append("Windows NSIS bundle configuration missing")

    oauth = content["desktop/src-tauri/src/drive_oauth.rs"]
    scope = re.search(r'pub\(crate\) const SCOPE:\s*&str\s*=\s*"([^"]+)"', oauth)
    if not scope or scope.group(1) != "https://www.googleapis.com/auth/drive.file":
        failures_found.append("Google scope changed: only drive.file is approved")
    if "code_challenge_method\",\"S256\"" not in oauth or "state\",&state" not in oauth:
        failures_found.append("OAuth PKCE S256/state contract changed: review required")
    if re.search(r'(?i)client_secret\s*[:=]\s*["\']', oauth):
        failures_found.append("OAuth client secret must not be bundled with AutoTrace")

    lock = content["desktop/src-tauri/Cargo.lock"]
    if "[[package]]" not in lock or not re.search(r"(?m)^version = 4$", lock):
        failures_found.append("A versioned Cargo.lock is required")
    if re.search(r'(?m)^source = "git\+http:', lock):
        failures_found.append("Unencrypted git dependency source in Cargo.lock")

    installer = content[".github/workflows/desktop-installer.yml"]
    foundation = content[".github/workflows/desktop-foundation.yml"]
    for name, workflow in (("installer", installer), ("foundation", foundation)):
        if "pull_request_target:" in workflow or "secrets." in workflow:
            failures_found.append(f"Unsafe GitHub Actions event or secret use in {name}")
        if "permissions:\n  contents: read" not in workflow:
            failures_found.append(f"Unexpected GitHub Actions permissions in {name}")
        if "desktop/**" not in workflow or "tests/**" not in workflow:
            failures_found.append(f"Missing Desktop/test change coverage in {name}")
    if "UNSIGNED" not in installer or "retention-days: 7" not in installer:
        failures_found.append("Unsigned preview artifact must be clearly identified and short-lived")
    if "sha256.txt" not in installer or "smoke-installer.ps1" not in installer:
        failures_found.append("NSIS installer checksum or installation smoke missing")
    upload_section = installer.split("name: Upload preview installer", 1)
    if len(upload_section) != 2 or re.search(r"\*\.sqlite|\*\.xlsx|\.atbackup\.age", upload_section[1]):
        failures_found.append("Release artifacts may expose private workshop data")

    return failures_found

if __name__ == "__main__":
    errors = failures()
    if errors:
        for issue in errors:
            print(f"RELEASE POLICY FAIL: {issue}", file=sys.stderr)
        sys.exit(1)
    print("RELEASE POLICY PASS: local security and distribution invariants")
