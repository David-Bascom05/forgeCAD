"""The two formulas forge-model's upto.rs proves torus ends with (needs sympy).

Run: fbuild python3 research/upto-proofs/check.py   (exits 1 on a mismatch)
"""

import sys

import sympy as sp

ok = True


def check(name, cond):
    global ok
    print(("ok   " if cond else "FAIL ") + name)
    ok &= bool(cond)


big, r, u, w = sp.symbols("R r u w", positive=True)
vx, vy, vz, t = sp.symbols("vx vy vz t")

# faces_ahead: on the torus, the outward normal along v is a positive multiple of
# (rho - R) (x vx + y vy) / rho + z vz
rho = big + r * sp.cos(w)
x, y, z = rho * sp.cos(u), rho * sp.sin(u), r * sp.sin(w)
px, py, pz = sp.symbols("px py pz")
f = (px**2 + py**2 + pz**2 + big**2 - r**2) ** 2 - 4 * big**2 * (px**2 + py**2)
grad = [sp.diff(f, s) for s in (px, py, pz)]
on = {px: x, py: y, pz: z}
along_grad = sum(g.subs(on) * v for g, v in zip(grad, (vx, vy, vz)))
along_code = (rho - big) * (x * vx + y * vy) / rho + z * vz
ratio = sp.simplify(along_grad / along_code)
check(f"torus normal along v: gradient / code = {ratio} > 0", sp.simplify(ratio - 8 * big * rho) == 0)

# ray_torus: the implicit along o + t d is c4[0] + c4[1] t + ... + c4[4] t^4
lx, ly, lz, dx, dy, dz = sp.symbols("lx ly lz dx dy dz")
qx, qy, qz = lx + t * dx, ly + t * dy, lz + t * dz
s2 = [lx * lx + ly * ly, 2 * (lx * dx + ly * dy), dx * dx + dy * dy]
fc = [s2[0] + lz * lz + big * big - r * r, s2[1] + 2 * lz * dz, s2[2] + dz * dz]
c4 = [
    fc[0] * fc[0] - 4 * big * big * s2[0],
    2 * fc[0] * fc[1] - 4 * big * big * s2[1],
    fc[1] * fc[1] + 2 * fc[0] * fc[2] - 4 * big * big * s2[2],
    2 * fc[1] * fc[2],
    fc[2] * fc[2],
]
along_ray = sp.expand(f.subs({px: qx, py: qy, pz: qz}))
check("ray against a torus: quartic coefficients", all(sp.expand(along_ray.coeff(t, i) - c4[i]) == 0 for i in range(5)))

sys.exit(0 if ok else 1)
