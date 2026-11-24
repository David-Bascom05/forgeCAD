#!/usr/bin/env python3
"""Fail if any of the Autodesk-hosted maker tool libraries is tracked in this repository.

David's rule (2026-11-24): the makers' Fusion libraries from "Tools for Autodesk Fusion" are used
(tools/fetch-tool-libraries.sh, a git-ignored cache) and never committed: no files, no records, no
per-tool derived values (research/cam/tooling/libraries.md). Checks every tracked file:
  1. the cache directory (.cache/tool-libraries/) and the pinned libraries' file names;
  2. size + SHA-256 equal to a pinned library (read from tools/fetch-tool-libraries.sh);
  3. HSMWorks libraries (*.hsmlib) and Fusion .tools archives anywhere but RobbJack's, whose licence
     allows redistribution (research/cam/tooling/libraries/robbjack/);
  4. JSON whose head is a Fusion tool library (a "data" array of tools with "geometry" and
     "start-values"), outside RobbJack's directory.
Exit 1 with the offending paths, else 0. Run: python3 -I tools/ci/no-vendor-tool-libraries.py
"""
import hashlib
import os
import re
import subprocess
import sys

FETCH = "tools/fetch-tool-libraries.sh"
ALLOWED = "research/cam/tooling/libraries/robbjack/"
SELF = {"tools/ci/no-vendor-tool-libraries.py", FETCH}
PIN = re.compile(r'^\s*"([^|"]+)\|[^|]+\|(\d+)\|([0-9a-f]{64})"', re.M)
FUSION = re.compile(rb'"(start-values|post-process)"')
GEOM = re.compile(rb'"geometry"\s*:')


def main():
    pins = PIN.findall(open(FETCH).read())
    names = {n for n, _, _ in pins}
    by_size = {}
    for n, b, h in pins:
        by_size.setdefault(int(b), set()).add(h)
    files = subprocess.run(["git", "ls-files", "-z"], check=True, capture_output=True).stdout.split(b"\0")
    bad = []
    for raw in files:
        if not raw:
            continue
        p = raw.decode("utf-8", "replace")
        if p in SELF:
            continue
        why = None
        base = os.path.basename(p)
        if p.startswith(".cache/tool-libraries/") or "/.cache/tool-libraries/" in p:
            why = "the library cache"
        elif base in names:
            why = "a pinned library's file name"
        elif not p.startswith(ALLOWED) and (p.endswith(".hsmlib") or p.endswith(".tools")):
            why = "a CAM tool library archive"
        else:
            try:
                size = os.path.getsize(raw)
            except OSError:
                continue
            if size in by_size:
                with open(raw, "rb") as f:
                    if hashlib.sha256(f.read()).hexdigest() in by_size[size]:
                        why = "identical to a pinned library"
            if not why and p.endswith(".json") and not p.startswith(ALLOWED):
                with open(raw, "rb") as f:
                    head = f.read(65536)
                if b'"data"' in head and GEOM.search(head) and FUSION.search(head):
                    why = "a Fusion tool library"
        if why:
            bad.append(f"{p}: {why}")
    if bad:
        print("The makers' Autodesk-hosted tool libraries must never be committed (use them from the", file=sys.stderr)
        print("git-ignored cache, tools/fetch-tool-libraries.sh). Remove:", file=sys.stderr)
        for b in bad:
            print("  " + b, file=sys.stderr)
        return 1
    print(f"no vendor tool libraries among {len([f for f in files if f])} tracked files")
    return 0


if __name__ == "__main__":
    sys.exit(main())
