"""Schwartz-Zippel check of the general (3D) ellipse-pair identities at random integer data:
  Res_mu(pencil, column j) = q_j^4 F           (novel-math's form, every j)
  Res_t(q1, q2) on L     = |n1|^4 |n2|^4 |l|^8 F   (symmetric line form, this file)
  (alQ1 + ga)^2 - 4 be^2 Q1 = m1^2 |n2|^4 F    (plane-1 form, general_probe.py)
  F = E1^2 - 4 E0 E2 from the Pluecker coordinates   (the form proved in Lean, Proofs/General.lean)
Exact rationals; 30 random points."""
import itertools
import sympy as sp, random
random.seed(7)
mu, t = sp.symbols('mu t')
def rnd(): return sp.Matrix([random.randint(-20, 20) for _ in range(3)])
def pencil(c1,X1,Y1,c2,X2,Y2,j):
    M = sp.Matrix.hstack(X1, Y1, -X2, -Y2); r = c2 - c1
    minor = lambda i: M[:, [k for k in range(4) if k != i]].det()
    q = [(-1)**i*minor(i) for i in range(4)]
    cols = [k for k in range(4) if k != j]; Mj = M[:, cols]; dj = Mj.det()
    Pv = [0]*4
    for idx, k in enumerate(cols):
        Mk = Mj.copy(); Mk[:, idx] = r; Pv[k] = Mk.det()
    qp = [qi*dj/q[j] for qi in q]
    e1 = (Pv[0]+mu*qp[0])**2 + (Pv[1]+mu*qp[1])**2 - dj**2
    e2 = (Pv[2]+mu*qp[2])**2 + (Pv[3]+mu*qp[3])**2 - dj**2
    return sp.resultant(sp.expand(e1), sp.expand(e2), mu) / q[j]**4
def lineform(c1,X1,Y1,c2,X2,Y2):
    n1, n2 = X1.cross(Y1), X2.cross(Y2); l = n1.cross(n2); w = l.dot(l)
    W0 = n1.dot(c1)*n2.cross(l) + n2.dot(c2)*l.cross(n1)
    def q(c, X, Y, n):
        V = W0 + t*l - w*c
        return sp.expand(V.cross(X).dot(V.cross(X)) + V.cross(Y).dot(V.cross(Y)) - w**2*n.dot(n))
    return sp.resultant(q(c1,X1,Y1,n1), q(c2,X2,Y2,n2), t) / (n1.dot(n1)**2*n2.dot(n2)**2*w**4)
def planeform(c1,X1,Y1,c2,X2,Y2):
    n2 = X2.cross(Y2); a, b, d = n2.dot(X1), n2.dot(Y1), n2.dot(c2-c1)
    m1 = a*a+b*b; Q1 = m1-d*d; l = a*Y1-b*X1; U = m1*(c1-c2) + d*(a*X1+b*Y1)
    al = l.cross(X2).dot(l.cross(X2)) + l.cross(Y2).dot(l.cross(Y2))
    be = U.cross(X2).dot(l.cross(X2)) + U.cross(Y2).dot(l.cross(Y2))
    ga = U.cross(X2).dot(U.cross(X2)) + U.cross(Y2).dot(U.cross(Y2)) - m1**2*n2.dot(n2)
    return ((al*Q1+ga)**2 - 4*be**2*Q1) / (m1**2*n2.dot(n2)**2)
def plucker(c1,X1,Y1,c2,X2,Y2):
    A = [X1, Y1, -X2, -Y2, c1 - c2]
    p = {}
    for i, j in itertools.combinations(range(5), 2):
        p[i+1, j+1] = sp.Matrix.hstack(*[A[k] for k in range(5) if k not in (i, j)]).det()
    E0 = p[1,2]**2 - p[1,5]**2 - p[2,5]**2
    E1 = p[1,3]**2 + p[1,4]**2 + p[2,3]**2 + p[2,4]**2 - p[1,5]**2 - p[2,5]**2 - p[3,5]**2 - p[4,5]**2
    E2 = p[3,4]**2 - p[3,5]**2 - p[4,5]**2
    return E1**2 - 4*E0*E2
for it in range(30):
    data = [rnd() for _ in range(6)]
    F = [sp.nsimplify(pencil(*data, j)) for j in range(4)]
    L = lineform(*data); Pl = planeform(*data)
    assert len(set(F)) == 1 and F[0] == L == Pl == plucker(*data), (F, L, Pl)
print("30/30 random points: all four pencil forms, the line form, the plane form and the Pluecker form agree")
# homogeneity: scaling ellipse 1's data (c1 is affine, so scale X1, Y1 and the offset) by k scales F by k^?
data = [rnd() for _ in range(6)]
base = lineform(*data)
for k in (2, 3):
    c1, X1, Y1, c2, X2, Y2 = data
    s1 = lineform(c1, k*X1, k*Y1, c2, X2, Y2)
    print("X1,Y1 scaled by", k, ": F ratio", s1/base)
