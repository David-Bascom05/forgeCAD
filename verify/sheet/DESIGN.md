# verify/sheet: the sheet-metal spec in Lean (design note, then status)

## Status (what is built and proven)

`lake build` (no `sorry`; Sheet/Axioms.lean, in the default target, checks every main theorem
rests on propext, Classical.choice and Quot.sound only). CI: .github/workflows/sheet-lean.yml.

| file | content |
|---|---|
| Fold.lean | one bend over ℝ: s = ρ tan(θ/2), `inner_radius` (towardRef), `neutral_radius`, κ ∈ [0,1] ⟺ rn ∈ [R, R+t], φ(BA) = θ, `zone_zero`/`zone_BA`, C¹ (`dir_zero`/`dir_BA`), `iso`, `neutral_length`, `deduction`, `refold_a`/`refold_b`, the hem branch `zone_BA_hem` and `Jat_eq_J` |
| Tree.lean | `refold`: fold ∘ develop = folded on every wall of a strip tree, zones glue on every tangent line, by induction on paths |
| Rules.lean | `BA_from_rule`, `ktable_physical`, `allowance_physical` (per-row κ check suffices), K ↔ BA, `s_eq_setback` (ISSB/OSSB), `flat_length_consistent`, `deduction_is_bd`, `bd90`, `outsideReach_eq` (min flange + t tan(θ/2)) |
| Classes.lean | Snap.lean's classes, generic: `cls_spec`, `rep_order`, `rep_class`, `rep_unique`, `conn_map` |
| Region.lean | the 2D LINEAR rule exact over ℚ, vertex classes, spread cap, half-open crossing, loop = polygon XOR lenses, hole owner |
| BranchBound.lean | `certify_sound`: forge-brake's Clear/Hit/Undecided sound for any work order, from the primitives' contracts |
| Trig.lean | rational intervals; `sc_mem`: proven sin/cos enclosures at a rational |
| Checker.lean | conditions 1, 3/4, 5, 7 three-valued; `kappa_sound`, `setback_sound`, `refoldVertex_sound`, `near_sound` |
| Oracle.lean | `lake exe sheet-oracle`: reads the kernel's dumps (f64 bits, exact) |

Differential tests (committed digests; `SHEET_LEAN=1` runs the oracle and rewrites them):
* interface/forge-lang/tests/sheet_lean_diff.rs: every sheet part in parts/sheet and examples/ (51
  sheets) and 57 seeded random strips (two flanges, random angle 20–160°, sense, R/t, K, gauge):
  condition 7, 3/4, 5, 1 per sheet. **Result: 431 of 432 pass**; the one FAIL is below.
* kernel/forge-sheet/tests/lean_diff_region.rs: 46 regions from the trims' case sets (those with
  no splines), 400 random points each plus points 1e-6/1e-3 off every vertex and 1e-9/1e-6 off
  every piece's middle: region.rs `holds` against `inLoop`. **Result: 0 disagreements.**

## Disagreements found (for the fix-sheet session)

