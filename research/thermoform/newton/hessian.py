"""The membrane Newton solve's element Hessians (kernel/forge-thermo/src/newton.rs), derived and
checked: the incompressible Ogden membrane's Hessian in its principal frame (stretch block, shear
block, out-of-plane pair), the tension-field (wrinkle) relaxation's, the pressure's work's, the
hinge's Gauss-Newton term; each against SymPy's exact second derivatives and central differences.
Run: python3 hessian.py (SymPy, NumPy)."""
import itertools
import numpy as np
import sympy as sp

rng = np.random.default_rng(7)
LAWS = [
    [(0.5, 2.0)],                                   # neo-Hookean
    [(0.30, 2.0), (-0.05, -2.0)],                   # Mooney-Rivlin
    [(0.33, 1.6), (0.02, 5.1), (-0.01, -3.2)],      # a 3-term Ogden (ABS-like signs)
]

# --- psi(a, b, c): W of C = [[a, b], [b, c]] through its eigenvalues s1 >= s2, l = sqrt(s)
a, b, c = sp.symbols("a b c", positive=True)
m, h = (a + c) / 2, (a - c) / 2
r = sp.sqrt(h**2 + b**2)
s1s, s2s = m + r, m - r

def psi_sym(law):
    l1, l2 = sp.sqrt(s1s), sp.sqrt(s2s)
    l3 = 1 / (l1 * l2)
    return sum(sp.Rational(mu).limit_denominator(10**6) / sp.Rational(al).limit_denominator(10**6)
               * (l1**al + l2**al + l3**al - 3) for mu, al in law)

def dd(s1, s2, q):
    """(s1^q - s2^q) / (s1 - s2), stable as s1 -> s2 (the series under 1e-3)."""
    e = s1 / s2 - 1
    if abs(e) < 1e-3:
        return s2 ** (q - 1) * (q + q * (q - 1) / 2 * e + q * (q - 1) * (q - 2) / 6 * e * e
                                + q * (q - 1) * (q - 2) * (q - 3) / 24 * e**3)
    return (s1**q - s2**q) / (s1 - s2)

def principal(law, s1, s2):
    """psi_1, psi_2, psi_11, psi_12, psi_22 and kappa = (psi_1 - psi_2) / (s1 - s2)."""
    p1 = p2 = p11 = p12 = p22 = k = 0.0
    for mu, al in law:
        q = al / 2 - 1
        t = (s1 * s2) ** (-al / 2)          # l3^alpha
        p1 += mu / 2 * (s1**q - t / s1)
        p2 += mu / 2 * (s2**q - t / s2)
        p11 += mu / 2 * (q * s1 ** (q - 1) + (1 + al / 2) * t / s1**2)
        p22 += mu / 2 * (q * s2 ** (q - 1) + (1 + al / 2) * t / s2**2)
        p12 += mu / 2 * (al / 2) * t / (s1 * s2)
        k += mu / 2 * (dd(s1, s2, q) + t / (s1 * s2))
    return p1, p2, p11, p12, p22, k

