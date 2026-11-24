# verify/forge-cam: CAM's L1

This folder proves the obligations G1–G6 of research/cam/design.md §5.2 in Lean 4.36.0 with Mathlib
v4.36.0, over ℝ. The executable parts run over ℚ or ℤ. There is no `sorry`, no `native_decide`,
and only the standard axioms (`lake env lean Axioms.lean`). CI: .github/workflows/forge-cam.yml.

```
lake exe cache get && lake build && lake env lean Axioms.lean
```

| file | what it proves |
|---|---|
| ForgeCam/SignedDist.lean | `sd`, the signed distance; `sd_translate`; `sd_le_sd_add_dist` (sd is 1-Lipschitz, for the lowering's μ). **`clear_of_frontier`**: take a connected body X and a closed set A. If X is at signed distance ≥ c > 0 from every frontier point of A, and some point of X is outside A, then every point of X is ≥ c from A. |
| ForgeCam/Hypograph.lean | **`sd_hyp_mono`, the lemma the 2.5D checker rests on**: for a body {z ∈ Z, ρ ≤ w(z)}, the signed distance is non-decreasing in ρ at fixed z. `sd_hyp_eq_sdRZ`: it depends only on (ρ, z). `sdRZ_le_sd`: a lower bound D on ρ gives sd ≥ sdRZ(D, z). Z and w are arbitrary. |
| ForgeCam/Spec.lean | `near_section_cthickening` and `near_section_sqrt`: G4's three-dimensional near-face rule, a section at z′ grown by √(g² − dz²), stays within g of P. The definitions of G1 (with ε_in), G2 (corrected, §7b, open form: T ∪ N stays out of the open c-neighbourhood of F on every piece; N misses int S); G2closed and G3. Declared removals: `target P Δ = P ∖ Δ`, `G1_target_iff`. `G4_held`: the leftover split exactly into "held by fixture" (inside the clamped region K) and "uncut". G4: `G4_lower` (P's (δ − ε)-thickening survives, from G1), `G4_part`, `G4_upper` (the leftover L = S_end ∖ cthickening covers the excess, is disjoint from the grown part, and is the least such set) and `leftover_slice` (per z band). |
| ForgeCam/Prism.lean | **The closed-form checker's soundness**: prism faces {σ ∈ S, h ∈ H}; planar or vertical moves (tip at (γ(s), ζ(s)), ζ(s) ∈ Zt; γ any curve). `clearFace_sound` covers one face. `Certified` packages what the checker establishes for one body against one closed set: its top is above the set, and each face lies within μ of a lowered face answered with lower ≥ c + μ. `certified_clear` turns that into clearance, and `hyp_preconnected` shows the tool bodies are connected. `G1_of_closedForm`, `G2closed_of_closedForm` (the closed form of G2, with a margin c' > c) and `G3_of_closedForm` give the obligations. |
| ForgeCam/Vise.lean | The vise as fixture.rs builds it (3e8a91d), a union of boxes. `frontier_box_subset` and `frontier_fixture_faces`: its frontier lies in its boxes' six prism faces each. `certified_vise`: the per-face verdicts make it `Certified`. `G2open_of_closedForm`: certified at exactly the clearance c, as forge-cam does it, the whole tool misses the open c-neighbourhood of the fixture; G2 is this open form (the master's decision); the closed F ⊕ B_c needs c' > c (`G2closed_of_closedForm`). `held_band`, `uncut_band`, `band_split`: G4's bands against the jaws' top. |
| ForgeCam/Fixture.lean | Soft jaws as forge-cam cuts them (5b32f0a): `softJaw = blank ∖ (M × [seat, top])`. `softJaw_seat`: if the held body is a stack of prisms whose sections each lie in M (forge-cam's exact seating check), and it meets the blank only within the grip heights, it misses the jaws. `softJaw_needs_grip` shows that last condition is needed. The earlier model: `jaw = blank ∖ ((H ⊕ B_k) ∩ slab)`. `jaw_seat` proves the held body misses the jaw, for any k, provided H meets the blank only inside the grip slab. `jaw_seat_needs_slab` shows that proviso is needed. |
| ForgeCam/Pocket.lean | **Pocket coverage** (`passes_cover`): offset passes at depths r + k·s from the original boundary, with 0 < s ≤ r, put every point of the opening R ∘ B_r within r of a pass. This is the void stock.rs assumes. |
| ForgeCam/Dist2.lean | **slab::dist's segment–segment candidates, exact** (over any ordered field, so they run over ℚ). `ptSegSq_le` / `ptSegSq_attained`: the end-or-perpendicular candidate is the least squared distance from a point to a segment. `segSegLo_le`: the low end (the four end-to-segment candidates if apart, else 0) is a lower bound for every pair of points. `segSegLo_attained`: when the segments are apart, it is attained. `clearFace_segSeg`: for a segment move against a wall over a segment, hypothesis (i) of `ClearFace` is proven, not assumed. `segSegLo_le_all` and `clearFace_segSeg_all` include degenerate segments (points: plunges and drill moves project to them). |
| ForgeCam/DistArc.lean | **slab::dist's candidates with arcs, sound over ℝ.**
- `dot_le_max_of_not_inSector`, the cone lemma: for a direction outside the sector, the nearer arc end is nearest. Proven by algebra, with no angles.
- `ptArcSq_le`: the point–arc candidate is a lower bound.
- `cross_eq_zero_of_arc_min`: first order on an arc.
- `segArcLo_le`: segment–arc (the end candidates, the centre's foot, the crossing).
- `arcArcLo_le`: arc–arc (the end candidates, the four line-of-centres candidates, the crossing, and |r1 − r2| for concentric arcs).
- `clearFace_segArc` and `arcArc_D` discharge `ClearFace` (i). |
| ForgeCam/Trim.lean | **The point-in-trim test and `dist_region`** (CAM 787239e). The trim's pieces: lines, and per arc its circle part plus two radial end pieces.
- `lineDec_sound`, `radial_misses`, `arcDec_sound`: each piece's decided `ray_side` count is the true number of meetings. Radial pieces are never met outside the gap, and decided rays meet no piece end and are tangent to no arc.
- `raySide_sound`, `raySide_classifies`, `pointSide_sound`: under the even-odd rule (`EvenOdd`, the region's meaning, assumed), a decided `classify` is exact.
- `distRegion_lower`: with the start classified outside and the frontier in the pieces, a piece bound is a region bound.
- `arcPiece_lower`: an arc's circle-part bound less the gap bounds its radial pieces.
- `clearFace_region` closes ClearFace (i) for flat faces.
- The stored ends (CAM 3a388e5): `segArcLo_shift` and `arcArcLo_shift` show that moving an arc's ends along their rays by at most g raises √lo by at most g. So `segArc_trim` and `arcArc_trim`: slab's max(0, √lo − 2g per arc) bounds the distance to the trim piece, radial pieces included. |
| ForgeCam/Shadow.lean | **The closed forms by symmetry** (shadow.rs, CAM 007ce37), S1–S4.
- `shadow_point`: a horizontal-distance bound into `sdRZ`.
- Plane: `frame_orthonormal`, `frame_gap` (the gap across and along), `plane_coord`, `line_coord`, `arc_coord`.
- Cone: `cone_factor` (the |c x + s y|² enclosure), `cone_radius`, `axis_gap`, `line_far`, `arc_far`, and `arc_near` (point–arc less the gap).
- Search: `shadow_cover`, `shadow_face`.
- `certified_clear_gen` carries a per-face bound of any face kind into the clearance. |
| ForgeCam/Halfspace.lean | `sd_ge_of_halfspace`: a tool lying in {D ≤ ⟪N, x⟫ − c} (N unit) has signed distance ≥ D from every point of the plane ⟪N, x⟫ = c, D of either sign: shadow.rs's `halfspace_bound` for plane faces, per piece of the move. |
| ForgeCam/Heights.lean | `apart_of_heights`, `setDist_of_heights`: a body at least c above a height T is at least c from anything at or under T; why forge-cam-lang skips building the staged stock when every checked body stays the clearance above the stock's top. |
| ForgeCam/PathEnds.lean | H.pathends, the metric core of `Move::path_gap`: `proj_near` (the stored end's ray meets the circle within 2ε of the true end), `chord_mono` (a circle point in the cone between the true end and that point is no farther from it). The cone decomposition of the whole arc is argued in the code (hyp). |
| ForgeCam/Machine.lean | G5 for 3-axis. `G5_line_iff`: a line move is within travel iff its ends are. `G5_arc`: an arc or helix is within travel if its circle's box is. `lineOkQ_iff`: the ℚ Bool check equals the ℝ spec. `G5rt_of_check`: the runtime check (posted F words against an outward-rounded lower end of max_feed · 10^k, S words in the rpm range) gives G5's runtime half. |
| ForgeCam/Post.lean | G6 for forge-post's motion subset. The printer is modal: G only on a mode change, an axis only when it changes, F only when the feed changes, and a null move is dropped. **`round_trip`**: `read (print p) = normalize (motions p)`. `G6` adds quantisation, with error ≤ ½·10^−d (`roundEven_isQuant`, ties to even). **`faulty_counterexample`**: the printer as of 155e2f2 reads a plunge back as a rapid. Arc semantics: `arcFull_agrees` (the meant `full` equals the control's reading on every posted arc, as forge-post 4bfe2a2 compares), `arc2_safe` (with X, Y compared, a posted arc read as a full circle was planned as a full turn or more than a half turn) and `arc3_unsafe` (with all three compared, as in 0a4c189, a short helical arc is read as a full helical turn). |

## The ℚ oracle (L6)

`lake env lean --run Oracle.lean < cases` answers `segseg`, `ptseg` and `apart` lines, exactly,
with the definitions proven in Dist2.lean. Numbers are written `n` or `n/d`, and an f64 as
m/2^k. Oracle.lean gives the format; tests/oracle.in and tests/oracle.out are checked in CI.
It runs interpreted: a native executable would compile all of Mathlib.

## What the checker's verdict means here

For one face, slab.rs answers `Clear { lower }` when two things hold:
- 0 ≤ D ≤ |σ − γ(s)| for every instant and section point, where D is the low end of
  `slab::dist`'s interval;
- lower ≤ sdRZ(D, h − z_t) for every height h of the face, which the Δ branch and bound gives.

`ClearFace` is exactly these two facts. Only D's low end matters, because of the monotonicity.

## Hypotheses, stated

- **The tool's body** is `hyp Z w` with Z an interval and w ≥ 0 on Z. tool.rs's rising side gives
  this.
- **The frontier** of the part, stock or fixture lies within μ of the checked (lowered) faces,
  and each verdict holds with lower ≥ c + μ (design.md §5.2's mu).
- **`TopAbove`**: the tool has an axis point whose height lies above the set, at every instant.
  Without it, a body wholly inside the set touches no face, and the face checks see nothing.
  ops::certify checks it on every move (`containment`, 01f0a30).
- **G2 and G3** are checked against the stock S₀ as it stood before the piece (S(s) ⊆ S₀),
  which is sound.

## Not covered yet

- ClearFace (i) is now proven for every face kind: segment–segment (Dist2); segment–arc and
  arc–arc (DistArc); flat regions (Trim). There are two stated conditions.
  - The even-odd rule (`EvenOdd`): the trim region's meaning, a Jordan-curve fact.
  - Arc ends: none needed beyond Piece.ok. Stored ends off the circle are covered by slab's
    lo − 2·gap (3a388e5), proven in `segArc_trim` and `arcArc_trim`.
- The arc forms have no ℚ executable: they need ℚ(√d).
- The interval arithmetic (L5) and the B&B checker for non-prism faces.
- The character level of G-code text: words to digits, and the dialect's templates.
- The Aeneas translation of forge-cam-geom and forge-post (L4).