1. **parts/sheet/holes.fg, condition 1 (names)**: the hole wholly inside the bend zone (`zone`, the
   r = 1 circle at (38.2, 50)) has its vertices named differently in the flat and folded bodies.
   Flat vertex `#8e2a7322f8f5ac98` at (39.2, 50, −2) (the circle's +u extreme, bottom skin) is
   named like the folded vertex at (38.829, **49**, −0.828) (the circle's −y extreme); the true
   refold of (39.2, 50) at depth 2, (39.639, 50, 0.340), exists in the folded body under another
   name (`#99949d97d41e1965`). The point sets agree; the name bijection does not (the wrapped
   circle is split at different points in the two maps).
2. hems: none (the hem's s = R + t is the kernel's `fl(R + t)`; the spec compares within tol).

## For the Rust session: the region statement (Region.lean)

* `near(p, q) ⟺ (px−qx)² + (py−qy)² ≤ L²`; line carriers `orient(a,b,c)² ≤ L²·|b−a|²` for the
  four ends (one), within 2L but not L (ambiguous → NearCoincident), else two; zero length:
  degenerate (refuse). Evaluate exactly (expansions; a float filter with a proven error bound
  first). Today: orient2d / float length, float sqrt in `distance`.
* Spread cap: every two points of a class within 2L; the box-diagonal test region.rs uses implies it.
* `inside` without a turned ray: horizontal ray, edge a→b counts iff `(a.y > p.y) ≠ (b.y > p.y)`
  and `(b.y > a.y) = (orient(a,b,p) > 0)`, over the loop's class points (lines end on them);
  each arc adds its lens `|p−c|² < r²` ∧ (`orient(a,b,p) < 0` for a ccw arc, `> 0` cw), XOR.
* `holds_hole` without the 0.3/0.7 vote: a hole goes to the outer loop holding its least-named
  vertex that is innermost (every other outer loop holding the vertex holds that loop's
  representative). One exact query; no areas.
* Splines: exact rational de Casteljau until no piece's hull holds p, then chords; fuel → refuse.

# Design note (as sent before building)

Step 7 of research/kernel-audit/audit2.md §2.10, taken over from the fix-sheet session. Lean
4.36.0 + Mathlib v4.36.0 (the pin of verify/ieee754 and verify/krawczyk), standard axioms only,
no `sorry`, `Axioms.lean` checked in CI (`.github/workflows/sheet-lean.yml`).

Source read: audit2 §2.0–2.5 and fix-sheet's forge-sheet (region.rs rewrite, refold.rs,
shop.rs, build.rs) at b8758a2; forge-brake at d02ba8e; forge-vendor at 4c8bebf; verify/ieee754;
verify/krawczyk at 6b69b4e; verify/forge-exact/ForgeExact/Snap.lean.

## 1. The fold (Sheet/Fold.lean, Sheet/Tree.lean, Sheet/Rules.lean) — over ℝ

Coordinates: wall a's joint frame (`ja`): x across, y along, z the reference-skin normal; mold
line x = 0; depth w ∈ [0,t] below the reference skin at z = −w.

* `Bend := (R θ t BA : ℝ) (sense : pos | neg)` with 0 ≤ R, 0 < θ < π, t > 0, BA > 0.
* `ρ` = R (pos, towardRef) | R + t (neg); `s = ρ sin θ /(1+cos θ)`; `rn = BA/θ`;
  `κ = (rn − R)/t`; `wn` = rn − R (pos) | R + t − rn (neg).
* `zone u y w = (−s + (ρ+σw) sin(u/rn), y, σρ − σ(ρ+σw) cos(u/rn))` (refold.rs `Zone::at`,
  build.rs `Geo::Round`); `J` = the turn by θ about the mold line (wall b's frame `jb` in a's);
  `develop x' = x' + (BA − 2s)` (build.rs:1187); `F` the flat→folded map of one bend.

Theorems (done = compiled):
* done `s_eq_tan`: s = ρ tan(θ/2).
* done `inner_radius` (towardRef): every layer at radius ≥ R, the inside skin at exactly R, both senses.
* done `neutral_radius`: ρ + σ·wn = rn, both senses. `wn_mem_iff`, `rn_mem_iff`: wn ∈ [0,t] ⟺
  R ≤ rn ≤ R+t ⟺ κ ∈ [0,1] (item 7).
* done `φ_BA`, `zone_zero`, `zone_BA`: φ(BA) = θ; the zone starts on a's tangent line and ends on
  b's, folded (needs exactly s(1+cos θ) = ρ sin θ and θ < π). `dir_zero`, `dir_BA`: C¹ (item 5).
* done `hasDerivAt_zone_u/y`, `metricE_eq`, `iso`: E = ((ρ+σw)/rn)², F = 0, G = 1, so E = 1 at wn
  (item 2). `neutral_length`: rn·θ = BA (item 3).
* done `deduction`: mold lines BA − 2s apart, flat = La + Lb − BD with BD = 2s − BA, tangent lines
  at u = 0 and u = BA (item 4). `F_glue`, `refold_a`, `refold_b`: one bend refolds.
* `refold` (Tree.lean): strips as paths of bends from the root; fold placement computed from the
  flat's own data (Fold_b = Fold_a ∘ lift ja⁻¹ ∘ J ∘ lift jb), development P_b = P_a ∘ ja⁻¹ ∘
  shift(BA−2s) ∘ jb; given the folded part's joint condition (mold lines coincide, angle θ),
  Fold_i = M_i for every wall, and F agrees on every tangent line — by induction on the path.
* Rules.lean, one theorem per named kernel branch:
  - `BA_from_rule` (K, KTable): BA = θ(R + Kt) ⇒ κ = K, so K ∈ [0,1] ⇒ physical; KTable
    interpolation of K between rows in [0,1] stays in [0,1].
  - `BA_from_table` (Allowance rows, linear in θ at one radius): if every row has
    Rθᵢ ≤ BAᵢ ≤ (R+t)θᵢ then every interpolated BA is physical (BA − Rθ is affine in θ). So
    item 7 is decided once per row, at table load: a row with κ ∉ [0,1] is refused there.
  - `θ < π` (and the hem's θ < 2π with s = R + t): exactly where 1 + cos θ > 0 is used.
  - K factor ↔ BA: κ(θ(R+Kt)) = K and θ(R + κ(BA)·t) = BA.
  - vendor: OSSB = (R+t) tan(θ/2), ISSB = R tan(θ/2), BD = 2·OSSB − BA; the flat length is the
    same from outside dimensions with BD and from inside ones with 2·ISSB − BA (OSSB − ISSB =
    t tan(θ/2)); BD90 = 2(R+t) − BA90 and K = (BA90/(π/2) − R)/t (forge-vendor lib.rs:1025-1050);
    min flange to the outside: reach + t tan(θ/2) when the reference skin is the inside
    (checks.rs:56-100), + 0 otherwise.

## 2. Flat-pattern regions (Sheet/Classes.lean, Sheet/Region.lean) — over ℚ (doubles are dyadic)

* Classes.lean: Snap.lean's union–find classes made generic (any type, a symmetric decided
  relation, names): `cls_spec`, `rep_order`, `rep_unique`, `spreadOk`, `gap_carried`. (forge-exact is
  core Lean on 4.36.0-rc2, so it cannot be a lake dependency; the proofs are ported, not imported.)
* The one LINEAR rule, exact: points p ~ q ⟺ |p − q|² ≤ L²; lines on one carrier ⟺
  orient2d(a,b,c)² ≤ L²|ab|² for the four ends; band L < gap ≤ 2L refused (`NearCoincident`);
  spread cap: class box dx² + dy² ≤ (2L)². Rust today divides orient2d by a float length and takes
  float sqrt: same rule, not exact at the boundary.
* Exact point-in-region, one rule replacing the 0.0137-rad ray and the 0.3/0.7 hole vote:
  a loop's region is its polygon over the class points XOR, for each arc, its lens
  D(c,r) ∩ H(chord, arc side) — both decided by exact signs: orient2d and |p−c|² − r².
  Polygon membership by the horizontal half-open crossing rule over ℚ (no turned ray: the
  half-open rule decides vertices on the ray level exactly). Splines: subdivided exactly
  (rational de Casteljau) until no piece's control hull holds p, then their chords; refused on fuel.
* Hole assignment: after `chain` (no pinches, no shared vertices: loops pairwise disjoint), hole
  H goes to the least-area positive loop O holding H's least-named vertex — one exact query, not a
  vote. Theorem: for pairwise-disjoint loops the answer does not depend on which vertex of H is
  asked (stated for the polygon part).
* Statement for the Rust session: in Region.lean's header, as the exact predicates and the
  order of decisions.

## 3. The refold checker as executable spec (Sheet/Checker.lean) — exact ℚ + proven enclosures

Inputs: the exported pattern (doubles as exact dyadics), the bend table, the folded vertices.
sin/cos enclosed over ℚ by Taylor polynomials with Mathlib's remainder (`Real.sin_bound`,
`Real.cos_bound`) after exact range reduction with a rational π enclosure; no float anywhere.
Conditions 1–8 as Lean `Bool` definitions over these enclosures; each comparison three-valued
(pass: proven within tol; fail: proven past tol; undecided → typed refusal at the locus).
Theorem `check_sound`: pass ⇒ the real-number condition holds. Condition 6: flat overlap by the
exact region rule of §2; folded clash by the certified branch and bound of §4, one rule: clash iff
a certified point penetrates > LINEAR, clear iff certified depth ≤ LINEAR everywhere, otherwise
undecided (refused): exhaustive and exclusive by construction.

