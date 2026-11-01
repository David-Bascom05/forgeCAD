#!/usr/bin/env python3
"""Whose fault is a built part that disagrees with the dataset? OpenCascade replays the same .fc.

  $VENV/bin/python -I triage_wrong.py WRONG.json --export PARTS.json --results RESULTS.jsonl --truth TRUTH.jsonl --out OUT.jsonl

ours-wrong   OCCT agrees with the dataset, we do not: a silent wrong answer, the most serious class.
importer     OCCT agrees with us: both kernels read the .fc alike, so the .fc is not the source's part.
three-way    nobody agrees: needs Monte Carlo (tools/occt/montecarlo.py) before blaming anyone.
no-opinion   OCCT could not build it, or the export cut the part short.
"""
import concurrent.futures as cf
import json
import os
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
TOL = 2e-3


def rel(a, b):
    return abs(a - b) / max(abs(b), 1e-12)


def main():
    a = sys.argv[1:]
    opt = lambda k: a[a.index(k) + 1]
    wrong = json.load(open(a[0]))
    parts = {p["name"]: p for p in json.load(open(opt("--export")))["parts"]}
    res = {json.loads(l)["id"]: json.loads(l) for l in open(opt("--results"))}
    truth = {json.loads(l)["id"]: json.loads(l) for l in open(opt("--truth"))}
    tmp = tempfile.mkdtemp(prefix="triage-", dir=os.path.dirname(os.path.abspath(opt("--out"))))

    def one(w):
        i = w["id"]
        r, t = res[i], truth[i]
        out = dict(w, ours=r.get("vol"), ours_area=r.get("area"), truth=t["vol"], truth_area=t["area"])
        p = parts.get(i)
        if p is None or p.get("cut"):
            return dict(out, cls="no-opinion", why="export cut or skipped: " + str(p and p.get("cut"))[:50])
        job = os.path.join(tmp, i + ".json")
        json.dump({"features": p["features"]}, open(job, "w"))
        try:
            c = subprocess.run([sys.executable, "-I", os.path.join(HERE, "occt_second.py"), "--child", job], capture_output=True, text=True, timeout=900)
        except subprocess.TimeoutExpired:
            return dict(out, cls="no-opinion", why="OCCT timed out")
        finally:
            os.remove(job)
        if c.returncode != 0 or not c.stdout.strip():
            return dict(out, cls="no-opinion", why=(c.stderr.strip().splitlines() or ["?"])[-1][:80])
        o = json.loads(c.stdout.strip().splitlines()[-1])
        out["occt"], out["occt_area"], out["occt_valid"] = o["vol"], o["area"], o["valid"]
        with_truth = rel(o["vol"], t["vol"]) <= TOL and rel(o["area"], t["area"]) <= TOL
        with_ours = rel(o["vol"], r["vol"]) <= TOL and rel(o["area"], r["area"]) <= TOL
        cls = "ours-wrong" if with_truth and not with_ours else "importer" if with_ours else "three-way"
        return dict(out, cls=cls, why="")

    with open(opt("--out"), "w") as f, cf.ThreadPoolExecutor(4) as ex:
        for x in ex.map(one, wrong):
            f.write(json.dumps(x) + "\n")
            f.flush()
    os.rmdir(tmp)


if __name__ == "__main__":
    main()
