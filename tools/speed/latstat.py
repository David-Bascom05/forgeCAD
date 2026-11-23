#!/usr/bin/env python3
"""The latency gate's statistics (stdlib only), over latency.sh's raw samples.

  latstat.py compare RAW [--wall-th 0.01] [--cpu-th 0.03] [--conf 0.99]
  latstat.py record RAW

RAW: one sample per line, `SIDE ROUND PART THREADS OP REP WALL_NS CPU_NS` (SIDE base|head).

A round is one process (one part, one thread setting, one side); its value for an op is the
median of its reps. Rounds of base and head were run back to back (ABAB, order flipped each
round), so they are paired: per line the statistic is the median of head/base over the pairs,
with a percentile bootstrap CI (resampling pairs) and a Wilcoxon signed-rank p as a cross-check.

compare: REGRESSED when the CI of head/base wall lies wholly above 1, its estimate is over
1 + wall-th, and the signed-rank p is under 1 - conf (both tests agree); IMPROVED the mirror. CPU (user+sys, all threads) is judged the same way against
cpu-th, and flagged `cpu+` (a warning: more CPU burnt for the same or less wait), not a failure.
THREADS HURT: on one side, a line whose all-cores rounds are slower than its one-thread rounds
of the same round (paired, the same test): fastest wins, so the default must not lose to one
thread. Exit 1 when any line REGRESSED, or THREADS HURT on head but not on base.

record: per line the median over rounds, its bootstrap CI, the CPU median, and per part and op
the speed-up at all cores (t1 wall / tN wall) and CPU per wall second (the cores kept busy).
"""
import math
import random
import statistics
import sys
from collections import defaultdict

B = 5000


def load(path):
    reps = defaultdict(list)  # (side, round, part, thr, op) -> [(wall, cpu)]
    for line in open(path):
        f = line.split()
        if len(f) != 8:
            continue
        side, rnd, part, thr, op, _rep, wall, cpu = f
        reps[(side, int(rnd), part, thr, op)].append((int(wall), int(cpu)))
    rounds = defaultdict(dict)  # (part, thr, op) -> {(side, round): (wall, cpu)}
    for (side, rnd, part, thr, op), v in reps.items():
        rounds[(part, thr, op)][(side, rnd)] = (
            statistics.median(w for w, _ in v),
            # (CPU comes in 10 ms ticks: the mean over the reps is their total, to the tick)
            statistics.fmean(c for _, c in v),
        )
    return rounds


def quantile(xs, q):
    xs = sorted(xs)
    i = q * (len(xs) - 1)
    lo = math.floor(i)
    hi = min(lo + 1, len(xs) - 1)
    return xs[lo] + (xs[hi] - xs[lo]) * (i - lo)


def boot_median(xs, conf, rng):
    """Percentile bootstrap CI of the median of xs."""
    n = len(xs)
    meds = [statistics.median(rng.choices(xs, k=n)) for _ in range(B)]
    a = (1 - conf) / 2
    return quantile(meds, a), quantile(meds, 1 - a)


