# forge-mass: what `forge_topo::mass` computes exactly, and how far its error bounds hold

`mass_rel` (kernel/forge-topo/src/mass.rs, at 8250ebb) computes each volume moment as a flux
through the boundary. It turns each face's flux into a loop integral of `G(a, b) = ∫_{a0}^{a} F db`
(Green). It computes G along the inner parameter by a rule picked per surface and axis
(mass.rs:2416–2436). This directory proves each step of that reduction exact over the reals, for
planes, cylinders, cones, spheres and tori. It proves the error propagation into the centroid and
inertia, and checks the kernel against both.

## The proofs (`lean/`, Lean 4.36 + Mathlib, no `sorry`; `Axioms.lean` fails the build unless every cited theorem uses only `propext`, `Classical.choice`, `Quot.sound`)

| file | what is proven |
|---|---|
| `Divergence.lean` | `div_phi`: each of `integrand`'s ten fields (mass.rs:224–239) has divergence 1, a, b, c, a², b², c², ab, bc, ca: the volume, first and second moments. |
| `Surfaces.lean` | Each surface's `deriv` (surface.rs 485–743) over an orthonormal frame. The normals `du × dv` in closed form (`cylinder_n` … `torus_n`). The area integrands: `|n|` constant on planes and cylinders, ρ√(1+k²) on a cone where ρ ≥ 0, r(R + r cos v) on a ring torus. The signed forms `n·(p−o)/r = r² cos v` (sphere, `sphere_signed`, equal to `|n|` between the poles) and `−n·dvv/r = rρ` (spindle torus, `torus_signed`). |
| `Trig.lean`, `Degrees.lean`, `Table.lean` | Trigonometric polynomials (span of e^{iqx}) and polynomials of bounded degree, closed under products. For every row of the rule table, every moment of F is in the class its rule needs, along the inner parameter and along the other: plane poly 3 (GL2); cylinder along v poly 3 (GL2); cone along v poly 4 (GL3); cylinder round u trig 4 × poly 3; cone round u trig 4 × poly 4; sphere and torus trig 4 × trig 5 either way. `mom_flux` ties each moment to its divergence field. |
| `Gauss.lean` | `gl2_exact`, `gl3_exact`: GL2 integrates degree ≤ 3, GL3 degree ≤ 5. `gl2_G`, `gl3_G`: `Inner::Poly`'s `(x/2) Σ w F(a0 + x(1+t)/2)` (mass.rs:2015–2022) is ∫_{a0}^{a0+x} F. |
| `Fourier.lean` | `sum_exp`: Σⱼ e^{ipθⱼ} = N·[p = 0] for \|p\| < N. `dft_exact`: `dft`'s coefficients (mass.rs:1821–1834), on 2k+1 samples, reproduce every trigonometric polynomial of degree ≤ k. `fourier_G`: the closed form `Inner::Fourier` adds up (mass.rs:2066–2073) is ∫_{a0}^{a0+x} F. `Along::Trig` is the same fit. |
| `Errors.lean` | `centroid_bound`: the rigorous centroid error, (√(e₁²+e₂²+e₃²) + \|g\| e₀)/(\|t₀\| − e₀). `centroid_err_understates`: `err[2]` is not a bound (below). `second_moment_bound`: the exact error of each S_ij. Its first-order part is what `err[3]` covers; the factor 2 covers I_ii = S_jj + S_kk. |
| `Rounding.lean` | `sum_err`: under the standard model (verify/ieee754's `F64.add_rel_err`, as a hypothesis), a running sum of n terms is within ((1+u)ⁿ − 1) Σ\|xᵢ\|. |

Assumed, not proven:
- The divergence and Green theorems on a face's trimmed domain.
- The barycentric weights `cheb` gives (−1)ᵏ sin θₖ. These are classical, and checked numerically below.
- That a cone face does not cross its apex, and a torus face keeps to ρ ≥ 0. The kernel's domains hold these: the cone's stops at the apex (surface.rs:69), and the spindle's at its poles.

The outer loop integrals along the edges are adaptive, with error estimates rather than bounds,
and are not covered here, except that their rules are exact where G is a polynomial or a
trigonometric polynomial along the edge.

## The tests (`cargo test -p forge-mass`)

- `tests/exact.rs`:
  - `exact_up_to_rounding`: boxes, cylinders, frustums, spheres and ring tori in 40 random poses each. Volume, area and principal moments agree with the closed forms to 1.1e-14 relative.
  - `reported_error_covers_the_difference`: `err` holds the difference from the closed form, with at least a 1.22× margin.
- `tests/model.rs`:
  - `normals_match_lean`: the kernel's `deriv` gives Surfaces.lean's closed-form normals and signed area integrands, to 1e-13.
  - `chebyshev_weights_interpolate`: `cheb`'s weights reproduce polynomials of degree < n on the 4 and 5 points used.
  - `exact_breps_far_away`: boxes held exactly, up to 2³⁰ from the origin. `mass` is within `err` (worst 0.11 of it): taking moments about the vertex centroid removes the offset.

## Found

**`err[2]` (centroid, "distance") can understate by √3.** mass.rs:205 takes
`em = max(e₁, e₂, e₃)` as the first moments' error. The centroid's distance needs the Euclidean
norm of (e₁, e₂, e₃), which reaches √3·em when the three are alike: `centroid_err_understates`
gives an instance. This is first order. The rigorous bound also divides by |t₀| − e₀, not |t₀|,
which is second order under mass.rs:181. The fix is
`let em = (e[1] * e[1] + e[2] * e[2] + e[3] * e[3]).sqrt();`, with `vol.abs() - e[0]` in the
denominator at mass.rs:217. `err[3]`'s first-order terms are right per entry.

**The rounding allowance is a rule of thumb.** Each piece adds `1e-15·|contribution|`
(mass.rs:2277, 2635), about 4.5u. `sum_err` is what the model gives for the face sum. On the
tests' bodies the allowance holds, but it is not derived.
