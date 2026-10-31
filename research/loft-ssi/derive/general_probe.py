"""Probe the general (3D) ellipse-family condition: degree in s1 of the L-line form, and its factors."""
import sympy as sp, random
random.seed(1)
s = sp.symbols('s')
def rv(deg): return sp.Matrix([sum(random.randint(-9,9)*s**k for k in range(deg+1)) for _ in range(3)])
def cv(): return sp.Matrix([random.randint(-9,9) for _ in range(3)])
c1, X1, Y1 = rv(3), rv(3), rv(3)        # loft 1 section data, cubic in s
c2, X2, Y2 = cv(), cv(), cv()           # a fixed ellipse of loft 2
n1, n2 = X1.cross(Y1), X2.cross(Y2)
a, b, d = n2.dot(X1), n2.dot(Y1), n2.dot(c2 - c1)
m1 = a**2 + b**2
Q1 = m1 - d**2
l = a*Y1 - b*X1
U = m1*(c1 - c2) + d*(a*X1 + b*Y1)
al = l.cross(X2).dot(l.cross(X2)) + l.cross(Y2).dot(l.cross(Y2))
be = U.cross(X2).dot(l.cross(X2)) + U.cross(Y2).dot(l.cross(Y2))
ga = U.cross(X2).dot(U.cross(X2)) + U.cross(Y2).dot(U.cross(Y2)) - m1**2*n2.dot(n2)
F = sp.expand((al*Q1 + ga)**2 - 4*be**2*Q1)
P = sp.Poly(F, s)
print("degree in s:", P.degree())
fl = sp.factor_list(P)
print("content", fl[0])
for f, k in fl[1]:
    print("  factor deg", f.degree(), "mult", k)
print("m1 deg", sp.Poly(m1,s).degree(), "n1.n1 deg", sp.Poly(n1.dot(n1),s).degree())
n22 = n2.dot(n2)
print("n2.n2 =", n22, " content/n2^2 powers:", [sp.Rational(10400625, n22**k) for k in range(1,4)])
for name, e in [("alQ1+ga", sp.expand(al*Q1+ga)), ("be", sp.expand(be)), ("al", sp.expand(al)), ("ga", sp.expand(ga))]:
    q, r = sp.div(sp.Poly(e, s), sp.Poly(m1, s))
    print(name, "deg", sp.Poly(e,s).degree(), "divisible by m1:", r.is_zero, " content", sp.Poly(e,s).content())
# novel-math's pencil resultant, column j
Fmin = [f for f, k in fl[1] if f.degree() == 24][0]
mu = sp.symbols('mu')
M = sp.Matrix.hstack(X1, Y1, -X2, -Y2); r = c2 - c1
def minor(j): return M[:, [k for k in range(4) if k != j]].det()
q = [(-1)**i * minor(i) for i in range(4)]
for j in range(4):
    cols = [k for k in range(4) if k != j]
    Mj = M[:, cols]; dj = Mj.det()
    Pv = [0]*4
    for idx, k in enumerate(cols):
        Mk = Mj.copy(); Mk[:, idx] = r; Pv[k] = Mk.det()
    qp = [sp.cancel(qi*dj/q[j]) for qi in q]
    e1 = sp.expand((Pv[0]+mu*qp[0])**2 + (Pv[1]+mu*qp[1])**2 - dj**2)
    e2 = sp.expand((Pv[2]+mu*qp[2])**2 + (Pv[3]+mu*qp[3])**2 - dj**2)
    R = sp.Poly(sp.resultant(e1, e2, mu), s)
    qq, rem = sp.div(R, sp.Poly(sp.expand(q[j]**4), s))
    q2, rem2 = sp.div(qq, Fmin)
    print("j", j, "Res deg", R.degree(), "div q_j^4:", rem.is_zero, " quotient/Fmin:", q2.as_expr() if rem2.is_zero else "not divisible")
