"""Rounding of loftmeet's hull points, SAT extents and quadric coefficients, measured.

Replays skinned.rs's `Poly::hull`, `Hull::extent` and `Hull::misses` in f64 (Python floats are IEEE
binary64 with round-to-nearest, no FMA, the same operation order as the Rust) and again in exact
rationals (fractions) from the SAME f64 inputs, and reports the worst error against the scale the
Lean README uses. Trig values (sin_cos, cos h) are inputs here: their own error (dmath: ~1 ulp)
is accounted separately. Run: python3 fp_check.py   (stdlib only, ~1 min)
"""
import math, random
from fractions import Fraction as F

U = 2.0 ** -53
rnd = random.Random(7)

def v_add(a, b): return [x + y for x, y in zip(a, b)]
def v_sub(a, b): return [x - y for x, y in zip(a, b)]
def v_mul(a, s): return [x * s for x in a]
def dot(a, b): return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]

def bern(c, b0, b1):
    # Poly::hull's `bern` closure, same operation order
    third = (b1 - b0) / 3 if isinstance(b0, float) else (b1 - b0) / F(3)
    val = lambda b: v_add(v_mul(v_add(v_mul(v_add(v_mul(c[3], b), c[2]), b), c[1]), b), c[0])
    der = lambda b: v_add(v_mul(v_add(v_mul(c[3], 3 * b), v_mul(c[2], 2)), b), c[1])
    return [val(b0), v_add(val(b0), v_mul(der(b0), third)), v_sub(val(b1), v_mul(der(b1), third)), val(b1)]

def hull(parts, b0, b1, trig):
    a, bb, dd = (bern(p, b0, b1) for p in parts)
    (c0, s0), (cm, sm), (c1, s1), ch = trig
    if isinstance(b0, float):
        rows = [(c0, s0), (cm / ch, sm / ch), (c1, s1)]
    else:
        rows = [(c0, s0), (cm / ch, sm / ch), (c1, s1)]
    return [[v_add(v_add(a[r], v_mul(bb[r], cx)), v_mul(dd[r], sy)) for (cx, sy) in rows] for r in range(4)]

def to_f(x):
    return [F(v) for v in x] if isinstance(x, list) else F(x)

def rand_vec(L): return [rnd.uniform(-L, L) for _ in range(3)]

def hull_check(trials=3000):
    worst = 0.0
    for _ in range(trials):
        L = 10.0 ** rnd.uniform(0, 4)
        parts = [[rand_vec(L) for _ in range(4)] for _ in range(3)]
        b0 = rnd.random(); b1 = b0 + rnd.random() * (1 - b0)
        x0 = rnd.uniform(-7, 7); half = rnd.uniform(-math.pi / 4, math.pi / 4)
        x1 = x0 + 2 * half
        trig = ((math.cos(x0), math.sin(x0)), (math.cos(x0 + half), math.sin(x0 + half)),
                (math.cos(x1), math.sin(x1)), math.cos(half))
        hf = hull(parts, b0, b1, trig)
        ftrig = tuple(tuple(F(t) for t in p) if isinstance(p, tuple) else F(p) for p in trig)
        hx = hull([[to_f(c) for c in p] for p in parts], F(b0), F(b1), ftrig)
        u = rand_vec(1.0); n = math.sqrt(dot(u, u)); u = [x / n for x in u]
        for r in range(4):
            for j in range(3):
                e = max(abs(F(hf[r][j][k]) - hx[r][j][k]) for k in range(3))
                ed = abs(F(dot(hf[r][j], u)) - sum(F(u[k]) * hx[r][j][k] for k in range(3)))
                worst = max(worst, float(e) / (U * L), float(ed) / (U * L))
    return worst

def misses_coefs(pts, w, o, a, b, c, exact):
    C3 = [1, 3, 3, 1]; C6 = [1, 6, 15, 20, 15, 6, 1]; cn = [1, 2, 1]; c2n = [1, 4, 6, 4, 1]
    n = 3
    h = lambda i, j: v_mul(v_sub(pts[i][j], o), w[j])
    mul = lambda m, v: [m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
                        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
                        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2]]
    zero = F(0) if exact else 0.0
    coef = [[zero] * 5 for _ in range(7)]
    mag = [[zero] * 5 for _ in range(7)]
    for i in range(4):
        for j in range(n):
            hij = h(i, j); ah = mul(a, hij)
            for k in range(4):
                for l in range(n):
                    t1 = dot(h(k, l), ah); t2 = 2.0 * dot(b, hij) * w[l]; t3 = c * w[j] * w[l]
                    v = t1 + t2 + t3
                    f = C3[i] * C3[k] * cn[j] * cn[l] / (C6[i + k] * c2n[j + l]) if not exact else \
                        F(C3[i] * C3[k] * cn[j] * cn[l], C6[i + k] * c2n[j + l])
                    coef[i + k][j + l] += v * f
                    if exact:
                        mag[i + k][j + l] += (abs(t1) + abs(t2) + abs(t3)) * f
    return coef, mag

def misses_check(trials=300):
    worst_T = 0.0; worst_rel = 0.0; flips = 0; rel_cases = 0
    for t in range(trials):
        # a curved piece near a plane whose frame origin is far away in-plane, or a big sphere
        far = 10.0 ** rnd.uniform(0, 6)
        size = 10.0 ** rnd.uniform(-3, 1)
        centre = [rnd.uniform(-far, far), rnd.uniform(-far, far), rnd.uniform(-1e-9, 1e-9) * size]
        ch = math.cos(rnd.uniform(0, math.pi / 4))
        pts = [[v_add(centre, [size * rnd.uniform(-1, 1), size * rnd.uniform(-1, 1),
                               size * 1e-3 * rnd.uniform(-1, 1)]) for _ in range(3)] for _ in range(4)]
        w = [1.0, ch, 1.0]
        if t % 2 == 0:
            o = [0.0, 0.0, 0.0]; a = [[0.0] * 3 for _ in range(3)]; b = [0.0, 0.0, 0.5]; c = 0.0
        else:
            R = far
            o = v_sub(centre, [0.0, 0.0, R]); a = [[1.0 if i == j else 0.0 for j in range(3)] for i in range(3)]
            b = [0.0, 0.0, 0.0]; c = -R * R
        cf, _ = misses_coefs(pts, w, o, a, b, c, False)
        cx, mag = misses_coefs([[to_f(p) for p in row] for row in pts], [F(x) for x in w], to_f(o),
                               [[F(x) for x in r] for r in a], to_f(b), F(c), True)
        mx = max(abs(v) for r in cf for v in r)
        for p in range(7):
            for q in range(5):
                e = abs(F(cf[p][q]) - cx[p][q])
                if mag[p][q] > 0:
                    worst_T = max(worst_T, float(e / mag[p][q]) / U)
                if mx > 0 and e > F(1e-12) * F(mx):
                    rel_cases += 1
                if (cf[p][q] > 0) != (cx[p][q] > 0) and cx[p][q] != 0:
                    flips += 1
                worst_rel = max(worst_rel, float(e) / mx if mx > 0 else 0.0)
    return worst_T, worst_rel, rel_cases, flips

if __name__ == "__main__":
    print(f"hull points and extents: max |error| = {hull_check():.1f} u L   (L = max |cubic coefficient|)")
    T, rel, cases, flips = misses_check()
    print(f"misses coefficients: max |error| = {T:.1f} u T   (T = sum of |terms| of that coefficient)")
    print(f"misses coefficients: max |error| / max|coef| = {rel:.3g}; "
          f"{cases} coefficients with error > 1e-12 max|coef| (the code's eps); {flips} sign flips")
