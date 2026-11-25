"""The Haas corpus: the smallest cases of each failure the sweep found, and a runner that replays
them from the zips, for the kernel's rewrite to pass.

    python3 -I tools/kernel-stress/corpus.py make OUTDIR > research/kernel-stress/corpus.tsv
    python3 -I tools/kernel-stress/corpus.py run [corpus.tsv] [-z ZIPDIR] [--only ID,..]

`make` reads a sweep's JSON lines (run.sh's OUTDIR) and keeps, for each failure class, the three
cases on the fewest faces. A row names the zip, the STEP file in it, the solid's entity (two for
a pair), the op and what it gave then. research/kernel-stress/pinned.tsv adds cases named by hand
(asked for by another session, with an owner), whatever their size. No geometry is in the corpus: `run` takes each solid out
of its file on the branch haas-3d-models (extract.py), runs the op (verify/forge-stress) and
says whether it now passes. Passing means: the solid reads valid (closed, checks, mass, a
watertight mesh), and the op answers Ok and right. A row that was wrong (or hung, or crashed)
and now refuses is `better`: no longer a wrong answer, not yet a pass. `harness`: the op's moved
copy itself fails check, so the row says nothing of the boolean.
"""

import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from collections import defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, HERE)
from report import family, load, variant  # noqa: E402

COLS = ["id", "class", "zip", "file", "ents", "op", "then", "detail"]


def make(out):
    zips = load(out)
    by = defaultdict(list)  # class -> [(faces, row)]
    for z in zips.values():
        for f in z["files"]:
            if f["bytes"] < 1000:
                continue
            for s, sol in f["solids"].items():
                bad = None
                if sol["problems"] != 0:
                    bad = "import: check " + sol["why"].split(":")[0]
                elif sol["merr"]:
                    bad = "import: mass " + variant(sol["merr"])
                elif not sol["watertight"]:
                    bad = "import: mesh " + (variant(sol["mesh_err"]) if sol["mesh_err"] else "open")
                if bad:
                    by[bad].append((sol["faces"], [z["name"] + ".zip", f["file"], str(sol["ent"]), "solid", "fail",
                                                   sol["why"][:120] or sol["merr"][:120] or "mesh open"]))
            imp = f["import"] or {}
            for r in imp.get("refusals", []):
                by["import: refused " + norm_why(r["why"])].append(
                    (0, [z["name"] + ".zip", f["file"], str(r["ent"]), "import", "refused", r["why"][:120]]))
            for (s, op), e in f["ops"].items():
                if e["st"] in ("ok", "empty", "harness"):
                    continue
                fam = family(op)
                ents = str(e["ent"])
                m = re.match(r"(?:union|int)_pair(\d+)|pair(\d+)_sum", op)
                if m:
                    j = int(m.group(1) or m.group(2))
                    other = f["solids"].get(j)
                    if not other:
                        continue
                    ents += f",{other['ent']}"
                g = "section" if fam == "section" else ("oracle" if fam.endswith("_sum") else "boolean")
                cls = f"{g} {e['st']}: {variant(e.get('d', ''))}"
                faces = f["solids"].get(s, {}).get("faces", 10**9)
                by[cls].append((faces, [z["name"] + ".zip", f["file"], ents, fam, e["st"], e.get("d", "")[:120]]))
            for h in f["hangs"] + [c for c in f["crashes"] if c.get("key") != "import"]:
                key = h.get("key") or ""
                if ":" not in key:
                    continue
                s, op = key.split(":", 1)
                sol = f["solids"].get(int(s[1:]), {})
                st = h.get("what", "hang") if h in f["hangs"] else f"crash rc {h['rc']}"
                by[f"{family(op)} {st}"].append((sol.get("faces", 10**9), [
                    z["name"] + ".zip", f["file"], str(sol.get("ent", "?")), family(op), st, h.get("d", "")[:120]]))
    print("\t".join(COLS))
    n = 0
    for cls in sorted(by):
        seen = set()
        for faces, row in sorted(by[cls], key=lambda x: x[0]):
            # one part in several machines' zips: the same entity and faces is the same case
            k = (row[2], faces, row[3])
            if k in seen:
                continue
            seen.add(k)
            n += 1
            print("\t".join([f"K{n:03d}", cls] + [c.replace("\t", " ") for c in row]))
            if len(seen) == 3:
                break
    # the cases named by hand (pinned.tsv: who asked, who owns it), whatever the sweep's smallest
    pinned = os.path.join(ROOT, "research", "kernel-stress", "pinned.tsv")
    for line in open(pinned):
        c = line.rstrip("\n").split("\t")
        if line.startswith("class\t") or len(c) < 7:
            continue
        n += 1
        print("\t".join([f"K{n:03d}"] + c[:7]))


def norm_why(w):
    return re.sub(r"\d+", "N", w)[:60]


def solid_ok(sol):
    return sol["problems"] == 0 and not sol["merr"] and sol["watertight"]