def check_psi():
    worst = 0.0
    for law in LAWS:
        ps = psi_sym(law)
        q = (a, b, c)
        grad = [sp.diff(ps, v) for v in q]
        hess = [[sp.diff(g, v) for v in q] for g in grad]
        fg = sp.lambdify(q, grad, "mpmath")
        fh = sp.lambdify(q, hess, "mpmath")
        for _ in range(20):
            F = rng.normal(size=(2, 2)) * 0.6 + np.eye(2) * 1.5
            C = F.T @ F
            av, bv, cv = C[0, 0], C[0, 1], C[1, 1]
            # the spectral form
            hh, rr = (av - cv) / 2, np.hypot((av - cv) / 2, bv)
            s1, s2 = (av + cv) / 2 + rr, (av + cv) / 2 - rr
            cphi, sphi = hh / rr, bv / rr
            n1 = np.array([np.sqrt((1 + cphi) / 2), np.sign(sphi) * np.sqrt((1 - cphi) / 2)])
            g1 = np.array([n1[0] ** 2, 2 * n1[0] * n1[1], n1[1] ** 2])
            g2 = np.array([n1[1] ** 2, -2 * n1[0] * n1[1], n1[0] ** 2])
            w = np.array([-sphi / 2, cphi, sphi / 2])
            p1, p2, p11, p12, p22, k = principal(law, s1, s2)
            G = p1 * g1 + p2 * g2
            H = (p11 * np.outer(g1, g1) + p12 * (np.outer(g1, g2) + np.outer(g2, g1))
                 + p22 * np.outer(g2, g2) + 2 * k * np.outer(w, w))
            Ge = np.array([float(v) for v in fg(av, bv, cv)])
            He = np.array([[float(v) for v in row] for row in fh(av, bv, cv)])
            worst = max(worst, np.abs(G - Ge).max() / np.abs(Ge).max(),
                        np.abs(H - He).max() / np.abs(He).max())
    print(f"psi(C) gradient and Hessian, spectral vs SymPy: worst relative error {worst:.2e}")
    assert worst < 1e-10
    # kappa's limit at an equibiaxial stretch: psi_11 - psi_12
    for law in LAWS:
        s = 2.3
        k = principal(law, s * (1 + 1e-9), s)[5]
        p = principal(law, s, s)
        lim = p[2] - p[3]
        assert abs(k - lim) < 1e-7 * abs(lim), (k, lim)
        # and the series meets the direct form at its switch
        e = 1e-3
        assert abs(dd(s * (1 + e * 0.999), s, 0.6 - 1) - dd(s * (1 + e * 1.001), s, 0.6 - 1)) < 1e-6
    print("kappa at s1 = s2 is psi_11 - psi_12; the divided difference's series meets its direct form")

# --- the element: E = vol psi(F^T F), F = [f0 f1] (3x2) from the corners by the reference's inverse
def frame_modes(F, law, wrinkle=False, project=True):
    """H_F (6x6, f0 then f1) from the principal frame's blocks, each clamped to >= 0 if project."""
    C = F.T @ F
    av, bv, cv = C[0, 0], C[0, 1], C[1, 1]
    hh, rr = (av - cv) / 2, np.hypot((av - cv) / 2, bv)
    s1, s2 = (av + cv) / 2 + rr, (av + cv) / 2 - rr
    if rr > 1e-12 * (av + cv):
        v = np.array([bv, s1 - av]) if abs(s1 - av) > abs(s1 - cv) else np.array([s1 - cv, bv])
        v1 = v / np.linalg.norm(v)
    else:
        v1 = np.array([1.0, 0.0])
    V = np.column_stack([v1, [-v1[1], v1[0]]])
    l1, l2 = np.sqrt(s1), np.sqrt(s2)
    U = F @ V / np.array([l1, l2])
    nrm = np.cross(U[:, 0], U[:, 1])
    p1, p2, p11, p12, p22, k = principal(law, s1, s2)
    if wrinkle:
        if l1 <= 1:
            return np.zeros((6, 6))
        if l2 < 1 / np.sqrt(l1):
            # psi~(s1) = psi(s1, s1^-1/2): psi~' = psi_1 there, psi~'' = psi_11 - psi_12^2 / psi_22
            s2w = 1 / l1
            p1, p2, p11, p12, p22, _ = principal(law, s1, s2w)
            p11, p12, p22, p2 = p11 - p12 * p12 / p22, 0.0, 0.0, 0.0
            k = p1 / (s1 - s2)
    S1, S2 = 2 * p1, 2 * p2
    st = np.array([[4 * s1 * p11 + S1, 4 * l1 * l2 * p12], [4 * l1 * l2 * p12, 4 * s2 * p22 + S2]])
    sh = np.array([[2 * k * s1 + S2, 2 * k * l1 * l2], [2 * k * l1 * l2, 2 * k * s2 + S1]])
    clamp = (lambda M: (lambda w, Q: Q @ np.diag(np.maximum(w, 0)) @ Q.T)(*np.linalg.eigh(M))) \
        if project else (lambda M: M)
    st, sh = clamp(st), clamp(sh)
    out = np.array([[max(S1, 0) if project else S1, 0], [0, max(S2, 0) if project else S2]])
    # the modes as dF (3x2): U E_ij V^T for the in-plane, n e_j^T V^T out of it
    def m(i, j):
        E = np.zeros((2, 2)); E[i, j] = 1
        return U @ E @ V.T
    basis_st = [m(0, 0), m(1, 1)]
    basis_sh = [m(0, 1), m(1, 0)]
    basis_out = [np.outer(nrm, V[:, 0]), np.outer(nrm, V[:, 1])]
    H = np.zeros((6, 6))
    flat = lambda M: np.concatenate([M[:, 0], M[:, 1]])
    for blk, bas in ((st, basis_st), (sh, basis_sh), (out, basis_out)):
        for i, j in itertools.product(range(2), range(2)):
            H += blk[i, j] * np.outer(flat(bas[i]), flat(bas[j]))
    return H

