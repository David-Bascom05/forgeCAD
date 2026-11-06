# The membrane's Newton solve

`kernel/forge-thermo/src/newton.rs` replaced the inner L-BFGS of `stretch.rs` (`relax`): each
iteration solves H d = -g on the coordinates that move, H the sum of each triangle's projected
Hessian, the pressure's, a hinge's and the plug's spring's, by a sparse Cholesky. The line search,
the contact stops, the turned-triangle guard, the blow-out rollback to volume control, the capped
step's retakes and the load steps are as they were.

## What `hessian.py` derives and checks

`python3 hessian.py` (SymPy, NumPy; e.g. a nix `python3.withPackages (ps: [ps.sympy ps.numpy
ps.mpmath])`) checks, and the Rust code uses:

- W of C = F^T F through its squared principal stretches s1 >= s2: psi_1, psi_2, psi_11, psi_12,
  psi_22 of the incompressible Ogden law in closed form, and kappa = (psi_1 - psi_2)/(s1 - s2) by a
  divided difference with its series under a thousandth (its limit psi_11 - psi_12). The Hessian
  in (a, b, c) = (C11, C12, C22) is sum psi_ij grad s_i grad s_j^T + 2 kappa w w^T: checked against
  SymPy's exact second derivatives (2e-14) for neo-Hookean, Mooney-Rivlin and a 3-term Ogden law.
- The element's Hessian in F (3x2) splits in the principal frame (F = U diag(l) V^T, n = u1 x u2)
  into a stretch block on (a11, a22), [[4 s1 psi_11 + S1, 4 l1 l2 psi_12], [., 4 s2 psi_22 + S2]],
  a shear block on (a12, a21), [[2 kappa s1 + S2, 2 kappa l1 l2], [., 2 kappa s2 + S1]], and the
  pair out of the plane, S1 and S2 (S_i = 2 psi_i). Each block clamped to no negative eigenvalue
  gives a positive semi-definite Hessian that is the exact one in tension (Teran, Sifakis, Irving,
  Fedkiw 2005, Robust quasistatic finite elements; Smith, de Goes, Kim 2019, Analytic eigensystems
  for isotropic distortion energies). Against central differences: 8e-6, including the
  tension-field (wrinkle) relaxation psi~(s1), psi~'' = psi_11 - psi_12^2 / psi_22, kappa =
  psi_1/(s1 - s2). The 9x9 corner Hessian B^T H_F B: 2e-6.
- The pressure's work p/6 a.(b x c): H_ab = -p/6 [c]x and cyclic, no diagonal blocks (exact).
- A hinge: grad theta is `bend`'s terms without 2 k sin theta (1.5e-10); its Hessian is taken as
  2 k max(cos theta, 0) grad theta grad theta^T (Gauss-Newton).

The Rust tests `newton::tests` repeat the element check against differences and the
positive-semi-definite one on the shipped code.

## The solve

- The pressure's Hessian is indefinite on its own (a flat sheet has no out-of-plane stiffness):
  the factor is tried with it, and without it where the sum is not positive definite (a shift of
  1e-10 of the largest diagonal entry keeps the flat sheet's null directions solvable; the step cap
  then bounds them). Drawn by volume, the penalty's rank-one term goes by Sherman-Morrison.
- Sparse Cholesky: nested dissection of the sheet's reference layout (each half by the wider side's
  middle, the nodes next to the other half last), elimination tree and column counts (Liu; Davis,
  LDL), supernodes, multifrontal dense panels; no third-party crate (bit-identical native and wasm,
  no FMA). At 56 nodes (8.9k coordinates) 61 M multiply-adds a factor, 14 ms.
- A step the cap shortens is far from equilibrium: the next takes the last factor (a chord step,
  the coordinates stopped since left out), up to 8 in a row.

## Measured (Varedi heat, 56 nodes, loaded machine)

L-BFGS as shipped: 14991 iterations, 22 s; every drape step stopped at its 150-iteration cap
870-110 000 times over the tolerance, so its answer was not an equilibrium. L-BFGS with every cap
100x (converged): mean error 8.43. Newton: 216 iterations, 2.7 s, every step converged, mean
error 8.00. The two converged solvers differ by where the sticking contact lands within a load
step (the minimiser's path); refining the drape 6 -> 24 -> 96 steps brings them together:
Newton 8.00 / 8.28 / 8.63, L-BFGS 8.43 / 8.54 / 8.52.