## 4. Bend simulation soundness (Sheet/BranchBound.lean)

Abstract over a cell type with a cover relation, a true quantity d(p) ≥ 0-clearance, sound
`enclose` (∀p∈cell, d p ≥ lower(cell)) and sound point bounds (d(q) ≤ upper(q)); interval
primitives only by their contracts (the corrected `Interval::sin`'s, from the trace session).
Theorems: `Clear` ⇒ ∀ p in the roots' cover, d p ≥ limit − res; `Hit` ⇒ a real point with
d < limit; budget exhausted ⇒ `Undecided` unless already proven.

Found in forge-brake (d02ba8e), for the bendsim session:
* bb.rs `search`: when the heap empties it returns `Clear{lower: min(lower, upper)}` without
  checking lower ≥ limit − res; a leaf (hd ≤ res/2) with lb < limit and no witness below limit is
  absorbed into `lower` — so `Clear` can be returned below the limit. The model forces
  Undecided there.
* every decision compare is plain f64 (lb ≥ limit, d < limit, lower ≥ limit − res); lb is a float
  sd at a float midpoint minus a float sqrt plus SLACK = 1e-9; `Mul<f64>`/`Add<f64>` take the f64
  constant as exact; `grow` is not outward-rounded; `fold` multiplies by a rounded 1/BA;
  TRIG_SLACK and SINC_SLACK are absolute 1e-15 claims, not proven.
* the witness is `region.rep(mid)`, not necessarily in the cell (fine for Hit: any real point).

## 5. Matching (Sheet/Oracle.lean, `lake exe sheet-oracle`)

kernel/forge-sheet/tests/lean_diff.rs dumps, for every sheet part in the repo (examples/, parts/)
and seeded random strips: per bend (R, θ, t, BA, sense, s, rn, ρ) and folded/flat vertex pairs,
and per region query (points, loops) the Rust decision; the oracle recomputes each in ℚ and prints
agree/disagree; committed digest `tests/lean_diff.digest`; `SHEET_LEAN=1` runs it. Every
disagreement is reported to the fix-sheet session, not patched here.

## Other findings

* forge-vendor `Vendor::shop`: KTable rows come in the file's radius order with the default
  radius first; `interpolate` refuses `TableOrder` unless r/t strictly increases, so a vendor
  file whose default radius is not the least fails. Sort the rows (and refuse duplicates) at load.
* shop.rs `Allowance`: nothing refuses a row with κ ∉ [0,1] (the refold checker reports it after
  the build); Rules.lean makes the per-row check sufficient.
* build.rs `phi`: `if u == ba { θ } else { u/rn }` is an exact special case the spec covers
  (φ_BA), fine.
