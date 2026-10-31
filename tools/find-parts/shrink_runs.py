#!/usr/bin/env python3
"""Each new failure signature of the given runs, still failing on this judge, shrunk once and written to OUT.

  fbuild python3 tools/find-parts/shrink_runs.py --out OUT [--known SIGS.txt] RUN..
"""
import glob
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import fp


def main():
    a = sys.argv[1:]
    out = a[a.index("--out") + 1]
    del a[a.index("--out") : a.index("--out") + 2]
    known = set()
    if "--known" in a:
        i = a.index("--known")
        with open(a[i + 1]) as f:
            known = {l.rstrip("\n") for l in f}
        del a[i : i + 2]
    os.makedirs(out, exist_ok=True)
    jobs = {}
    for run in a:
        for f in sorted(glob.glob(os.path.join(run, "sigs", "*.src.fc"))):
            with open(f) as h:
                head = h.readline()[2:].rstrip("\n")
            if head.split(":")[0] in ("fail", "timeout", "crash", "oom") and "nary: the part is" not in head and head not in known:
                jobs.setdefault(head, (run, f))

    def one(job):
        sig, (run, f) = job
        with open(f) as h:
            src = h.read()
        r = fp.judge_text(src, 30)
        if fp.sig_of(r) != sig:
            return sig, run, None
        mini, probes = fp.shrink(src, sig, timeout=30)
        with open(os.path.join(out, fp.sig_id(sig) + ".fc"), "w") as h:
            h.write(f"# {sig}\n# found by {os.path.basename(run)}, shrunk in {probes} probes\n{mini}")
        return sig, run, len(mini.splitlines())

    for sig, run, n in fp.pool_map(one, list(jobs.items()), 3):
        print(fp.sig_id(sig), os.path.basename(run), n if n else "gone", sig[:90], flush=True)


if __name__ == "__main__":
    main()
