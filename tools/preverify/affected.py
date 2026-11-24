#!/usr/bin/env python3
"""What a branch's diff can reach, for its owner's pre-verify (never for what lands: every train
runs every check on the merged tree, whatever this says).

    tools/preverify/affected.py [--base REF] [--reads MAP.json] [--json]

The diff is BASE...HEAD (from their merge base; BASE defaults to origin/main). A package's tests
are selected when:
  - a file changed under its own directory, or a path dependency's (then every package depending
    on it, by the Cargo graph, every dependency kind: a dependent's tests run the changed code);
  - a test of it read the changed file at run time or its build read it (MAP, from readsmap.py on
    main), or listed a directory in which a file was added, deleted or renamed;
  - its tests in MAP were cut short (`partial`): any change selects it;
  - the change is global for its workspace: the workspace's Cargo.toml, rust-toolchain.toml,
    .cargo/, clippy.toml, rustfmt.toml, or a Cargo.lock entry in the package's dependency closure.
Without MAP every file outside a package that is not plain text documentation (*.md, docs/)
selects the whole workspace: no map, no narrowing.

Also says whether the speed gate's binaries can have changed (a package in forge-model's or
forge-lang's normal-dependency closure, a Cargo.lock entry in it, the workspace's global files, or
tools/speed/), and whether the forge-render / forge-speech workspaces are reached.
"""
import argparse, json, os, subprocess, sys