def run(path, zipdir, only):
    rows = [dict(zip(COLS, line.rstrip("\n").split("\t"))) for line in open(path) if not line.startswith("id\t")]
    if only:
        rows = [r for r in rows if r["id"] in only]
    binp = os.path.join(ROOT, "target", "release", "forge-stress")
    subprocess.run(["cargo", "build", "--release", "-q", "-p", "forge-stress"], cwd=ROOT, check=True)
    tmp = tempfile.mkdtemp(prefix="ks-corpus.")
    results = []
    try:
        byzip = defaultdict(list)
        for r in rows:
            # a row not out of a zip (an example part, pinned) is replayed by its own test
            if r["zip"].startswith("("):
                results.append((r, ("elsewhere", r["detail"][-60:])))
                continue
            byzip[r["zip"]].append(r)
        for zname, rs in byzip.items():
            zd = os.path.join(tmp, "z")
            os.makedirs(zd, exist_ok=True)
            zp = os.path.join(zd, "z.zip")
            if zipdir:
                shutil.copy(os.path.join(zipdir, zname), zp)
            else:
                with open(zp, "wb") as fh:
                    subprocess.run(["git", "-C", ROOT, "show", f"origin/haas-3d-models:research/haas-models/{zname}"],
                                   stdout=fh, check=True)
            for r in rs:
                subprocess.run(["unzip", "-qq", "-o", zp, r["file"], "-d", zd], check=True)
                src = os.path.join(zd, r["file"])
                one = os.path.join(tmp, r["id"] + ".step")
                subprocess.run([sys.executable, "-I", os.path.join(HERE, "extract.py"), src, r["ents"], one],
                               check=True, stdout=subprocess.DEVNULL)
                p = subprocess.run([binp, one, "--max-solids", "2", "--timeout", "120"], capture_output=True, text=True)
                evs = [json.loads(x) for x in p.stdout.splitlines() if x.startswith("{")]
                results.append((r, judge(r, evs, p.returncode)))
                os.remove(one)
            shutil.rmtree(zd)
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    width = max(len(r["class"]) for r, _ in results) if results else 10
    passed = 0
    tally = {}
    for r, (verdict, now) in results:
        passed += verdict == "pass"
        tally[verdict] = tally.get(verdict, 0) + 1
        print(f"{r['id']}  {verdict:7}  {r['class'][:width]:{width}}  then {r['then']:8} now {now}")
    print(f"{passed}/{len(results)} pass; " + ", ".join(f"{k} {v}" for k, v in sorted(tally.items())))
    return 0 if passed == len(results) else 1


def judge(r, evs, rc):
    """('pass'|'fail', what happened now)."""
    if rc == 3:
        return "fail", "hang"
    if rc not in (0,):
        return "fail", f"crash rc {rc}"
    imp = next((e for e in evs if e.get("ev") == "import"), None)
    if not imp or imp["st"] != "ok":
        return "fail", "import " + (imp or {}).get("st", "none")
    sols = [e for e in evs if e.get("ev") == "solid"]
    if r["op"] == "import":
        return ("pass", "read") if sols and not imp["refusals"] else ("fail", "refused")
    bad = [s for s in sols if not solid_ok(s)]
    if r["op"] == "solid" or bad:
        if bad:
            return "fail", "solid " + (bad[0]["why"][:60] or bad[0]["merr"][:60] or "mesh open")
        return "pass", "valid"
    ops = [e for e in evs if e.get("ev") == "op" and family(e["op"]) == r["op"]]
    if not ops:
        return "fail", "op not run"
    worst = max(ops, key=lambda e: {"ok": 0, "empty": 0, "refused": 1, "wrong": 2, "panic": 3}.get(e["st"], 1))
    if worst["st"] in ("ok", "empty"):
        return "pass", "ok"
    # a wrong answer, a hang or a crash that now refuses is better, not a pass: told apart
    if worst["st"] == "refused" and r["then"] != "refused":
        return "better", f"refused {variant(worst.get('d', ''))} (was {r['then']})"
    if worst["st"] == "harness":
        return "harness", variant(worst.get("d", ""))
    return "fail", f"{worst['st']} {variant(worst.get('d', ''))}"


def main():
    a = sys.argv[1:]
    if not a or a[0] not in ("make", "run"):
        print(__doc__)
        return 2
    if a[0] == "make":
        make(a[1])
        return 0
    zipdir = a[a.index("-z") + 1] if "-z" in a else None
    only = set(a[a.index("--only") + 1].split(",")) if "--only" in a else None
    rest = [x for i, x in enumerate(a[1:], 1) if not x.startswith("-") and a[i - 1] not in ("-z", "--only")]
    path = rest[0] if rest else os.path.join(ROOT, "research", "kernel-stress", "corpus.tsv")
    return run(path, zipdir, only)


if __name__ == "__main__":
    sys.exit(main())
