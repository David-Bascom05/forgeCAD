#!/usr/bin/env python3
# What the repository may hold, by size and type (David: models need no licence or documentation
# record; the guard is against bulk). A tracked file is refused when
#   - it is a raw archive (ARCHIVES: a vendor's download committed whole): unpack what is needed,
#     or leave it out;
#   - or it is bigger than MAX_FILE_MB: decimate a mesh, trim a dataset, or keep it out (Git LFS
#     only as decided with the coordinator).
# MODEL-LICENCE.toml and tools/ci/own-models.txt, where they exist, are information only.
#   tools/ci/model-licences.py [ROOT]      prints every refusal with its fix; exit 1 on any
import os, subprocess, sys

ARCHIVES = {"zip", "7z", "rar"}
MAX_FILE_MB = 20


def files(root):
    out = subprocess.run(["git", "ls-files", "-z"], cwd=root, check=True, stdout=subprocess.PIPE).stdout
    return [f for f in out.decode().split("\0") if f]


def main(root):
    bad = []
    for f in files(root):
        base = os.path.basename(f)
        ext = base.rsplit(".", 1)[-1].lower() if "." in base else ""
        if ext in ARCHIVES:
            bad.append(f"{f}: a .{ext} archive: commit what is needed from it, unpacked, or nothing")
        p = os.path.join(root, f)
        # (a tracked path deleted in the work tree, or a submodule: nothing to measure)
        if os.path.isfile(p) and not os.path.islink(p):
            mb = os.path.getsize(p) / 1e6
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
