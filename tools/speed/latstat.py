#!/usr/bin/env python3
"""The latency gate's statistics (stdlib only), over latency.sh's raw samples.

  latstat.py compare RAW [--wall-th 0.01] [--cpu-th 0.03] [--conf 0.99]
  latstat.py record RAW

RAW: one sample per line, `SIDE ROUND PART THREADS OP REP WALL_NS CPU_NS [RUN_NS]` (SIDE
base|head; RUN: the main thread's time on a CPU, the scheduler's clock, steal excluded).

A round is one process (one part, one thread setting, one side). Its value for an op is the
least of its reps for wall and run time (the noise of a shared machine, other tenants, steal,
only ever adds time, so the least rep is the steadiest estimate of the op's own time), and
the mean of its reps for CPU (which comes in 10 ms ticks: the mean is their total, to the tick).
Rounds of base and head were run back to back (ABAB, the order flipped each round), so they are
paired: per line the statistic is the median of head/base over the pairs, with a percentile
bootstrap CI (resampling pairs) and a Wilcoxon signed-rank p.

compare: a metric is `slower` when its CI lies wholly above 1, its estimate is over 1 + th, and
p < 1 - conf (both tests agree, and the effect is past the threshold). REGRESSED: wall slower
(any threads), or run time slower at t1 (one thread: the wait less what other tenants took).
IMPROVED: the mirror, on wall. `cpu+`/`cpu-`: CPU (user+sys, every thread) past cpu-th, a
warning, not a failure. THREADS HURT: on one side, a line whose all-cores rounds are slower than
its one-thread rounds of the same round (the same test, on wall): fastest wins, so the default
must not lose to one thread. Exit 1 when any line REGRESSED, or THREADS HURT on head but not on
base (one on base too is listed, `(base too)`, a warning).

record: per line the median over rounds and its bootstrap CI, the CPU median, per part and op
the speed-up at all cores (t1 wall / tN wall, information, never a pass) and CPU per wall second
(the cores kept busy), and the THREADS HURT lines.
"""
import math
import random
import statistics
import sys
from collections import defaultdict

B = 5000
WALL, CPU, RUN = 0, 1, 2


