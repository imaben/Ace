#!/usr/bin/env python3
"""List license metadata for the Ace dependency graph.

Writes a stable text inventory to stdout. Run from the repository root:

    python scripts/list-third-party-licenses.py > THIRD_PARTY_LICENSES.txt
"""

from __future__ import annotations

import json
import subprocess
import sys
from datetime import datetime, timezone


def cargo_metadata():
    last_error = None
    for args in (
        ["cargo", "metadata", "--format-version", "1", "--offline"],
        ["cargo", "metadata", "--format-version", "1"],
    ):
        try:
            completed = subprocess.run(
                args,
                check=True,
                capture_output=True,
                text=True,
                encoding="utf-8",
                errors="replace",
            )
            raw = completed.stdout or ""
            idx = raw.find("{")
            if idx < 0:
                raise RuntimeError("cargo metadata did not print JSON")
            return json.loads(raw[idx:])
        except (subprocess.CalledProcessError, RuntimeError) as exc:
            last_error = exc
            continue
    raise SystemExit(f"cargo metadata failed: {last_error}")


def walk_graph(data):
    workspace_members = set(data.get("workspace_members", []))
    resolve = {node["id"]: node for node in data.get("resolve", {}).get("nodes", [])}
    seen = set()
    stack = list(workspace_members)
    while stack:
        pkg_id = stack.pop()
        if pkg_id in seen:
            continue
        seen.add(pkg_id)
        node = resolve.get(pkg_id)
        if not node:
            continue
        for dep in node.get("deps", []):
            stack.append(dep["pkg"])
    return workspace_members, seen


def main():
    data = cargo_metadata()
    workspace_members, seen = walk_graph(data)
    packages = {pkg["id"]: pkg for pkg in data["packages"]}
    rows = []
    for pkg_id in seen:
        pkg = packages[pkg_id]
        license_expr = (pkg.get("license") or "").strip()
        if not license_expr:
            license_file = (pkg.get("license_file") or "").strip()
            license_expr = f"license-file:{license_file}" if license_file else "UNKNOWN"
        rows.append(
            {
                "kind": "workspace" if pkg_id in workspace_members else "dependency",
                "name": pkg["name"],
                "version": pkg["version"],
                "license": license_expr,
                "repository": (pkg.get("repository") or "").strip() or "-",
            }
        )
    rows.sort(key=lambda r: (r["kind"] != "workspace", r["name"].lower(), r["version"]))

    now = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M UTC")
    print("# Third-party license inventory for Ace")
    print(f"# Generated: {now}")
    print("# Source: cargo metadata (crate license / license-file fields)")
    print("# This file lists metadata only; see each crate for full license text.")
    print("#")
    print("# kind\tname\tversion\tlicense\trepository")
    for row in rows:
        print(
            "{kind}\t{name}\t{version}\t{license}\t{repository}".format(**row)
        )
    print(f"# package_count={len(rows)}")


if __name__ == "__main__":
    sys.exit(main())