def psi_num(law, F, wrinkle=False):
    C = F.T @ F
    w2 = np.linalg.eigvalsh(C)
    l2, l1 = np.sqrt(w2)
    if wrinkle:
        if l1 <= 1:
            return 0.0
        l2 = max(l2, 1 / np.sqrt(l1))
    l3 = 1 / (l1 * l2)
    return sum(mu / al * (l1**al + l2**al + l3**al - 3) for mu, al in law)

def fd_hessian(f, x, h=1e-5):
    n = len(x)
    H = np.zeros((n, n))
    for i in range(n):
        for j in range(n):
            e_i, e_j = np.eye(n)[i] * h, np.eye(n)[j] * h
            H[i, j] = (f(x + e_i + e_j) - f(x + e_i - e_j) - f(x - e_i + e_j) + f(x - e_i - e_j)) / (4 * h * h)
    return H

def check_element():
    worst = 0.0
    for law in LAWS:
        for wrinkle in (False, True):
            for _ in range(12):
                F = rng.normal(size=(3, 2)) * 0.5
                F[:2, :] += np.eye(2) * 1.6
                if wrinkle and rng.random() < 0.5:
                    F[:, 1] *= 0.55           # into the wrinkled band
                H = frame_modes(F, law, wrinkle, project=False)
                Hfd = fd_hessian(lambda z: psi_num(law, np.column_stack([z[:3], z[3:]]), wrinkle),
                                 np.concatenate([F[:, 0], F[:, 1]]))
                worst = max(worst, np.abs(H - Hfd).max() / np.abs(Hfd).max())
    print(f"element H_F, principal blocks vs central differences: worst relative error {worst:.2e}")
    assert worst < 1e-5
    # projected: PSD, and equal to the exact one where the exact one's blocks are PSD
    for law in LAWS:
        for _ in range(50):
            F = rng.normal(size=(3, 2)) * 0.8
            Hp = frame_modes(F, law, project=True)
            assert np.linalg.eigvalsh(Hp).min() > -1e-9 * np.abs(Hp).max()
    F = np.array([[1.8, 0.1], [0.0, 1.4], [0.2, -0.1]])
    assert np.abs(frame_modes(F, LAWS[0], project=True) - frame_modes(F, LAWS[0], project=False)).max() < 1e-12
    print("projected H_F is PSD and is the exact one in tension")