def load(path):
    reps = defaultdict(list)  # (side, round, part, thr, op) -> [(wall, cpu, run)]
    for line in open(path):
        f = line.split()
        if len(f) not in (8, 9):
            continue
        side, rnd, part, thr, op, _rep, wall, cpu = f[:8]
        run = int(f[8]) if len(f) == 9 else 0
        reps[(side, int(rnd), part, thr, op)].append((int(wall), int(cpu), run))
    rounds = defaultdict(dict)  # (part, thr, op) -> {(side, round): (wall, cpu, run)}
    for (side, rnd, part, thr, op), v in reps.items():
        rounds[(part, thr, op)][(side, rnd)] = (
            min(x[WALL] for x in v),
            statistics.fmean(x[CPU] for x in v),
            min(x[RUN] for x in v),
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
    """Two-sided Wilcoxon signed-rank p of the differences d (zeros dropped): exact for n <= 20
    without ties, else the normal approximation with tie correction."""
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
        lo = sum(counts[: int(math.floor(min(w, 2 * mean - w))) + 1])
        return min(1.0, 2 * lo / 2 ** n)
    var = n * (n + 1) * (2 * n + 1) / 24 - ties / 48
    z = (abs(w - mean) - 0.5) / math.sqrt(var) if var > 0 else 0.0
    return math.erfc(z / math.sqrt(2))


class Test:
    """Paired rounds a -> b of one metric: the median ratio b/a, its CI, the signed-rank p."""

    def __init__(self, a, b, m, rng, conf):
        ks = sorted(k for k in set(a) & set(b) if a[k][m] > 0 and b[k][m] > 0)
        self.n = len(ks)
        if not ks:
            return
        lr = [math.log(b[k][m] / a[k][m]) for k in ks]
        self.est = math.exp(statistics.median(lr))
        self.lo, self.hi = (math.exp(x) for x in boot_median(lr, conf, rng))
        self.p = wilcoxon(lr)
        self.a = statistics.median(a[k][m] for k in ks)
        self.b = statistics.median(b[k][m] for k in ks)

    def verdict(self, th, alpha):
        """1 slower, -1 faster, 0 neither."""
        if self.n == 0:
            return 0
        if self.lo > 1 and self.est > 1 + th and self.p < alpha:
            return 1
        if self.hi < 1 and self.est < 1 - th and self.p < alpha:
            return -1
        return 0

    def pct(self):
        if self.n == 0:
            return f"{'':>7} {'':>17}"
        return (f"{(self.est - 1) * 100:+6.2f}% [{(self.lo - 1) * 100:+6.2f},"
                f"{(self.hi - 1) * 100:+6.2f}]%")


def side(r, s):
    return {k: v for (x, k), v in r.items() if x == s}


def threads_hurt(rounds, s, th, conf, rng):
    """Lines (part, op) where all cores is slower than one thread on side s: {(part, op): Test}."""
    out = {}
    for (part, thr, op), r in rounds.items():
        if thr != "tN" or (part, "t1", op) not in rounds:
            continue
        t = Test(side(rounds[(part, "t1", op)], s), side(r, s), WALL, rng, conf)
        if t.verdict(th, 1 - conf) == 1:
            out[(part, op)] = t
    return out


def print_hurt(hurt, known=()):
    for (part, op), t in sorted(hurt.items()):
        tag = " (base too)" if (part, op) in known else ""
        print(f"THREADS HURT: {part} {op}  tN/t1 {t.pct()}"
              f" ({t.a / 1e6:.2f} -> {t.b / 1e6:.2f} ms){tag}")


def compare(path, wall_th, cpu_th, conf):
    alpha = 1 - conf
    rng = random.Random(1)
    rounds = load(path)
    bad = 0
    counts = defaultdict(int)
    print(f"# head/base per line: median over paired rounds, {conf:.0%} bootstrap CI, Wilcoxon p")
    print(f"# REGRESSED: CI wholly > 1, estimate > {1 + wall_th:.3f}, p < {alpha:.3f}, in wall"
          f" (or in run time at t1); cpu+: the same for CPU past {1 + cpu_th:.3f} (warning)")
    print(f"{'part':11} {'thr':3} {'op':5} {'n':>3} {'base ms':>9} {'head ms':>9}"
          f" {'wall':>7} {'CI':>17} {'p':>7}  {'run t1':>7} {'CI':>17}  {'cpu':>7} {'CI':>17}"
          f"  verdict")
    for key in sorted(rounds):
        part, thr, op = key
        r = rounds[key]
        b, h = side(r, "base"), side(r, "head")
        w = Test(b, h, WALL, rng, conf)
        if w.n == 0:
            continue
        c = Test(b, h, CPU, rng, conf)
        u = Test(b, h, RUN, rng, conf) if thr == "t1" else Test({}, {}, RUN, rng, conf)
        v = []
        wv, uv = w.verdict(wall_th, alpha), u.verdict(wall_th, alpha)
        if wv == 1 or uv == 1:
            v.append("REGRESSED" + ("" if wv == 1 else " (run)"))
            bad += 1
        elif wv == -1:
            v.append("IMPROVED")
        cv = c.verdict(cpu_th, alpha)
        if cv:
            v.append("cpu+" if cv == 1 else "cpu-")
        for x in v:
            counts[x.split()[0]] += 1
        print(f"{part:11} {thr:3} {op:5} {w.n:3} {w.a / 1e6:9.2f} {w.b / 1e6:9.2f}"
              f" {w.pct()} {w.p:7.4f}  {u.pct()}  {c.pct()}  {' '.join(v)}")
    # fastest wins: on head, all cores must not be slower than one thread; a line that is so on
    # base too is listed but does not fail this change
    hb = threads_hurt(rounds, "base", wall_th, conf, rng)
    hh = threads_hurt(rounds, "head", wall_th, conf, rng)
    print_hurt(hh, hb)
    new = [k for k in hh if k not in hb]
    if new:
        counts["THREADS HURT (new)"] += len(new)
        bad += len(new)
    old = [k for k in hh if k in hb]
    if old:
        counts["threads hurt (base too)"] += len(old)
    print("summary: " + (", ".join(f"{k} {n}" for k, n in sorted(counts.items())) or "no change"))
    return 1 if bad else 0


def record(path):
    rng = random.Random(1)
    rounds = load(path)
    print("# per line: median of rounds (one process each; a round's value is its least rep),")
    print("# 95% bootstrap CI, CPU median; speedup = t1 wall / tN wall (information, never a")
    print("# pass); busy = CPU / wall at tN (cores kept busy)")
    print(f"{'part':11} {'thr':3} {'op':5} {'n':>3} {'wall ms':>10} {'CI ms':>21} {'cpu ms':>10}")
    med = {}
    s = "head" if any(x == "head" for r in rounds.values() for x, _ in r) else "base"
    for key in sorted(rounds):
        part, thr, op = key
        vals = list(side(rounds[key], s).values())
        w = [x[WALL] / 1e6 for x in vals]
        c = [x[CPU] / 1e6 for x in vals]
        lo, hi = boot_median(w, 0.95, rng)
        med[key] = (statistics.median(w), statistics.median(c))
        print(f"{part:11} {thr:3} {op:5} {len(w):3} {med[key][0]:10.2f}"
              f" [{lo:9.2f},{hi:9.2f}] {med[key][1]:10.2f}")
    print()
    print(f"{'part':11} {'op':5} {'speedup':>8} {'busy':>6}")
    for (part, thr, op), (w, c) in sorted(med.items()):
        if thr == "tN" and (part, "t1", op) in med:
            print(f"{part:11} {op:5} {med[(part, 't1', op)][0] / w:8.2f} {c / w:6.2f}")
    print()
    hurt = threads_hurt(rounds, s, 0.01, 0.99, rng)
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
