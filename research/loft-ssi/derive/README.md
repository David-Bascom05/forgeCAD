# Loft x loft: the exact section-pair formula, proved (lean-derive)

Offline research. Nothing in the kernel uses these files: the shipped method is the Bezier-hull test in
`crates/forge-geom/src/loftmeet.rs` (`../novel/`; proved separately on branch lean-loftmeet,
`../shipped/`). This directory is the exact alternative: where two smooth lofts with arc rails meet,
as one polynomial equation in the two spline parameters, and a Bernstein certificate for disjointness.
`proofs/` builds with no `sorry`, and no axioms beyond Lean's `propext`, `Classical.choice`, `Quot.sound`.

## Rerun

    cd proofs && lake exe cache get && lake build              # Lean 4 v4.36.0-rc2 + Mathlib (elan)
    python3 -m venv venv && venv/bin/pip install sympy numpy
    venv/bin/python general_identity.py   # general F: all elimination forms agree (exact, 30 points)
    venv/bin/python minimal.py            # unequal sweeps give non-conic sections
    venv/bin/python cert.py               # the certificate on volute x core: 616 boxes, all certified
    venv/bin/python cert_count.py         # its rounding counts against Rounding.lean's bound

To list the axioms: a file with `import Proofs` and `#print axioms LoftSSI.meet_iff` (etc.), run with
`lake env lean`.

## The surface

One face of a `Skinned` loft is one span of a cubic spline (natural, or clamped when the loft has end
slopes) across the sections. When every rail is an arc with the same sweep (any start angles), the
blend is one ellipse per spline parameter (`arc_family`):

    S(s, phi) = c(s) + cos(phi) X(s) + sin(phi) Y(s),   c, X, Y cubic in s on the span.

Volute class: every section frame has y = -e, x and the centre in one plane square to e, starts 0 or
pi. Then S = P(s) + cos(phi) A(s) - sin(phi) rho(s) e with P, A in that plane and rho scalar.

## What is proved (namespace LoftSSI)

Everything about sections is pointwise in the section data, so it holds for any spline, any knots.

General arcs, `Proofs/General.lean`. Columns A1 = X1, A2 = Y1, A3 = -X2, A4 = -Y2, A5 = c1 - c2;
p_ij = det of the three columns other than i, j (Pluecker coordinates of the solution pencil);
with d = (1, 1, l, l, -(1 + l)): E(l) = sum_{i<j} d_i d_j p_ij^2 = E0 + E1 l + E2 l^2 and

    E0 = p12^2 - p15^2 - p25^2
    E1 = p13^2 + p14^2 + p23^2 + p24^2 - p15^2 - p25^2 - p35^2 - p45^2
    E2 = p34^2 - p35^2 - p45^2
    F  = E1^2 - 4 E0 E2

`Sec.Meet e1 e2`: some alpha^2 + beta^2 = 1, gamma^2 + delta^2 = 1 with c1 + alpha X1 + beta Y1 =
c2 + gamma X2 + delta Y2. `General`: (p15, p25, p35, p45) != 0, i.e. X1, Y1, X2, Y2 span R^3.

- `meet_imp` (no hypothesis): Meet => F = 0, E0 <= 0, E2 <= 0.
- `meet_iff` (General): Meet <=> F = 0, E0 <= 0, E2 <= 0.
- `meet_iff_of_ne` (General, X2 x Y2 != 0): Meet <=> F = 0, E0 <= 0.
- The elimination identity, `Cols.pencil`: for any solution w of the linear system and the pencil
  direction k, the two circle conditions restricted to span(w, k) are binary quadratics with
  resultant w5^4 F and discriminants -w5^2 E0, -w5^2 E2 (`pencil_alg`, Pluecker relations
  `Cols.plucker`, binary-form lemmas `res_of_common`, `common_of_res`, `disc_of_zero`).
- `GeneralDegree.lean`: `deg_s1`, `deg_s2` (one section's c, X, Y of degree <= d in its parameter,
  the other fixed): F is a polynomial of degree <= 8d, so (24, 24) for cubic spans. Tight:
  `witness_s1`, `witness_s2` (a circle of radius s^3 at height s^3 against the unit circle in
  x = 1) give F = 4 s^24 - 8 s^18 + 4 s^12, natDegree exactly 24.
- `volute_F`: in the volute class F_general = F_volute^2, E0 = -rho2^2 (W^2 - a2^2),
  E2 = -rho1^2 (W^2 - a1^2).
