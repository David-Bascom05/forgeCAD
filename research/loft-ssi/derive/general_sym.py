"""Symmetric line form: both ellipses cut the line L = plane1 ^ plane2; F_L = Res_t(q1, q2) with
q_i(t) = |V_i x X_i|^2 + |V_i x Y_i|^2 - w^2 |n_i|^2 at the point (W0 + t l)/w, V_i = W0 + t l - w c_i.
Compare with F_min (the primitive degree-24 factor)."""
import sympy as sp, random
random.seed(1)
s, t = sp.symbols('s t')
def rv(deg): return sp.Matrix([sum(random.randint(-9,9)*s**k for k in range(deg+1)) for _ in range(3)])
def cv(): return sp.Matrix([random.randint(-9,9) for _ in range(3)])
c1, X1, Y1 = rv(3), rv(3), rv(3)
c2, X2, Y2 = cv(), cv(), cv()
n1, n2 = X1.cross(Y1), X2.cross(Y2)
l = n1.cross(n2)
w = l.dot(l)
W0 = n1.dot(c1)*n2.cross(l) + n2.dot(c2)*l.cross(n1)
def q(c, X, Y, n):
    V = W0 + t*l - w*c
    return sp.expand(V.cross(X).dot(V.cross(X)) + V.cross(Y).dot(V.cross(Y)) - w**2*n.dot(n))
q1, q2 = q(c1,X1,Y1,n1), q(c2,X2,Y2,n2)
R = sp.Poly(sp.resultant(q1, q2, t), s)
print("Res_L degree in s", R.degree())
fl = sp.factor_list(R)
print("content", fl[0])
for f, k in fl[1]:
    print(" factor deg", f.degree(), "mult", k)
for name, e in [("|l|^2", w), ("|n1|^2", n1.dot(n1)), ("n2.n2", n2.dot(n2))]:
    print(name, sp.factor(e) if e.is_number else sp.factor_list(sp.Poly(e, s))[1].__len__() and [ (f.degree(),k) for f,k in sp.factor_list(sp.Poly(e,s))[1]])
