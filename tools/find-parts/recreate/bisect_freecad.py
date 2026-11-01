#!/usr/bin/env python3
"""A wrong FreeCAD body: the first feature whose result leaves that feature's stored shape, since every object keeps its
own. Says which step to look at, and whether the part was already off before a blend.

  bisect_freecad.py RESULTS.jsonl --files DIR --judge EXE --topo EXE --venv PY --tmp DIR --out OUT.jsonl
"""
import collections
import json
import os
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import freecad as F
import run_cadfs as R


def main():
    a = sys.argv[1:]
    opt = lambda k: a[a.index(k) + 1]
    import run_freecad as RF

    rows = [json.loads(l) for l in open(a[0])]
    bad = [r for r in rows if RF.klass(r) == "wrong"]
    tally = collections.Counter()
    with open(opt("--out"), "w") as out:
        for r in bad:
            path = os.path.join(opt("--files"), r["file"] + ".FCStd")
            doc = F.Doc(open(path, "rb").read())
            chain = F.chain_of(doc, r["body"])
            want = {"shapes": {c: doc.shape_file(c) for c in chain}, "subs": {}}
            for c in chain:
                if doc.types[c] in ("PartDesign::Fillet", "PartDesign::Chamfer"):
                    base = doc.link(c, "Base")
                    subs = ["ALL"] if doc.boolean(c, "UseAllEdges") else (doc.subs(c, "Base") or ["ALL"])
                    want["subs"].setdefault(base, []).extend(subs)
            with tempfile.NamedTemporaryFile("w", suffix=".json", dir=opt("--tmp")) as f:
                json.dump(want, f)
                f.flush()
                p = subprocess.run([opt("--venv"), "-I", os.path.join(HERE, "fcstd_truth.py"), path, f.name], capture_output=True, text=True, timeout=600)
            truth = json.loads(p.stdout)
            t = F.Tr(doc, r["body"], opt("--topo"), truth)
            first = None
            try:
                t.run(chain)
            except (F.Unsupported, F.Unresolved, F.KernelStop) as x:
                first = {"step": "translate", "why": str(x)[:80]}
            if t.topo:
                t.topo.kill()
                t.topo.wait()
            for name, n in t.steps if first is None else []:
                fc = os.path.join(opt("--tmp"), r["file"] + ".prefix.fc")
                open(fc, "w").write("\n".join(t.lines[:n]) + "\n")
                j = R.judge(opt("--judge"), fc)
                os.remove(fc)
                tr = truth.get(name, {})
                if j["v"] != "ok" and not j["sig"].startswith("nary: the part is"):
                    first = {"step": name, "type": doc.types[name], "why": j["sig"][:80]}
                    break
                if tr.get("solids") and not R.compare(j, tr, r.get("notes", []))["match"]:
                    first = {"step": name, "type": doc.types[name], "ours": j.get("vol"), "theirs": tr["vol"], "ours_area": j.get("area"), "theirs_area": tr["area"], "index": [c for c in chain].index(name), "of": len(chain)}
                    break
            rec = dict(file=r["file"], body=r["body"], first=first or {"step": "none found"})
            out.write(json.dumps(rec) + "\n")
            out.flush()
            tally[(first or {}).get("type", (first or {}).get("step", "none found"))] += 1
    print(tally.most_common())


if __name__ == "__main__":
    main()