- Checked in SymPy only (`general_identity.py`, exact rationals, 30 random points, Schwartz-Zippel):
  F equals novel-math's pencil form Res_j / q_j^4 for every column j, the line form
  Res_t(q1, q2) / (|n1|^4 |n2|^4 |l|^8) and the plane form. The Lean theorems do not depend on it.

Volute class, `Proofs/Section.lean`, `Degree.lean`. W = A1 ^ A2, D = P2 - P1, a1 = D ^ A1,
a2 = D ^ A2, F_volute = rho1^2 (W^2 - a2^2) - rho2^2 (W^2 - a1^2).

- `on_section_iff` (|A| != 0, rho != 0): (h, z) on the section ellipse <=> g = 0 and Psi = 0.
  `on_section_mp`: => with no hypothesis.
- `sections_meet_iff` (|A1| != 0, rho1 != 0): R2's reduction to (s1, s2, c2).
  `sections_meet_mp`: => with no hypothesis (all the certificate uses).
- `pair_iff` (W != 0, rho2 != 0): meet <=> F_volute = 0 and a2^2 <= W^2; lift c1 = a2/W,
  c2 = a1/W, sin phi2 = sin phi1 rho1/rho2. (The old rho1 != 0 was never used.)
- `pair_iff_sym` (W != 0 only): meet <=> F_volute = 0, a1^2 <= W^2, a2^2 <= W^2.
- `degrees_s1`, `degrees_s2` (cubic data): g <= 6, Psi <= 18, F <= 12 in s1; g <= 3, Psi <= 6,
  F <= 12 in s2. `volute_deg` (degree <= d data): F <= 4d. `volute_witness`: 12 is reached.

Bernstein and de Casteljau, `Proofs/Bernstein.lean`. Shared with the loftmeet proof: byte-identical
to `research/loft-ssi/shipped/LoftMeet/Bernstein.lean` on lean-loftmeet (be8914b); edit it there.
`bb_sum`, `bb_nonneg`, `bern_ge`, `bern_le`, `bern3_pos`, `rounded_pos`, `bb_half`, `bb_symm`,
`bb_split_left` (B^n_k(r t) = sum_j B^n_j(t) B^j_k(r)), `bb_split_right` (k <= n:
B^n_k(r + (1-r) t) = sum_j B^n_j(t) B^(n-j)_(n-k)(1-r)), `bb_half_right`, `sum_split_left/right`,
`bern_split_left/right`, `bern3_split_x/y/z` (both sides, any r), `bb_mul`, `bern2`, `bern2_ge`.
`Minimal.lean`: `bern3_pos_corners`. `Rounding.lean`: `bb_elevate`.

Floating point, `Proofs/Rounding.lean`. Model: a rounding function with |rnd x - x| <= u |x| for every
rounded add, sub, mul and division by an exact constant (f64 round-to-nearest with u = 2^-53, provided
no product or quotient underflows below 2^-1022 and nothing overflows; underflowing sums are exact).
Expression trees `Ex` (exact inputs, approximate constants, add, sub, mul, division by a constant,
exact negation and halving) with exact value, magnitude (the same tree on |.|) and an op count c.

- `Ex.err_le`: |computed - exact| <= gamma_c * magnitude, gamma_c = c u / (1 - c u) (c u < 1).
- `Ex.mag_comp_ge`: the magnitude computed in f64 is >= (1 - gamma_c) * the exact one.
- `Ex.clear_pos_of_le`, `clear_neg_of_le`, `bern3_clear_pos/neg`: a coefficient tensor whose computed
  entries all exceed K u * (a bound on the computed magnitudes) in one sign is exactly of that sign,
  and the polynomial has no zero on the box, provided K u >= c u / (1 - 2 c u), i.e. c <= 255 for K = 256.
- `dc`, `val_dc_left/right`, `bern_dc_left/right`, `cnt_dc`: de Casteljau halving in f64 computes
  the halves of `bern_split_left/right` and adds n to the count per halving on an axis of degree n.
- `sum_prod_err`: a left-to-right sum of n products of m inputs has error <= gamma_(m-1+n-1) * sum|terms|.

## The certificate (BOARD R2, R3; `cert.py`), and how far the proof covers it

A box is cleared when g's or Psi's Bernstein coefficients all exceed K u max(mag) in one sign.
Proved chain: cleared => exact coefficients of that sign (`bern3_clear_pos`, count <= 255) => g or
Psi has no zero on the box (`bern3_pos`) => no meet in that box (`sections_meet_mp`, no hypotheses).
`cert_count.py` re-evaluates cert.py under the `Ex` rules and reproduces its values bit for bit:
depth-0 counts are 35 (g) and 158 (Psi); each halving adds 6/3/1 (g) or 18/6/2 (Psi). On volute x core
the deepest box is depth 8 and the largest count among clearances 234, so every clearance in that run
is covered by K = 256. In general K = 256 covers Psi only to depth 10 (count 260 at depth 11) while
cert.py allows depth 36: a deeper run needs K >= c / (1 - 2 c u) for the box's count (about
158 + 26 depth/3 for Psi).

