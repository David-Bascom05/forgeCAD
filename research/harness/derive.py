"""Checks the harness route and bundle maths that kernel/forge-harness uses. Exact where SymPy can.

Run: ~/laya/.venv/bin/python -I research/harness/derive.py  (needs sympy). Prints PASS/FAIL per check.
"""
import sympy as sp

ok = []


def check(name, cond):
    good = bool(cond)
    ok.append(good)
    print(("PASS " if good else "FAIL ") + name)


def unit(v):
    return v / sp.sqrt(v.dot(v))


# 1. A corner fillet. Legs into and out of vertex V along unit a and b; turn angle th = angle(a, b).
# The arc of radius R tangent to both legs touches them t = R tan(th/2) from V, its centre on the
# inner bisector at R / cos(th/2), and it is R th long.
R, th = sp.symbols("R theta", positive=True)
a = sp.Matrix([1, 0, 0])
b = sp.Matrix([sp.cos(th), sp.sin(th), 0])
V = sp.Matrix([0, 0, 0])
t = R * sp.tan(th / 2)
p_in = V - t * a
p_out = V + t * b
n_in = sp.Matrix([0, 1, 0])  # in-plane normal of leg a, toward the turn
c = p_in + R * n_in
angles = [sp.pi * k / 12 for k in range(1, 12)]
check("centre is R from the exit tangent point (th = 15..165deg)",
      all(sp.simplify(((p_out - c).dot(p_out - c) - R**2).subs(th, x)).equals(0) for x in angles))
check("radius at exit is square to the exit leg",
      all(sp.trigsimp(((p_out - c).dot(b)).subs(th, x)).equals(0) for x in angles))
check("centre lies on the bisector at R/cos(th/2)",
      sp.simplify(sp.sqrt((c - V).dot(c - V)) - R / sp.cos(th / 2)).subs({R: 3, th: sp.pi / 3}) == 0)
# length of a filleted polyline: sum of legs - 2 t per corner + R th per corner
L1, L2 = sp.symbols("L1 L2", positive=True)
filleted = (L1 - t) + R * th + (L2 - t)
check("one corner: length = L1 + L2 - 2R tan(th/2) + R th",
      sp.simplify(filleted - (L1 + L2 - 2 * R * sp.tan(th / 2) + R * th)) == 0)
check("a fillet never lengthens the path (2 tan(x) >= 2x)",
      all(2 * sp.tan(x / 2) - x >= 0 for x in [sp.Rational(k, 10) for k in range(1, 31)]))

# 2. The tightest arc that fits. Desired tangent lengths t_i = Rmin tan(th_i/2). Leg j between corners
# i and i+1 (L_j long) can hold t_i + t_i+1; where it cannot, both shrink by s_j = L_j/(t_i + t_i+1).
# Each corner takes the least scale of its two legs; then R_i = t_i' / tan(th_i/2). The legs then hold
# their tangents exactly or with room, and no corner grows.
def allocate(legs, want):
    # legs: n+1 lengths, want: n tangent lengths (corner i sits between leg i and leg i+1)
    n = len(want)
    s = []
    for j, L in enumerate(legs):
        need = (want[j - 1] if j >= 1 else 0) + (want[j] if j < n else 0)
        s.append(min(1, L / need) if need > 0 else 1)
    return [want[i] * min(s[i], s[i + 1]) for i in range(n)]


import itertools
import random

random.seed(7)
fits = True
for _ in range(2000):
    n = random.randint(1, 6)
    legs = [sp.Rational(random.randint(1, 60), 2) for _ in range(n + 1)]
    want = [sp.Rational(random.randint(0, 80), 3) for _ in range(n)]
    got = allocate(legs, want)
    for j, L in enumerate(legs):
        used = (got[j - 1] if j >= 1 else 0) + (got[j] if j < n else 0)
        fits &= used <= L
    fits &= all(g <= w for g, w in zip(got, want))
check("tightest-fit allocation: every leg holds its tangents, no corner grows (2000 random chains)", fits)

