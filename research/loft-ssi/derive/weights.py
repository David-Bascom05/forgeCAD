"""SymPy: the cardinal-spline weight of datum d on span i, w_d(tau) (skinned.rs Spline::weights), in the
cubic Bernstein basis of the local tau = (s - k_i)/h. Prints the four coefficients the recipe uses."""
import sympy as sp
tau, h, mi, mj, yi, yj = sp.symbols('tau h m_i m_j y_i y_j')
a, b = 1 - tau, tau
w = a*yi + b*yj + ((a**3 - a)*mi + (b**3 - b)*mj)*h**2/6
B = [sp.binomial(3, k)*tau**k*(1 - tau)**(3 - k) for k in range(4)]
c = sp.symbols('c0:4')
sol = sp.solve(sp.Poly(sp.expand(sum(ci*Bi for ci, Bi in zip(c, B)) - w), tau).all_coeffs(), c)
for k in range(4):
    print(f"w[{k}] =", sp.factor(sol[c[k]]))
assert sp.simplify(sol[c[1]] - ((2*yi + yj)/3 - h**2*(2*mi + mj)/18)) == 0
assert sp.simplify(sol[c[2]] - ((yi + 2*yj)/3 - h**2*(mi + 2*mj)/18)) == 0
print("ok: w = [y_i, (2y_i+y_j)/3 - h^2(2m_i+m_j)/18, (y_i+2y_j)/3 - h^2(m_i+2m_j)/18, y_j]")