Assumed, not formalised: (1) that cert.py's tensor `bmul`, `elevate` and `add` produce the Bernstein
coefficients of g and Psi over the box (the 1D identities they use are proved: `bb_mul`, `bb_elevate`,
`bb_sum` for D_ij = P2_j - P1_i, and 1 - c2^2 = 2 B^2_1(u); the 3D index bookkeeping is checked only
numerically); (2) the rounding model above; (3) the certified surface is the one whose span
coefficients are cert.py's f64 P, A, rho (`span_data`); the rounding of the spline weights that
produce them is outside the count.

## Assumptions: which are needed

    hypothesis                               result                 status (evidence)
    -------------------------------------    -------------------    ---------------------------------------------
    frames y = -e (one common axis)          volute formula         weakened: any frames (meet_iff, F of bidegree
                                                                    (24, 24)); volute F = sqrt of general F (volute_F)
    centres on one plane square to e         volute formula         needed for F_volute (need_centres); weakened: meet_iff
    circles split at the same angles         ellipse family         weakened to one common sweep, any starts (arc_family)
    equal sweeps                             ellipse family         needed: section not a conic otherwise (minimal.py)
    natural vs clamped spline ends           all                    not needed (pointwise; any degree-d data, deg_s1)
    cubic data                               degree bounds          weakened: degree d gives 8d / 4d (deg_s1, volute_deg)
    rho > 0                                  pair_iff               not needed (any sign; rho1 never used)
    rho1 != 0                                pair_iff               not needed
    rho2 != 0                                pair_iff               needed with one side condition (need_r2);
                                                                    not needed with both (pair_iff_sym)
    W != 0                                   pair_iff, _sym         needed (need_W: coplanar parallel sections)
    a2^2 <= W^2                              pair_iff               needed (need_side: F = 0, cos phi1 = 2)
    |A| != 0, rho != 0                       on_section_iff <=      needed (need_A, need_rho); => needs neither
    |A1| != 0, rho1 != 0                     sections_meet_iff <=   needed (same examples); => needs neither
    general position (X1,Y1,X2,Y2 span R^3)  meet_iff <=            needed (need_general: coplanar circles, all p = 0);
                                                                    => needs nothing (meet_imp)
    E0 <= 0 and E2 <= 0                      meet_iff               both needed in general (need_E2); E2 dropped
                                                                    when X2 x Y2 != 0 (meet_iff_of_ne)
    X1 x Y1 != 0, X2 x Y2 != 0               meet_iff               not needed (only general position)
    all coefficients >= m > 0                bern3_pos              weakened: all >= 0, 8 corners > 0 (bern3_pos_corners)
    count c <= 255 (K = 256)                 clearing rule          needed for the proof; holds on volute x core (<= 234),
                                                                    fails past depth 10 for Psi (cert_count.py)
    no underflow / overflow                  Rounding.lean          needed by the model (relative error fails in subnormals)

## Not proved

- That F equals the line and pencil forms of BOARD R4 (SymPy, exact at random points); not needed by
  `meet_iff`, which is stated for the Pluecker F.
- The certificate's 3D tensor bookkeeping (assumption 1 above) and the spline-weight rounding
  (assumption 3).
- The non-conic section for unequal sweeps is a SymPy determinant, not Lean.

## Files

    proofs/                 Lean project (lakefile.toml, lean-toolchain, lake-manifest.json, Proofs/*.lean)
    cert.py                 reference implementation of the certificate (Bernstein-native, magnitude bounds)
    cert_count.py           its operation counts under Rounding.lean's model
    volute.py               volute and core sections (tools/bench-parts/volute.fc), spline, F per span pair
    weights.py              SymPy: the spline weights' Bernstein coefficients
    fol.py, robust.py       power-basis prototype of R2 (box counts with and without a margin)
    subdiv.py, where.py     F (R1) subdivision on the volute: does not terminate on diagonal pairs, and where
    check.py                lifts F's zeros to 3D (none are real on the volute faces)
    general_probe.py        general 3D ellipse families: degree 24 factor, extraneous factors
    general_sym.py          the symmetric line form's extraneous factors
    general_identity.py     all elimination forms of the general F agree (exact rationals)
    minimal.py              unequal sweeps: the section is not a conic