def wilcoxon(d):
    """Two-sided Wilcoxon signed-rank p of the differences d (zeros dropped): exact for n <= 20,
    else the normal approximation with tie correction."""
    d = [x for x in d if x != 0]
    n = len(d)
    if n == 0:
        return 1.0
    order = sorted(range(n), key=lambda i: abs(d[i]))
    ranks = [0.0] * n
    i = 0
    ties = 0.0
    while i < n:
        j = i
        while j + 1 < n and abs(d[order[j + 1]]) == abs(d[order[i]]):
            j += 1
        r = (i + j) / 2 + 1
        for k in range(i, j + 1):
            ranks[order[k]] = r
        t = j - i + 1
        ties += t * t * t - t
        i = j + 1
    w = sum(r for r, x in zip(ranks, d) if x > 0)
    mean = n * (n + 1) / 4
    if n <= 20 and ties == 0:
        # exact: the distribution of the positive rank sum over 2^n sign choices
        counts = [1] + [0] * (n * (n + 1) // 2)
        for k in range(1, n + 1):
            for s in range(len(counts) - 1, k - 1, -1):
                counts[s] += counts[s - k]
        total = 2 ** n
        lo = sum(counts[: int(math.floor(min(w, 2 * mean - w))) + 1])
        return min(1.0, 2 * lo / total)
    var = n * (n + 1) * (2 * n + 1) / 24 - ties / 48
    z = (abs(w - mean) - 0.5) / math.sqrt(var) if var > 0 else 0.0
    return math.erfc(z / math.sqrt(2))


def paired(a, b, rng, conf):
    """Paired rounds a, b ({round: (wall, cpu)}): the median ratio b/a of wall and of CPU, their
    bootstrap CIs, the signed-rank p of wall, and the pairs' count."""
    ks = sorted(set(a) & set(b))
    if not ks:
        return None
    lw = [math.log(b[k][0] / a[k][0]) for k in ks]
    lc = [math.log(max(b[k][1], 1) / max(a[k][1], 1)) for k in ks]
    ew, ec = math.exp(statistics.median(lw)), math.exp(statistics.median(lc))
    wlo, whi = (math.exp(x) for x in boot_median(lw, conf, rng))
    clo, chi = (math.exp(x) for x in boot_median(lc, conf, rng))
    return ew, wlo, whi, ec, clo, chi, wilcoxon(lw), len(ks)


def slower(t, th, alpha):
    """A paired test's verdict on wall: 1 slower, -1 faster, 0 neither (CI wholly past 1, the
    estimate past the threshold, p under alpha)."""
    ew, wlo, whi, _, _, _, p, _ = t
    if wlo > 1 and ew > 1 + th and p < alpha:
        return 1
    if whi < 1 and ew < 1 - th and p < alpha:
        return -1
    return 0


def threads_hurt(rounds, side, th, conf, rng):
    """Lines (part, op) where all cores is slower than one thread on SIDE: {(part, op): test}."""
    out = {}
    for (part, thr, op), r in rounds.items():
        if thr != "tN" or (part, "t1", op) not in rounds:
            continue
        one = {k: v for (s, k), v in rounds[(part, "t1", op)].items() if s == side}
        alln = {k: v for (s, k), v in r.items() if s == side}
        t = paired(one, alln, rng, conf)
        if t and slower(t, th, 1 - conf) == 1:
            out[(part, op)] = t
    return out


def print_hurt(hurt, known=()):
    for (part, op), t in sorted(hurt.items()):
        ew, wlo, whi = t[0], t[1], t[2]
        tag = " (base too)" if (part, op) in known else ""
        print(f"THREADS HURT: {part} {op}  tN/t1 {(ew - 1) * 100:+.2f}%"
              f" [{(wlo - 1) * 100:+.2f},{(whi - 1) * 100:+.2f}]%{tag}")


def compare(path, wall_th, cpu_th, conf):
    alpha = 1 - conf
    rng = random.Random(1)
    rounds = load(path)
    bad = 0
    counts = defaultdict(int)
    print(f"# head/base per line: median over paired rounds, {conf:.0%} bootstrap CI, Wilcoxon p")
    print(f"# REGRESSED: wall CI wholly > 1, estimate > {1 + wall_th:.3f} and p < {alpha:.3f};"
          f" cpu+: the same for CPU past {1 + cpu_th:.3f} (warning)")
    print(f"{'part':11} {'thr':3} {'op':5} {'n':>3} {'base ms':>9} {'head ms':>9}"
          f" {'wall':>7} {'CI':>17} {'p':>7}  {'cpu':>7} {'CI':>17}  verdict")
    for key in sorted(rounds):
        part, thr, op = key
        r = rounds[key]
        base = {k: v for (side, k), v in r.items() if side == "base"}
        head = {k: v for (side, k), v in r.items() if side == "head"}
        t = paired(base, head, rng, conf)
        if not t:
            continue
        ew, wlo, whi, ec, clo, chi, p, n = t
        ks = sorted(set(base) & set(head))
        bw = statistics.median(base[k][0] for k in ks) / 1e6
        hw = statistics.median(head[k][0] for k in ks) / 1e6
        v = []
        w = slower(t, wall_th, alpha)
        if w == 1:
            v.append("REGRESSED")
            bad += 1
        elif w == -1:
            v.append("IMPROVED")
        if clo > 1 and ec > 1 + cpu_th:
            v.append("cpu+")
        elif chi < 1 and ec < 1 - cpu_th:
            v.append("cpu-")
        for x in v:
            counts[x] += 1
        print(f"{part:11} {thr:3} {op:5} {n:3} {bw:9.2f} {hw:9.2f}"
              f" {(ew - 1) * 100:+6.2f}% [{(wlo - 1) * 100:+6.2f},{(whi - 1) * 100:+6.2f}]%"
              f" {p:7.4f}  {(ec - 1) * 100:+6.2f}% [{(clo - 1) * 100:+6.2f},{(chi - 1) * 100:+6.2f}]%"
              f"  {' '.join(v)}")
    # fastest wins: on head, all cores must not be slower than one thread; a line that is so on
    # base too is listed but does not fail this change
    hb = threads_hurt(rounds, "base", wall_th, conf, rng)
    hh = threads_hurt(rounds, "head", wall_th, conf, rng)
    print_hurt(hh, hb)
    new = [k for k in hh if k not in hb]
    if new:
        counts["THREADS HURT (new)"] += len(new)
        bad += len(new)
    if hb:
        counts["threads hurt on base too"] += len([k for k in hh if k in hb])
    print("summary: " + (", ".join(f"{k} {n}" for k, n in sorted(counts.items())) or "no change"))
    return 1 if bad else 0


def record(path):
    rng = random.Random(1)
    rounds = load(path)
    print("# per line: median of rounds (one process each), 95% bootstrap CI, CPU median;")
    print("# speedup = t1 wall / tN wall; busy = CPU / wall at tN (cores kept busy)")
    print(f"{'part':11} {'thr':3} {'op':5} {'n':>3} {'wall ms':>10} {'CI ms':>21} {'cpu ms':>10}")
    med = {}
    for key in sorted(rounds):
        part, thr, op = key
        vals = [v for (s, _), v in rounds[key].items() if s == "head"] or list(rounds[key].values())
        w = [x[0] / 1e6 for x in vals]
        c = [x[1] / 1e6 for x in vals]
        lo, hi = boot_median(w, 0.95, rng)
        med[key] = (statistics.median(w), statistics.median(c))
        print(f"{part:11} {thr:3} {op:5} {len(w):3} {med[key][0]:10.2f}"
              f" [{lo:9.2f},{hi:9.2f}] {med[key][1]:10.2f}")
    print()
    print(f"{'part':11} {'op':5} {'speedup':>8} {'busy':>6}")
    for (part, thr, op), (w, c) in sorted(med.items()):
        if thr != "t1" and (part, "t1", op) in med:
            print(f"{part:11} {op:5} {med[(part, 't1', op)][0] / w:8.2f} {c / w:6.2f}")
    print()
    side = "head" if any(s == "head" for r in rounds.values() for s, _ in r) else "base"
    hurt = threads_hurt(rounds, side, 0.01, 0.99, rng)
    print_hurt(hurt)
    print(f"threads hurt: {len(hurt)} lines (all cores slower than one thread, paired rounds, 99%)")
    return 0


def main():
    a = sys.argv[1:]
    if len(a) < 2 or a[0] not in ("compare", "record"):
        print(__doc__)
        return 64
    opt = {"--wall-th": 0.01, "--cpu-th": 0.03, "--conf": 0.99}
    for i in range(2, len(a) - 1, 2):
        opt[a[i]] = float(a[i + 1])
    if a[0] == "compare":
        return compare(a[1], opt["--wall-th"], opt["--cpu-th"], opt["--conf"])
    return record(a[1])


if __name__ == "__main__":
    sys.exit(main())
