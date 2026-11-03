#!/usr/bin/env python3
"""Tables from results/trim-*.log and results/trim-cost.txt. Run from research/sheet-metal: python3 summarise.py"""
import collections
import glob
import re

KINDS = ["isolated", "overlap", "overlap-lines", "boundary", "tangent", "coincident", "mix", "nudged"]
tab = collections.defaultdict(collections.Counter)
miss = collections.defaultdict(float)
why = collections.defaultdict(collections.Counter)
area = {}
worst_ab = collections.defaultdict(float)
for f in glob.glob("results/trim-*.log"):
    for line in open(f):
        m = re.match(r"(\S+) (\S+) (\d+) (\d+) (.*)", line)
        if not m:
            continue
        opt, kind, n, seed, rest = m.groups()
        key = (kind, int(n), opt)
        if rest.startswith("Good"):
            v = "good"
        elif rest.startswith("Pinched"):
            v = "pinched"
        elif rest.startswith("REFUSED"):
            v = "refused"
            if "ZeroThickness" in rest:
                v = "zero-thickness"
            if "NearCoincident" in rest:
                v = "near-coincident"
            why[key][re.sub(r"[ {(].*", "", rest.split("why: ")[-1] if "why: " in rest else rest[8:])] += 1
            if "Internal(" in rest or "Unsupported(" in rest:
                why[key][re.search(r"(Internal|Unsupported)\((\w+)", rest).group(2)] += 1
        elif rest.startswith("PANIC"):
            v = "panic"
        else:
            v = "wrong"
            why[key][rest[:60]] += 1
        tab[key][v] += 1
        a = re.search(r"area (\S+) ref (\S+) err (\S+)", rest)
        if a:
            got, ref = float(a.group(1)), float(a.group(2))
            miss[key] = max(miss[key], abs(got - ref))
            area[(kind, int(n), int(seed), opt)] = got

print("| set | n | option | cases | good | pinched (reported) | zero-thickness refusal | near-coincident refusal | other refusal | wrong | panic | worst area miss |")
print("|---|---|---|---|---|---|---|---|---|---|---|---|")
for k in KINDS:
    for n in (2, 20, 200):
        for o in ("a", "bseq", "bnary"):
            c = tab[(k, n, o)]
            tot = sum(c.values())
            if tot:
                print(f"| {k} | {n} | {o} | {tot} | {c['good']} | {c['pinched']} | {c['zero-thickness']} | {c['near-coincident']} | {c['refused']} | {c['wrong']} | {c['panic']} | {miss[(k, n, o)]:.1e} |")
print()
tot = collections.Counter()
for (k, n, o), c in tab.items():
    for v, x in c.items():
        tot[(o, v)] += x
for o in ("a", "bseq", "bnary"):
    print(o, {v: x for (p, v), x in tot.items() if p == o})
print()
for key, c in sorted(why.items()):
    print(key, dict(c))
print()
# A and B on the same case, where both answered
ab = collections.Counter()
worst = 0.0
for (k, n, s, o), got in area.items():
    if o != "a":
        continue
    other = area.get((k, n, s, "bseq"))
    if other is not None:
        d = abs(got - other)
        worst = max(worst, d)
        ab["both"] += 1
print("A and bseq both answered on", ab["both"], "cases; worst area difference", f"{worst:.2e}")
# A pinched against B zero-thickness, per case
agree = collections.Counter()
per = collections.defaultdict(dict)
for f in glob.glob("results/trim-*.log"):
    for line in open(f):
        m = re.match(r"(\S+) (\S+) (\d+) (\d+) (\S+) ?(.*)", line)
        if m:
            opt, kind, n, seed, a0, rest = m.groups()
            per[(kind, n, seed)][opt] = a0 + " " + rest[:40]
for key, d in per.items():
    if "a" in d and "bseq" in d:
        a_p = d["a"].startswith("Pinched")
        b_z = "ZeroThickness" in d["bseq"]
        agree[(a_p, b_z)] += 1
print("(A pinched, bseq zero-thickness):", dict(agree))
print()
print("| set | n | a | bseq | bnary | bseq/a | bnary/a |  (instructions per case)")
print("|---|---|---|---|---|---|---|")
cost = collections.defaultdict(dict)
try:
    for line in open("results/trim-cost.txt"):
        if line.startswith("#"):
            continue
        k, n, c, o, x = line.split()
        cost[(k, int(n))][o] = int(x) / int(c)
except FileNotFoundError:
    pass
for k in KINDS:
    for n in (2, 20, 200):
        d = cost.get((k, n))
        if d:
            print(f"| {k} | {n} | {d['a']:.3g} | {d['bseq']:.3g} | {d['bnary']:.3g} | {d['bseq']/d['a']:.1f} | {d['bnary']/d['a']:.1f} |")
