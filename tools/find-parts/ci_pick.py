#!/usr/bin/env python3
"""Picks the CI tier: greedy set cover of kernel branches per CPU-second over a sample of the
generators' parts (no downloaded data), and writes tools/find-parts/ci-corpus.txt.

  fbuild python3 tools/find-parts/ci_pick.py --exe COV_JUDGE [--seconds 6] [--sample 1500]
"""
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cover
import fp
import gens


def main():
    args = sys.argv[1:]
    opt = lambda k, d=None: args[args.index(k) + 1] if k in args else d
    exe = opt("--exe")
    seconds = float(opt("--seconds", "6"))
    n = int(opt("--sample", "1500"))
    tmp = opt("--tmp", os.path.join(fp.ROOT, ".scratch"))
    r = random.Random(7)

    def pick(g, k):
        g = list(g)
        return r.sample(g, min(k, len(g)))

    cands = list(gens.standard()) + list(gens.bench())
    cands += pick(gens.enumerate_pairs(), n)
    cands += pick(gens.matrix(), n)
    cands += pick(gens.sweep(literals=False), n // 2)

    def one(x):
        return x[0], x[1], cover.run(exe, x[1], tmp)

    runs = []
    for name, src, (res, cov) in fp.pool_map(one, cands):
        if res["v"] in ("ok", "refused", "fail") and cov:
            runs.append((name, src, res, cov))
    # plain judge times: the instrumented binary's overhead is uneven
    print(len(runs), "judged", flush=True)
    covered, chosen, spent = 0, [], 0.0
    left = runs
    while left and spent < seconds:
        best, score = None, 0.0
        for x in left:
            gain = bin(x[3] & ~covered).count("1")
            s = gain / (x[2].get("cpu", 0) + 0.01)
            if gain and s > score:
                best, score = x, s
        if best is None:
            break
        chosen.append(best)
        covered |= best[3]
        spent += best[2].get("cpu", 0)
        left = [x for x in left if x is not best]
    every = 0
    for x in runs:
        every |= x[3]
    print(len(chosen), "chosen", f"{spent:.1f} instrumented cpu s, covers", bin(covered).count("1"), "of", bin(every).count("1"))
    with open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "ci-corpus.txt"), "w") as f:
        for name, src, res, _ in sorted(chosen, key=lambda x: x[0]):
            f.write(f"### {name} | {res['v']}\n{src if src.endswith(chr(10)) else src + chr(10)}")


if __name__ == "__main__":
    main()