ap = argparse.ArgumentParser()
ap.add_argument("--base", default="origin/main")
ap.add_argument("--head", default="HEAD")
ap.add_argument("--reads")
ap.add_argument("--json", action="store_true")
a = ap.parse_args()
root = subprocess.run(["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True,
                      check=True).stdout.strip()


def git(*args):
    return subprocess.run(["git", *args], cwd=root, capture_output=True, text=True,
                          check=True).stdout


changes = []  # (status, path)
for line in git("diff", "--name-status", "-M", f"{a.base}...{a.head}").splitlines():
    f = line.split("\t")
    if f[0].startswith("R"):
        changes += [("D", f[1]), ("A", f[2])]
    else:
        changes.append((f[0][0], f[1]))
paths = [p for _, p in changes]
WORKSPACES = [".", "interface/forge-render", "interface/forge-speech"]
GLOBAL = ("rust-toolchain.toml", "clippy.toml", "rustfmt.toml")
reads = json.load(open(a.reads))["workspaces"] if a.reads else None


def meta(ws):
    r = subprocess.run(["cargo", "metadata", "--format-version", "1", "--locked"],
                       cwd=os.path.join(root, ws), capture_output=True, text=True)
    if r.returncode:
        sys.exit(f"affected: cargo metadata failed in {ws}:\n{r.stderr}")
    return json.loads(r.stdout)


def lock_changed(ws):
    """Names of Cargo.lock entries the diff adds, removes or changes in this workspace."""
    lock = os.path.normpath(os.path.join(ws, "Cargo.lock"))
    if lock not in paths:
        return set()
    names = set()
    for line in git("diff", "-U200", f"{a.base}...{a.head}", "--", lock).split("\n[[package]]"):
        if any(l[:1] in "+-" and not l.startswith(("+++", "---")) for l in line.splitlines()):
            for l in line.splitlines():
                l = l.lstrip("+- ")
                if l.startswith("name = "):
                    names.add(l.split('"')[1])
                    break
    return names


def inert(p):
    # plain text documentation, and CI's own configuration (no cargo test reads it; CI runs it on
    # the train's pull request)
    return p.endswith(".md") or p.startswith(("docs/", ".github/"))


metas = {ws: meta(ws) for ws in WORKSPACES}
# every file under some workspace's path package, whichever workspace
anyowned = set()
for m in metas.values():
    ds = [os.path.relpath(os.path.dirname(p["manifest_path"]), root) for p in m["packages"]
          if p["source"] is None]
    anyowned |= {p for p in paths if any(p.startswith(d + "/") for d in ds)}


out = {"base": git("rev-parse", a.base).strip(), "head": git("rev-parse", a.head).strip(),
       "changed": len(paths), "workspaces": {}, "gate": False, "why_gate": []}
for ws in WORKSPACES:
    m = metas[ws]
    pk = {p["id"]: p for p in m["packages"]}
    members = set(m["workspace_members"])
    # every path package (members and path dependencies), by directory, longest first
    pdirs = sorted(((os.path.relpath(os.path.dirname(p["manifest_path"]), root), i)
                    for i, p in pk.items() if p["source"] is None), key=lambda x: -len(x[0]))
    nodes = {n["id"]: n for n in m["resolve"]["nodes"]}

    def closure(i, normal_only=False):
        s, st = set(), [i]
        while st:
            x = st.pop()
            if x in s:
                continue
            s.add(x)
            for d in nodes.get(x, {}).get("deps", []):
                if not normal_only or any(k["kind"] is None for k in d["dep_kinds"]):
                    st.append(d["pkg"])
        return s

    full = {i: closure(i) for i in members}
    why = {}

    def pick(i, reason):
        why.setdefault(pk[i]["name"], reason)

    wsdir = os.path.normpath(ws)
    pre = "" if wsdir == "." else wsdir + "/"
    glob = [p for p in paths if p == pre + "Cargo.toml" or p.startswith(pre + ".cargo/")
            or p in GLOBAL]
    if glob:
        for i in members:
            pick(i, f"global: {glob[0]}")
    locked = lock_changed(ws)
    owned = {}
    for p in paths:
        for d, i in pdirs:
            if p.startswith(d + "/"):
                owned[p] = i
                break
    for p, i in owned.items():
        for j in members:
            if i in full[j]:
                pick(j, f"{p}" if i == j else f"{p} (depends on {pk[i]['name']})")
    if locked:
        for j in members:
            hit = {pk[x]["name"] for x in full[j]} & locked
            if hit:
                pick(j, f"Cargo.lock: {sorted(hit)[0]}")
    loose = [p for p in paths if p not in anyowned and not inert(p)]
    if reads is None:
        if loose:
            for i in members:
                pick(i, f"no reads map; {loose[0]} is outside every package")
    else:
        rmap = reads.get(ws, {})
        byname = {pk[i]["name"]: i for i in members}
        for st, p in changes:
            for name, e in rmap.items():
                if name not in byname:
                    continue
                if p in e["files"] or (st in "AD" and os.path.dirname(p) in e["dirs"]):
                    pick(byname[name], f"its tests read {p}")
        for name, e in rmap.items():
            if e.get("partial") and name in byname and paths:
                pick(byname[name], "its reads map is partial")
        # a member the map does not know (new on this branch) is selected by its own files above
    sel = sorted(why)
    out["workspaces"][ws] = {"packages": sel, "why": why, "members": len(members)}
    if wsdir == ".":
        gate = set()
        for i in members:
            if pk[i]["name"] in ("forge-model", "forge-lang"):
                gate |= closure(i, normal_only=True)
        gdirs = [d for d, i in pdirs if i in gate]
        for p in paths:
            if any(p.startswith(d + "/") for d in gdirs):
                out["why_gate"].append(p)
        out["why_gate"] += [p for p in glob if p]
        out["why_gate"] += [f"Cargo.lock: {n}" for n in sorted(locked & {pk[i]["name"] for i in gate})]
out["why_gate"] += [p for p in paths if p.startswith("tools/speed/")]
out["gate"] = bool(out["why_gate"])
if a.json:
    json.dump(out, sys.stdout, indent=1)
    print()
else:
    print(f"{out['changed']} files changed, {a.base}...{a.head}")
    for ws, w in out["workspaces"].items():
        print(f"{ws}: {len(w['packages'])} of {w['members']} packages")
        for n in w["packages"]:
            print(f"  {n:28s} {w['why'][n]}")
    print("speed gate:", ("yes, " + out["why_gate"][0]) if out["gate"] else "no (its binaries cannot have changed)")