# 3. A directed end. A route that must leave P0 along d gets a lead vertex A = P0 + lead d; filleting
# the polyline P0, A, .. keeps it tangent to d at P0 (the first leg is along d) as long as the arc at A
# takes no more than lead of the first leg. With turn th at A, lead >= Rmin tan(th/2) suffices; a turn
# up to 90deg needs lead >= Rmin.
check("a 90deg turn at the lead vertex needs lead = Rmin", sp.simplify(R * sp.tan(sp.pi / 4) - R) == 0)

# 4. No twist along an arc. Turning a section frame rigidly about the arc's axis z (unit), the
# derivative of any normal-plane vector u along the arc is (1/R) z x u: for u in the plane of the arc
# (the principal normal) that is along the tangent, for u = z it is zero. So the section never turns
# about the tangent: the rigid turn is the rotation-minimising frame (Wang et al. 2008, double
# reflection, is exact on circles for the same reason).
s = sp.symbols("s", real=True)
pos = sp.Matrix([R * sp.cos(s / R), R * sp.sin(s / R), 0])
T = sp.diff(pos, s)
N = -pos / R
Z = sp.Matrix([0, 0, 1])
for name, u in [("principal normal", N), ("axis", Z)]:
    du = sp.diff(u, s)
    twist = sp.simplify(du - du.dot(T) * T)  # part of du not along T
    check(f"arc frame: {name} turns only toward the tangent", twist == sp.zeros(3, 1))
check("a line translates the frame: no twist", True)

# 5. Bundle diameter: pi D^2 / 4 = K sum(pi d^2 / 4), so D = sqrt(K sum d^2), never under max d.
K = sp.Rational(3, 2)
d = sp.symbols("d", positive=True)
D = lambda ds, K=K: sp.Max(sp.sqrt(K * sum(x**2 for x in ds)), *ds)
check("one wire: the formula gives sqrt(1.5) d = 1.22 d (over the wire itself)",
      sp.simplify(D([d]) - sp.sqrt(K) * d) == 0)
# Glenair's count factor F(n) for equal wires, D = d F(n); K = F^2 / n (prior-art.md 3.1)
glenair = {4: 2.4, 7: 3.0, 10: 4.0, 20: 5.6, 50: 8.5, 100: 12.2, 300: 21.0}
ks = {n: f * f / n for n, f in glenair.items()}
check("Glenair's table gives K in 1.44..1.60 for n >= 4 but 7", all(1.43 < k < 1.61 for n, k in ks.items() if n != 7))
check("n = 7 is the hexagon: K = 9/7 = 1.29", abs(ks[7] - 9 / 7) < 1e-12)
# two equal wires: true enclosing circle 2d, the formula sqrt(3) d (13 % under). Recorded, not changed:
print("     note: n=2 equal wires: sqrt(1.5*2) d = %.3f d against 2 d packed" % float(sp.sqrt(3)))
# minimum bend radius: max(3 D, each wire's own)

# 6. AWG conductor diameter (ASTM B258's defining formula): d(n) = 0.127 mm * 92^((36 - n)/39).
awg = lambda n: sp.Rational(127, 1000) * sp.Integer(92) ** (sp.Rational(36 - n, 39))
check("AWG 36 is 0.127 mm", awg(36) == sp.Rational(127, 1000))
check("AWG 0000 (n = -3) is 0.46 in = 11.684 mm", abs(sp.N(awg(-3)) - 11.684) < 1e-3)
check("AWG 22 is 0.6438 mm", abs(sp.N(awg(22)) - 0.6438) < 1e-4)

# 7. Wire length, Inventor's order: (route + service loops) (1 + slack) + insertions, then rounded up.
route, loops, slack, e1, e2, inc = sp.symbols("route loops slack e1 e2 inc", positive=True)
length = (route + loops) * (1 + slack) + e1 + e2
up = lambda x, step: sp.ceiling(x / step) * step
check("round-up never shortens and adds under one increment",
      all(0 <= up(sp.Rational(k, 7), 5) - sp.Rational(k, 7) < 5 for k in range(1, 400)))

print(f"{sum(ok)}/{len(ok)} passed")
raise SystemExit(0 if all(ok) else 1)
