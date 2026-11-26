#!/usr/bin/env python3
# What the repository may hold, by size and type (David: models need no licence or documentation
# record; the guard is against bulk). A tracked file is refused when
#   - it is a raw archive (ARCHIVES: a vendor's download committed whole): unpack what is needed,
#     or leave it out;
#   - or it is bigger than MAX_FILE_MB: decimate a mesh, trim a dataset, or keep it out (Git LFS
#     only as decided with the coordinator).
# A file stored in Git LFS (.gitattributes filter=lfs: the machine models, the workholding models)
# is measured as what the repository holds, its pointer in the index, whether or not its content
# is pulled into the work tree; an archive is refused there too.
# MODEL-LICENCE.toml and tools/ci/own-models.txt, where they exist, are information only.
#   tools/ci/model-licences.py [ROOT]      prints every refusal with its fix; exit 1 on any
import os, subprocess, sys

ARCHIVES = {"zip", "7z", "rar"}
MAX_FILE_MB = 20


def files(root):
    out = subprocess.run(["git", "ls-files", "-z"], cwd=root, check=True, stdout=subprocess.PIPE).stdout
    return [f for f in out.decode().split("\0") if f]


def in_lfs(root, fs):
    """The paths of fs that .gitattributes puts in Git LFS (filter=lfs)."""
    if not fs:
        return set()
    out = subprocess.run(["git", "check-attr", "-z", "filter", "--stdin"], cwd=root, check=True,
                         input="\0".join(fs).encode() + b"\0", stdout=subprocess.PIPE).stdout
    parts = out.decode().split("\0")
    # (path, attribute, value) triples
    return {parts[i] for i in range(0, len(parts) - 2, 3) if parts[i + 2] == "lfs"}


def index_mb(root, f):
    """The size of f's blob in the index (an LFS file's pointer), in MB."""
    out = subprocess.run(["git", "cat-file", "-s", f":{f}"], cwd=root, check=True, stdout=subprocess.PIPE).stdout
    return int(out) / 1e6


def main(root):
    bad = []
    fs = files(root)
    lfs = in_lfs(root, fs)
    for f in fs:
        base = os.path.basename(f)
        ext = base.rsplit(".", 1)[-1].lower() if "." in base else ""
        if ext in ARCHIVES:
            bad.append(f"{f}: a .{ext} archive: commit what is needed from it, unpacked, or nothing")
        p = os.path.join(root, f)
        # (a tracked path deleted in the work tree, or a submodule: nothing to measure)
        if os.path.isfile(p) and not os.path.islink(p):
            mb = index_mb(root, f) if f in lfs else os.path.getsize(p) / 1e6
            if mb > MAX_FILE_MB:
                bad.append(
                    f"{f}: {mb:.1f} MB, over {MAX_FILE_MB} MB: decimate or trim it, or keep it out "
                    f"(Git LFS only as decided with the coordinator)"
                )
    for b in bad:
        print(b)
    print(f"model-licences (size and type guard): {len(bad)} refusal(s)")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else "."))
