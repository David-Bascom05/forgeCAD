#!/usr/bin/env python3
"""Discovery runs: each METHOD generates parts, judges them, dedupes failures by signature and
shrinks each to a minimal .fc, keeping a ledger so methods compare by distinct failures per CPU-hour.

  fbuild python3 tools/find-parts/find.py METHOD --out RUN [--cpu SECONDS] [--seed N] [--jobs 4]
                                          [--no-shrink] [--from DIR]
METHOD: fuzz swarm sweep enum matrix std mutate augment noise
--from DIR: extra real parts (.fc) for sweep and mutate. Runs stop at the CPU budget.
"""
import itertools
import json
import os
import random
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import fp
import gens


def read_dir(d):
    out = []
    for f in sorted(os.listdir(d)):
        if f.endswith(".fc"):
            with open(os.path.join(d, f)) as h:
                out.append((f[:-3], h.read()))
    return out


def cases(method, a):
    seed = a["seed"]
    extra = read_dir(a["from"]) if a["from"] else []
    if method == "sweep":
        # one edit of each part in turn: a slow part must not eat the budget
        by = {}
        for n, src in gens.sweep(list(gens.bench()) + extra):
            by.setdefault(n.split("/")[0], []).append((n, src))
        r = random.Random(seed)
        for v in by.values():
            r.shuffle(v)
        return iter([x for row in itertools.zip_longest(*by.values()) for x in row if x])
    if method == "enum":
        return list(gens.enumerate_pairs())
    if method == "matrix":
        return list(gens.matrix())
    if method == "std":
        return list(gens.standard())
    if method == "augment":
        return list(gens.augmented(list(gens.bench()) + list(gens.standard()) + extra, seed))
    if method == "mutate":
        return gens.mutated(list(gens.bench()) + extra, seed)
    raise SystemExit(f"unknown method {method}")


def jump(src, seed):
    """Does a hair of noise change the part's volume? (base result, noisy result, jumped?)"""
    a = fp.judge_text(src)
    if a["v"] != "ok" or not a.get("vol"):
        return a, None, False
    b = fp.judge_text(gens.perturb(src, seed))
    if b["v"] != "ok":
        return a, b, False
    return a, b, abs(b["vol"] - a["vol"]) > 1e-4 * abs(a["vol"])


def noise_run(book, parts, cpu, seed, jobs, out, shrink):
    """A built part, its coordinates moved 3e-7: it must still build, to about the same volume."""
    random.Random(seed).shuffle(parts)
    it = iter(parts)
    jumps = []

    def one(x):
        return (x, *jump(x[1], seed))

    while book.cpu < cpu:
        batch = list(itertools.islice(it, jobs * 8))
        if not batch:
            break
        for (n, src), a, b, j in fp.pool_map(one, batch, jobs):
            book.add(n, a, src if a["v"] != "ok" else None)
            if b is None:
                continue
            noisy = gens.perturb(src, seed)
            if j:
                book.add(n + "~", {"v": "wrong", "sig": "noise moved the volume", "cpu": b.get("cpu", 0)}, src)
                jumps.append((n, src))
            else:
                book.add(n + "~", b, noisy if b["v"] != "ok" else None)
        print(book.n, book.counts, f"{book.cpu:.0f} cpu s", len(book.sigs), "sigs", len(jumps), "jumps", flush=True)
    for i, (n, src) in enumerate(jumps[:40]):
        mini, _ = fp.shrink(src, "", keep=lambda t: jump(t, seed)[2])
        with open(os.path.join(out, "sigs", f"noise{i}.fc"), "w") as f:
            f.write(f"# a part whose volume noise moves ({n}); moved with gens.perturb seed {seed}\n{mini}# noisy:\n# " + gens.perturb(mini, seed).replace("\n", "\n# ") + "\n")
    print(json.dumps(book.summary()))
    with open(os.path.join(out, "summary-noise.json"), "w") as f:
        json.dump(book.summary(), f)
    if shrink:
        book.shrink_all()


def main():
    args = sys.argv[1:]
    method = args[0]
    opt = lambda k, d=None: args[args.index(k) + 1] if k in args else d
    a = {"seed": int(opt("--seed", "0")), "from": opt("--from")}
    out = opt("--out")
    cpu = float(opt("--cpu", "600"))
    jobs = int(opt("--jobs", "4"))
    book = fp.Book(out, method)
    os.environ.setdefault("FP_TMP", out)
    t0 = time.time()
    if method == "noise":
        return noise_run(book, read_dir(a["from"]) if a["from"] else list(gens.bench()) + list(gens.standard()), cpu, a["seed"], jobs, out, "--no-shrink" not in args)
    if method in ("fuzz", "swarm"):
        seeds = iter(range(a["seed"], 10**9))
        rng = random.Random(a["seed"])

        def one(s):
            # swarm: each history draws from a random third of the kinds
            kinds = sum(1 << i for i in range(64) if rng.random() < 0.33) if method == "swarm" else None
            return s, fp.judge_gen(s, kinds=kinds)

        while book.cpu < cpu:
            batch = [next(seeds) for _ in range(jobs * 4)]
            for s, (fails, c) in fp.pool_map(one, batch, jobs):
                for sig, src in fails:
                    v, tail = sig.split(": ", 1)
                    book.add(f"seed{s}", {"v": v, "sig": tail, "cpu": 0}, src)
                book.add(f"seed{s}", {"v": "ok", "sig": "", "cpu": c})
    else:
        gen = cases(method, a)
        if isinstance(gen, list):
            random.Random(a["seed"]).shuffle(gen)
        gen = iter(gen)
        def one(x):
            return x[0], x[1], fp.judge_text(x[1])
        done = False
        while not done and book.cpu < cpu:
            batch = []
            for _ in range(jobs * 8):
                x = next(gen, None)
                if x is None:
                    done = True
                    break
                batch.append(x)
            for n, s, r in fp.pool_map(one, batch, jobs):
                book.add(n, r, s if r["v"] != "ok" else None)
            print(book.n, book.counts, f"{book.cpu:.0f} cpu s", len(book.sigs), "sigs", flush=True)
    sm = book.summary()
    sm["wall_s"] = round(time.time() - t0, 1)
    print(json.dumps(sm))
    with open(os.path.join(out, f"summary-{method}.json"), "w") as f:
        json.dump(sm, f)
    for s, n in sorted(book.sigs.items()):
        print(fp.sig_id(s), s[:120], "|", n)
    if "--no-shrink" not in args:
        book.shrink_all()


if __name__ == "__main__":
    main()
