#!/usr/bin/env python3
"""The Lake build cache of one CI job: what to cache and under which key.

    tools/ci/lake-cache.py NAME PROJECT_DIR...

Writes to $GITHUB_OUTPUT (or prints, outside Actions):
  paths   every directory Lake builds into for these projects: each project's .lake/build, the
          .lake/build of each of its path dependencies (Lake builds a path dependency in its own
          directory), and each git package that Mathlib's cache does not provide (Aeneas's Lean
          library), whole. Mathlib and its own dependencies are not cached here: `lake exe cache
          get` fetches their prebuilt files from Mathlib's server.
  key     NAME, the toolchains and manifests, then the git trees of the projects and their path
          dependencies: equal keys mean equal sources.
  prefix  the key up to the sources: the latest cache for the same toolchains and manifests.
  any     NAME alone: the latest cache of this job at all.

A restored cache only saves work: Lake checks every file it restores against the hash of its
inputs (sources, imports, toolchain) and rebuilds what differs, so the result of `lake build`
is the result of a build from scratch. Lake also replays the messages of a module it does not
rebuild (warnings, `#print axioms`), so a check that reads the build log sees the same text.
"""
import hashlib
import json
import os
import subprocess
import sys

# The packages `lake exe cache get` provides prebuilt (Mathlib's dependency closure).
MATHLIB_CACHED = {"mathlib", "batteries", "Qq", "aesop", "proofwidgets", "importGraph",
                  "LeanSearchClient", "plausible", "Cli"}


def git_tree(path):
    return subprocess.run(["git", "rev-parse", f"HEAD:{path}"], check=True, capture_output=True,
                          text=True).stdout.strip()


def read(path):
    try:
        with open(path, "rb") as f:
            return f.read()
    except FileNotFoundError:
        return b""


def main():
    if len(sys.argv) < 3:
        sys.exit(__doc__)
    name, roots = sys.argv[1], [os.path.normpath(r) for r in sys.argv[2:]]
    paths, sources, cfg = [], [], hashlib.sha256()
    for root in roots:
        paths.append(f"{root}/.lake/build")
        sources.append(root)
        for f in ("lean-toolchain", "lake-manifest.json", "lakefile.toml", "lakefile.lean"):
            cfg.update(f"{root}/{f}\0".encode() + read(f"{root}/{f}") + b"\0")
        manifest = json.loads(read(f"{root}/lake-manifest.json") or b"{}")
        pkgs = manifest.get("packagesDir", ".lake/packages")
        for p in manifest.get("packages", []):
            if p.get("type") == "path":
                dep = os.path.normpath(os.path.join(root, p["dir"]))
                paths.append(f"{dep}/.lake/build")
                sources.append(dep)
            elif p["name"] not in MATHLIB_CACHED:
                paths.append(os.path.normpath(os.path.join(root, pkgs, p["name"])))
    paths = sorted(set(paths))
    src = hashlib.sha256()
    for s in sorted(set(sources)):
        src.update(f"{s} {git_tree(s)}\n".encode())
    anyk = f"lake-v1-{name}/"  # "/" ends the name: loft-ssi-proofs is not a prefix of loft-ssi-proofs-alt
    prefix = f"{anyk}{cfg.hexdigest()[:16]}-"
    key = f"{prefix}{src.hexdigest()[:16]}"
    out = f"key={key}\nprefix={prefix}\nany={anyk}\npaths<<EOF\n" + "\n".join(paths) + "\nEOF\n"
    if os.environ.get("GITHUB_OUTPUT"):
        with open(os.environ["GITHUB_OUTPUT"], "a") as f:
            f.write(out)
    print(out, end="")


if __name__ == "__main__":
    main()
