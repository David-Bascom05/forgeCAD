#!/usr/bin/env python3
"""Coverage-guided search: mutate parts, keep any that reach a kernel branch no part has reached,
and mutate most those holding the rarest branches (FairFuzz's idea; Lemieux & Sen, ASE 2018).

The judge is built with -C instrument-coverage. Each run writes a raw profile; its counters are read
straight from the file, so no LLVM tools are needed. Build (a separate target dir, slower binary):
  fbuild env RUSTFLAGS="-C instrument-coverage -C target-cpu=x86-64-v3" CARGO_TARGET_DIR=$PWD/.scratch/target-cov \\
      cargo build --release -p forge-proof --example judge

  fbuild python3 tools/find-parts/cover.py --exe PATH --out RUN --cpu SECONDS [--seed N] [--from DIR]
"""
import json
import os
import random
import struct
import sys
import tempfile
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import fp
import gens
from find import read_dir


def counters(path):
    """Raw profile v11: a bitset of the counters that fired."""
    b = open(path, "rb").read()
    h = struct.unpack("<19Q", b[:152])
    assert h[1] == 11, f"raw profile version {h[1]}"
    off = 152 + h[2] + h[3] * 72 + h[4]
    n = h[5]
    # any nonzero byte in a counter: fold the eight byte planes together
    c = b[off : off + 8 * n]
    fold = 0
    for i in range(8):
        fold |= int.from_bytes(c[i :: 8], "little")
    # bytes to bits
    return int.from_bytes(fold.to_bytes(n, "little").translate(NONZERO), "little")


NONZERO = bytes([0] + [1] * 255)


def run(exe, src, tmp, seen_lock=None):
    with tempfile.NamedTemporaryFile("w", suffix=".fc", dir=tmp) as f:
        f.write(src)
        f.flush()
        prof = f.name + ".profraw"
        env = dict(os.environ, LLVM_PROFILE_FILE=prof)
        r = fp.judge_file(f.name, 30.0, exe=exe, env=env)
        cov = 0
        if os.path.exists(prof):
            try:
                cov = counters(prof)
            finally:
                os.unlink(prof)
    return r, cov


def main():
    args = sys.argv[1:]
    opt = lambda k, d=None: args[args.index(k) + 1] if k in args else d
    exe, out = opt("--exe"), opt("--out")
    budget = float(opt("--cpu", "600"))
    rng = random.Random(int(opt("--seed", "0")))
    book = fp.Book(out, "cover")
    tmp = out
    seeds = list(gens.bench()) + list(gens.standard())
    if opt("--from"):
        seeds += read_dir(opt("--from"))
    seen = 0
    corpus = []  # [source blocks, novel bytes (the rare branches it brought), times used]
    hits = {}  # byte index -> how many runs fired that byte

    def consider(name, src, r, cov):
        nonlocal seen
        book.add(name, r, src if r["v"] != "ok" else None)
        new = cov & ~seen
        if new and r["v"] in ("ok", "refused", "fail"):
            seen |= cov
            b = new.to_bytes((new.bit_length() + 7) // 8, "little")
            idx = [i for i, x in enumerate(b) if x][:64]
            corpus.append([gens.feature_lines(src), idx, 0])
            for i in idx:
                hits[i] = 0
        elif new:
            seen |= cov
        # cheap rarity counting: only the bytes some corpus part brought
        if hits:
            b = cov.to_bytes((cov.bit_length() + 7) // 8, "little")
            for i in hits:
                if i < len(b) and b[i]:
                    hits[i] += 1

    def one(x):
        return x[0], x[1], run(exe, x[1], tmp)

    for n, s, (r, c) in fp.pool_map(one, seeds):
        consider(n, s, r, c)
    pool = [c[0] for c in corpus]
    i = 0
    t0 = time.time()
    while book.cpu < budget:
        batch = []
        for _ in range(16):
            # tournament: of a few, the one holding the rarest branch, least mutated breaks ties
            pick = min(rng.sample(corpus, min(4, len(corpus))), key=lambda c: (min((hits.get(k, 0) for k in c[1]), default=0), c[2]))
            pick[2] += 1
            t, w = gens.mutate_once(pick[0], rng, pool)
            if rng.random() < 0.3:
                t, w2 = gens.mutate_once(t, rng, pool)
                w += "+" + w2
            batch.append((f"cov{i}/{w}", "\n".join(t) + "\n"))
            i += 1
        for n, s, (r, c) in fp.pool_map(one, batch):
            consider(n, s, r, c)
        if i % 320 == 0:
            print(i, book.counts, f"{book.cpu:.0f} cpu s", len(book.sigs), "sigs", len(corpus), "corpus", bin(seen).count("1"), "bytes covered", flush=True)
    sm = book.summary()
    sm.update(wall_s=round(time.time() - t0, 1), corpus=len(corpus), covered=bin(seen).count("1"))
    print(json.dumps(sm))
    with open(os.path.join(out, "summary-cover.json"), "w") as f:
        json.dump(sm, f)
    for k, v in enumerate(corpus[-200:]):
        with open(os.path.join(out, f"corpus{k}.fc"), "w") as f:
            f.write("\n".join(v[0]) + "\n")
    if "--no-shrink" not in args:
        book.shrink_all()


if __name__ == "__main__":
    main()
