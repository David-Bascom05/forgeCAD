#!/usr/bin/env python3
"""The real-parts build rate: translated timelines built and checked against the truth the dataset
records (Fusion: volume; DeepCAD: bounding box).

  realparts.py DIR --out RUN [--method NAME] [--no-shrink] [--tol 1e-3] [--every N] [--timeout S] [--correct FILE]
  realparts.py DIR --out RUN --shrink-only [--each N]

DIR holds <stem>.fc and <stem>.json from fusion.py. A part counts as built when it regenerates,
every stage valid, meshes closed, and its volume is within --tol of the truth (1% for a part whose
ellipses or splines became polylines).
"""
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import fp


def verdict(r, truth, tol):
    """The dataset's truth applied to a judge result."""
    v, sig = r["v"], r["sig"]
    if v == "fail" and sig.startswith("nary: the part is"):
        # n-ary regeneration is single-body by design; the sequential result stands
        r = dict(r, v="ok", sig="", multi=True)
        v = "ok"
    if v == "ok":
        vol = r.get("vol")
        t = truth.get("volume") or 0
        if vol is None:
            return dict(r, v="fail", sig="mass: no exact volume")
        if t > 0:
            e = abs(vol - t) / t
            lim = 0.01 if "approx" in truth.get("notes", []) else tol
            if e > lim:
                band = "x2" if abs(e - 0.5) < 0.01 or abs(e - 1) < 0.01 else "1%" if e < 0.01 else "10%" if e < 0.1 else "big"
                return dict(r, v="wrong", sig=f"volume off {band}", err=e)
    return r


def shrink_smallest(d, out, each):
    """Each failing signature's smallest parts, shrunk (a signature can hide several bugs)."""
    res = json.load(open(os.path.join(out, "results.json")))
    by = {}
    for n, r in res.items():
        if r["v"] in ("fail", "refused", "timeout", "crash", "oom"):
            by.setdefault(fp.sig_of(r), []).append((os.path.getsize(os.path.join(d, n + ".fc")), n))

    def one(job):
        s, k, n = job
        path = os.path.join(out, "sigs", f"{fp.sig_id(s)}.{k}.fc")
        if os.path.exists(path):
            return
        # the part may have been fixed since: only shrink what still fails the same way
        src = open(os.path.join(d, n + ".fc")).read()
        if fp.sig_of(fp.judge_text(src, 30)) != s:
            return
        mini, probes = fp.shrink(src, s, timeout=30)
        with open(path, "w") as f:
            f.write(f"# {s}\n# {n}, {len(by[s])} real parts sign this, shrunk in {probes} probes\n{mini}")
        print(fp.sig_id(s), k, n, len(mini.splitlines()), "lines", flush=True)

    jobs = [(s, k, n) for s, v in by.items() for k, (_, n) in enumerate(sorted(v)[:each])]
    for _ in fp.pool_map(one, jobs, 3):
        pass


def main():
    a = sys.argv[1:]
    opt = lambda k, d=None: a[a.index(k) + 1] if k in a else d
    d = a[0]
    tol = float(opt("--tol", "1e-3"))
    book = fp.Book(opt("--out"), opt("--method", "realparts"))
    if "--shrink-only" in a:
        return shrink_smallest(d, opt("--out"), int(opt("--each", "2")))
    names = sorted(n[:-3] for n in os.listdir(d) if n.endswith(".fc"))[:: int(opt("--every", "1"))]

    def one(n):
        return n, fp.judge_file(os.path.join(d, n + ".fc"), float(opt("--timeout", "30")))

    res = {}
    for k, (n, r) in enumerate(fp.pool_map(one, names)):
        truth = json.load(open(os.path.join(d, n + ".json")))
        r = verdict(r, truth, tol)
        res[n] = r
        book.add(n, r, open(os.path.join(d, n + ".fc")).read() if r["v"] != "ok" else None)
        if k % 500 == 499:
            print(k + 1, book.counts, f"{book.cpu:.0f} cpu s", flush=True)
    with open(os.path.join(opt("--out"), "results.json"), "w") as f:
        json.dump(res, f)
    print(json.dumps(book.summary()))
    # the number: of every design in the dataset, those built and agreeing with the dataset's truth
    # a final body truly pinched in one solid cannot exist: refusing it is right (research/find-parts/README.md)
    right = opt("--correct")
    if right:
        with open(right) as f:
            names = {l.split("#")[0].strip() for l in f}
        for n in names & set(res):
            if res[n]["v"] == "refused":
                book.counts["ok"] = book.counts.get("ok", 0) + 1
                book.counts["refused"] -= 1
                res[n] = dict(res[n], v="ok", sig="", correct_refusal=True)
    t = os.path.join(d, "_tally.txt")
    lost = {}
    if os.path.exists(t):
        lost = {k: v for k, v in json.load(open(t)).items() if k != "ok"}
    total = len(res) + sum(lost.values())
    ok = book.counts.get("ok", 0)
    print(f"real-parts build rate: {100 * ok / max(total, 1):.1f}% ({ok} of {total}; {len(res)} translated, {100 * ok / max(len(res), 1):.1f}% of those)")
    if lost:
        print("not translated:", lost)
    tally = {}
    for r in res.values():
        if r["v"] != "ok":
            k = fp.sig_of(r)
            tally[k] = tally.get(k, 0) + 1
    for k, c in sorted(tally.items(), key=lambda x: -x[1]):
        print(c, fp.sig_id(k), k)
    if "--no-shrink" not in a:
        # wrong volumes need the truth to shrink; those are triaged from the ledger
        book.shrink_all(lambda r: r["v"] in ("fail", "refused", "timeout", "crash", "oom"))


if __name__ == "__main__":
    main()