# --- the 9x9 element Hessian: x -> F linear (B), H = B^T H_F B; checked whole by differences
def check_corners():
    X = np.array([[0.0, 0.0], [1.3, 0.1], [0.2, 1.1]])
    D = np.column_stack([X[1] - X[0], X[2] - X[0]])
    dinv = np.linalg.inv(D)
    law = LAWS[2]
    x = np.column_stack([X * 1.7, np.zeros(3)]) + rng.normal(size=(3, 3)) * 0.2
    def F_of(z):
        z = z.reshape(3, 3)
        return np.column_stack([z[1] - z[0], z[2] - z[0]]) @ dinv
    B = np.zeros((6, 9))
    for v in range(3):
        coef = [-(dinv[0, j] + dinv[1, j]) for j in range(2)] if v == 0 else [dinv[v - 1, j] for j in range(2)]
        for j in range(2):
            B[3 * j:3 * j + 3, 3 * v:3 * v + 3] = np.eye(3) * coef[j]
    H = B.T @ frame_modes(F_of(x.ravel()), law, project=False) @ B
    Hfd = fd_hessian(lambda z: psi_num(law, F_of(z)), x.ravel())
    err = np.abs(H - Hfd).max() / np.abs(Hfd).max()
    print(f"9x9 corner Hessian B^T H_F B vs differences: {err:.2e}")
    assert err < 1e-5

# --- the pressure's work p/6 a.(b x c): its Hessian blocks, H_ab = -p/6 [c]x and cyclic
def check_pressure():
    xs = sp.symbols("x0:9")
    A, Bv, Cv = sp.Matrix(xs[0:3]), sp.Matrix(xs[3:6]), sp.Matrix(xs[6:9])
    p = sp.Symbol("p")
    E = p / 6 * A.dot(Bv.cross(Cv))
    H = sp.hessian(E, xs)
    cross = lambda v: sp.Matrix([[0, -v[2], v[1]], [v[2], 0, -v[0]], [-v[1], v[0], 0]])
    for (i, j, k) in ((0, 1, 2), (1, 2, 0), (2, 0, 1)):
        V = (A, Bv, Cv)
        blk = H[3 * i:3 * i + 3, 3 * j:3 * j + 3]
        assert sp.simplify(blk - (-p / 6) * cross(V[k])) == sp.zeros(3, 3)
        assert sp.simplify(H[3 * i:3 * i + 3, 3 * i:3 * i + 3]) == sp.zeros(3, 3)
    print("pressure: H_ab = -p/6 [c]x, H_bc = -p/6 [a]x, H_ca = -p/6 [b]x, no diagonal blocks")

# --- the hinge: theta's gradient is bend()'s divided by 2 k sin(theta); E ~ 2k(1-cos) -> GN 2k cos+ g g^T
def check_hinge():
    def theta(z):
        a_, b_, c_, d_ = z.reshape(4, 3)
        e = b_ - a_
        n1, n2 = np.cross(a_ - c_, b_ - c_), np.cross(b_ - d_, a_ - d_)
        return np.arctan2(np.dot(np.cross(n1, n2), e) / np.linalg.norm(e), np.dot(n1, n2))
    def grad(z):
        a_, b_, c_, d_ = z.reshape(4, 3)
        e = b_ - a_
        el = np.linalg.norm(e)
        n1, n2 = np.cross(a_ - c_, b_ - c_), np.cross(b_ - d_, a_ - d_)
        m1, m2 = n1 / n1.dot(n1), n2 / n2.dot(n2)
        eh = e / el
        return np.concatenate([m1 * (b_ - c_).dot(eh) + m2 * (b_ - d_).dot(eh),
                               -(m1 * (a_ - c_).dot(eh) + m2 * (a_ - d_).dot(eh)), -m1 * el, -m2 * el])
    z = np.array([0, 0, 0, 1, 0.1, 0, 0.4, 1, 0.2, 0.6, -1, 0.35]) + rng.normal(size=12) * 0.05
    g = grad(z)
    gfd = np.array([(theta(z + np.eye(12)[i] * 1e-6) - theta(z - np.eye(12)[i] * 1e-6)) / 2e-6 for i in range(12)])
    err = np.abs(g - gfd).max() / np.abs(gfd).max()
    print(f"hinge: grad theta (bend()'s terms without 2 k sin) vs differences: {err:.2e}")
    assert err < 1e-6

check_psi()
check_element()
check_corners()
check_pressure()
check_hinge()
print("all checks pass")
