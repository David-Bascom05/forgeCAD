#!/usr/bin/env python3
# Every 3D model file the repository commits is ours or licensed to be here (David's rule): a
# maker's or vendor's model may be committed only where its licence explicitly permits
# redistribution (or redistribution as part of software), recorded beside it. Anything else stays
# the user's own file, attached in the Machine Builder and kept in their library
# (forge_machine_builder::models), never committed.
#
# A committed file with a model extension passes when
#   - tools/ci/own-models.txt lists it (a glob per line, `# why` after it): files forgecad or this
#     repository made (exports of our own parts, test fixtures written here). Never under
#     kernel/forge-machine/data/: a machine's model there is a vendor's and needs (b).
#   - or a MODEL-LICENCE.toml in its folder has a [[file]] for it with
#       name, maker, url (where the terms were read), read (YYYY-MM-DD), terms (the licence's exact
#       words on redistribution, quoted), redistribution = "allowed" | "allowed-in-software"
# and it is no bigger than MAX_FILE_MB, and its folder's licensed models no more than MAX_DIR_MB
# together (decimate the mesh; a bigger set needs Git LFS, decided with the coordinator).
#   tools/ci/model-licences.py [ROOT]      prints every refusal with its fix; exit 1 on any
import fnmatch, os, re, subprocess, sys, tomllib

EXTS = {
    "step", "stp", "igs", "iges", "stl", "x_t", "x_b", "xmt_txt", "xmt_bin", "3mf", "obj",
    "glb", "gltf", "jt", "sldprt", "sldasm", "f3d", "ipt", "iam", "catpart", "catproduct",
    "prt", "dae", "fbx", "ply", "wrl", "vrml",
}
MAX_FILE_MB = 5
MAX_DIR_MB = 20
VENDOR_ONLY = "kernel/forge-machine/data/"
FIELDS = ("name", "maker", "url", "read", "terms", "redistribution")
ALLOWED = ("allowed", "allowed-in-software")


def files(root):
    out = subprocess.run(["git", "ls-files", "-z"], cwd=root, check=True, stdout=subprocess.PIPE).stdout
    return [f for f in out.decode().split("\0") if f]


def own_globs(root):
    """(glob, unverified) per line of tools/ci/own-models.txt."""
    p = os.path.join(root, "tools/ci/own-models.txt")
    if not os.path.exists(p):
        return []
    globs = []
    for line in open(p, encoding="utf-8"):
        g, _, why = line.partition("#")
        if g.strip():
            globs.append((g.strip(), "UNVERIFIED" in why))
    return globs


def main(root):
    bad, warn = [], []
    globs = own_globs(root)
    records = {}
    sizes = {}
    for f in files(root):
        ext = f.rsplit(".", 1)[-1].lower() if "." in os.path.basename(f) else ""
        if ext not in EXTS:
            continue
        d = os.path.dirname(f)
        own = [u for g, u in globs if fnmatch.fnmatchcase(f, g)]
        if own and not f.startswith(VENDOR_ONLY):
            if own[0]:
                warn.append(f"{f}: listed as ours but UNVERIFIED (tools/ci/own-models.txt): show where it came from")
            continue
        if d not in records:
            rp = os.path.join(root, d, "MODEL-LICENCE.toml")
            try:
                with open(rp, "rb") as h:
                    records[d] = {e.get("name"): e for e in tomllib.load(h).get("file", [])}
            except FileNotFoundError:
                records[d] = {}
            except tomllib.TOMLDecodeError as e:
                bad.append(f"{d}/MODEL-LICENCE.toml: does not read ({e})")
                records[d] = {}
        e = records[d].get(os.path.basename(f))
        if e is None:
            bad.append(
                f"{f}: a model with no licence record. If forgecad or this repository made it, list it in "
                f"tools/ci/own-models.txt with why; if it is a maker's, add a [[file]] to {d or '.'}/MODEL-LICENCE.toml "
                f"quoting terms that permit redistribution, or do not commit it (the user attaches it in the "
                f"Machine Builder, kept in their library)"
            )
            continue
        missing = [k for k in FIELDS if not str(e.get(k, "")).strip()]
        if missing:
            bad.append(f"{f}: its licence record lacks {', '.join(missing)}")
            continue
        if e["redistribution"] not in ALLOWED:
            bad.append(
                f"{f}: redistribution = {e['redistribution']!r}: only {' or '.join(ALLOWED)} may be committed; "
                f"terms that are silent, unclear or personal-use-only keep the model out of the repository"
            )
        if not re.fullmatch(r"\d{4}-\d{2}-\d{2}", str(e["read"])):
            bad.append(f"{f}: read = {e['read']!r}: the date the terms were read, YYYY-MM-DD")
        if not str(e["url"]).startswith(("https://", "http://")):
            bad.append(f"{f}: url = {e['url']!r}: where the terms were read")
        mb = os.path.getsize(os.path.join(root, f)) / 1e6
        if mb > MAX_FILE_MB:
            bad.append(f"{f}: {mb:.1f} MB, over {MAX_FILE_MB} MB: decimate it (or Git LFS, decided with the coordinator)")
        sizes[d] = sizes.get(d, 0.0) + mb
    for d, mb in sorted(sizes.items()):
        if mb > MAX_DIR_MB:
            bad.append(f"{d}: licensed models {mb:.1f} MB together, over {MAX_DIR_MB} MB")
    for w in warn:
        print(f"warning: {w}")
    for b in bad:
        print(b)
    print(f"model-licences: {len(bad)} refusal(s), {len(warn)} warning(s)")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else "."))
