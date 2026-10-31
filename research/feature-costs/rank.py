#!/usr/bin/env python3
"""Rank feature kinds from measure.py's tables: rank.py seq.tsv post.tsv > table.txt"""
import collections, statistics as st, sys

rows = [l.rstrip("\n").split("\t") for l in open(sys.argv[1])]
real = lambda s: s.startswith("bench/")
kinds = collections.defaultdict(list)
total_real = 0
for src, i, kind, size, status, cost, cum in rows:
    if status.startswith("err") or status == "crash":
        continue
    kinds[kind.removeprefix("in.")].append((int(cost), src, int(i), int(size)))
    total_real += int(cost) if real(src) else 0


def m(n):
    return f"{n / 1e6:10.2f}"


print(f"one seq regen of the 15 bench parts: {total_real / 1e6:.0f}M instructions:u\n")
print(f"{'kind':34}{'n':>5}{'real':>5}{'median M':>11}{'worst M':>11}{'per size M':>11}{'real sum M':>11}{'real %':>7}  worst at")
for k, v in sorted(kinds.items(), key=lambda kv: -sum(c[0] for c in kv[1] if real(c[1]))):
    costs = [c[0] for c in v]
    r = sum(c[0] for c in v if real(c[1]))
    w = max(v)
    size = sum(max(c[3], 1) for c in v)
    print(f"{k:34}{len(v):5}{sum(real(c[1]) for c in v):5}{m(st.median(costs))} {m(w[0])} {m(sum(costs) / size)} {m(r)}{100 * r / total_real:7.1f}  {w[1]}#{w[2]}")

print("\nby family (real parts)")
fam = collections.Counter()
for k, v in kinds.items():
    fam[k.split(".")[0]] += sum(c[0] for c in v if real(c[1]))
for k, c in fam.most_common():
    print(f"{k:14}{m(c)}{100 * c / total_real:7.1f}%")

print("\nslowest single features")
every = [(c[0], c[1], c[2], k, c[3]) for k, v in kinds.items() for c in v]
for c, s, i, k, n in sorted(every, reverse=True)[:40]:
    print(f"{m(c)}  {s}#{i} {k} size {n}")

if len(sys.argv) > 2:
    print("\nafter the regen, per part: regen, mesh, mass, check (M)")
    post = [l.rstrip("\n").split("\t") for l in open(sys.argv[2])]
    post = [p for p in post if p[0].startswith(("bench/", "suite/", "torture"))]
    for p in sorted(post, key=lambda p: -int(p[1])):
        print(f"{p[0]:24}" + "".join(m(int(x)) for x in p[1:]))
    for j, w in enumerate(("mesh", "mass", "check")):
        r = [p for p in post if p[0].startswith("bench/") and int(p[2 + j]) >= 0]
        print(f"bench sum {w}: {sum(int(p[2 + j]) for p in r) / 1e6:.0f}M over {sum(int(p[1]) for p in r) / 1e6:.0f}M regen")
