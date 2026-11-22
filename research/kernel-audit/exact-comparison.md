# The kernel against the exact reference, under §6.1 (2026-11-22)

`verify/forge-exact` is the exact flat-face reference. Its Lean proofs cover the regularised
boolean and §6.1's tolerance rule, and the same functions are run as an executable. Its fuzz
crate (`forge-exact-fuzz`) gives every flat-face case of each source to the kernel and to the
reference, and judges each kernel path's answer. The paths are the sequential regen, the n-ary
regen, the last stage moved by ulps, and, for box pairs, one `forge_bool::boolean`.

Every number below is under the **snap rule** (`FORGE_EXACT_RULE=snap`): §6.1 applied,
ambiguous only past the 2·LINEAR spread cap. The first comparison used the strict rule, where
any decision within LINEAR is ambiguous.

Rebuild: `lake build` in verify/forge-exact (toolchain from `lean-toolchain`), then
`cargo run --release -p forge-exact-fuzz --bin exact-fuzz -- [SOURCE..] [--seeds N] [--lang N]
[--band AXES:δ,..] [--only NAME] [--shrink]`. `examples/pair_one` and `examples/part_one` show
one case. Repros: `cargo test --release -p forge-exact-fuzz --test repros -- --ignored`.

## What changed before the numbers mean anything

1. **The reference's §6.1 plane test merged every solid's faces.** Two carriers were equal when
   each plane was within LINEAR of the other over `region`, the overlap of the faces' boxes grown
   by LINEAR. Any two faces meeting at an edge, at any angle, pass that test, because their
   `region` is a slab 2·LINEAR across round the edge. So every face of a solid fell into one
   class, its vertices no longer met, and the snap rule answered ambiguous on every case.
   The planes are now compared over `extent`, the hull of both faces' grown boxes. `region` must
   still be non-empty. The change touches ForgeExact/Snap.lean (`Box.hull`, `extent`, `tolEq`,
   `spreadOk`, `gap_carried`) and SnapSym.lean (a `hull` law, proved for every symmetry).
   `lake build` is clean and the standard-axioms check passes. README §6.1 now says the same.
   (The handoff proposed a `Box.wide` guard instead; this needs no width constant and also
   refuses near-parallel faces that tilt apart across their extent. Both give the same totals
   on the original corpus.)
2. **Refusals are judged at the stage they name.** A refusal at feature N is set against the
   reference's answer for the history up to N. It is correct where that stage is empty or
   non-manifold, or a sliver for a ZeroThickness refusal, even if a later feature mends the
   part. It is a refusal bug only where that stage is a valid solid. coincident_walls 1, 42 and
   46 are correct refusals by this rule. In coincident_walls 1, the stage after the Add is
   non-manifold along z = 7, and the next cut removes it.
3. **Volumes get the snap's reach.** The reference decides a merged surface in its
   representative's plane, which can be up to 2·LINEAR from where the kernel keeps it. So
   volumes agree within 2·LINEAR × the body's area, as well as within 1e-7 relative. Without
   this, every band run at δ = 0.5 failed on volume alone.
4. **A new class, kernel unmerged.** The body is valid and its volume right, and its topology
   is the reference's once neighbouring faces that are one plane to within LINEAR are merged.
   It is listed apart from "wrong": the kernel leaves an imprint, and §6.1 says those faces are
   one surface.
5. **The corpus is widened.**
   - Families: 128 seeds of the six families. `more` adds 128 seeds of every other scoreboard
     family, taking its flat sub-history where that combines at least two solids.
   - Language: 1 500 forge_lang::fuzz programs, restricted to flat tools.
   - `boxes`: the coverage worker's box-box family, 13³ Allen patterns × 3 ops.
   - `band`: every box pattern with coincident ends, B's coincident ends moved outward by
     δ·LINEAR. δ ∈ (LINEAR/√2, LINEAR] on three axes and on two, ±; δ ∈ (LINEAR, 4.88·LINEAR)
     on x alone and on z alone; δ = 0.5 (three axes) and δ = 5 (x) as controls.

   The reference gets 120 s a question (`FORGE_EXACT_TIMEOUT`) and is then restarted. Three
   language programs timed out, lang/925 among them, and are counted out of scope.

## Totals

"Before" is this branch's kernel. "After" is a scratch copy of it with the three fix branches
merged: fix-nary bea6c8a, fix-touch da13034, fix-slit f8d8c89. That merge is not pushed. Counts
are per judged path: a part counts once for its sequential path and once for its n-ary path,
and the notched part once for each of its 400 ulp seeds.

### Before the fixes

| source | agree | unmerged | wrong | refused | ambiguous | out of scope |
|---|--:|--:|--:|--:|--:|--:|
| corpus (24 seeds) | 513 | 12 | 4 | 1 | 0 | 143 |
| notched ulp | 67 | 0 | 32 | 301 | 0 | 0 |
| families ×128 | 1460 | 64 | 4 | 8 | 0 | 0 |
| more families ×128 | 1524 | 0 | 12 | 0 | 0 | 187 |
| lang ×1500 | 721 | 0 | 0 | 1 | 0 | 1171 |
| boxes | 5139 | 1444 | 8 | 0 | 0 | 0 |
| band, 3 axes (x y z) | 22140 | 3427 | 374 | 15660 | 0 | 0 |
| band, 2 axes (x y) | 12304 | 3192 | 68 | 5184 | 0 | 0 |
| band, 1 axis (z) | 9752 | 1656 | 1460 | 1328 | 0 | 0 |
| band, 1 axis (x) | 20690 | 4424 | 1717 | 1561 | 0 | 0 |
| **all** | **74310** | **14219** | **3679** | **24044** | **0** | **1501** |

### After the fixes

| source | agree | unmerged | wrong | refused | ambiguous | out of scope |
|---|--:|--:|--:|--:|--:|--:|
| corpus (24 seeds) | 518 | 12 | 0 | 0 | 0 | 143 |
| notched ulp | 67 | 0 | 32 | 301 | 0 | 0 |
| families ×128 | 1460 | 64 | 4 | 8 | 0 | 0 |
| more families ×128 | 1536 | 0 | 0 | 0 | 0 | 187 |
| lang ×1500 | 722 | 0 | 0 | 0 | 0 | 1171 |
| boxes | 5147 | 1444 | 0 | 0 | 0 | 0 |
| band, 3 axes (x y z) | 22448 | 3427 | 63 | 15663 | 0 | 0 |
| band, 2 axes (x y) | 12368 | 3192 | 4 | 5184 | 0 | 0 |
| band, 1 axis (z) | 9752 | 1656 | 28 | 2760 | 0 | 0 |
| band, 1 axis (x) | 20690 | 4424 | 24 | 3254 | 0 | 0 |
| **all** | **74708** | **14219** | **155** | **27170** | **0** | **1501** |

The strict rule on the original corpus reproduces the first comparison's totals. With stage
judging, those become sequential 234 agree / 2 wrong / 29 ambiguous and n-ary 224 / 2 wrong /
1 refused / 38 ambiguous before the fixes, and 236 / 0 / 29 and 227 / 0 / 0 / 38 after them.
The snap rule settles all 38 ambiguous cases. On the sequential path, 32 agree and 6 are
unmerged (near_tolerance 7, 10, 18, 20, 22, 23: faces under LINEAR apart kept as two).

### The band, per sweep (agree / unmerged / wrong / refused, before → after)

| axes | δ (LINEAR) | before | after |
|---|--:|---|---|
| 012 | -0.75 | 3053 / 331 / 117 / 2442 | 3148 / 331 / 21 / 2443 |
| 012 | -0.9 | 3053 / 331 / 117 / 2442 | 3148 / 331 / 21 / 2443 |
| 012 | -1 | 3053 / 331 / 117 / 2442 | 3148 / 331 / 21 / 2443 |
| 012 | 0.5 | 4491 / 1444 / 8 / 0 | 4499 / 1444 / 0 / 0 |
| 012 | 0.75 | 2830 / 330 / 5 / 2778 | 2835 / 330 / 0 / 2778 |
| 012 | 0.9 | 2830 / 330 / 5 / 2778 | 2835 / 330 / 0 / 2778 |
| 012 | 1 | 2830 / 330 / 5 / 2778 | 2835 / 330 / 0 / 2778 |
| 01 | -0.9 | 3130 / 798 / 44 / 1215 | 3170 / 798 / 4 / 1215 |
| 01 | 0.75 | 3058 / 798 / 8 / 1323 | 3066 / 798 / 0 / 1323 |
| 01 | 0.9 | 3058 / 798 / 8 / 1323 | 3066 / 798 / 0 / 1323 |
| 01 | 1 | 3058 / 798 / 8 / 1323 | 3066 / 798 / 0 / 1323 |
| 2 | 1.5 | 2438 / 414 / 365 / 332 | 2438 / 414 / 7 / 690 |
| 2 | 2.5 | 2438 / 414 / 365 / 332 | 2438 / 414 / 7 / 690 |
| 2 | 4 | 2438 / 414 / 365 / 332 | 2438 / 414 / 7 / 690 |
| 2 | 4.8 | 2438 / 414 / 365 / 332 | 2438 / 414 / 7 / 690 |
| 0 | -1.5 | 2684 / 563 / 179 / 123 | 2684 / 563 / 0 / 302 |
| 0 | -2.5 | 2684 / 563 / 179 / 123 | 2684 / 563 / 0 / 302 |
| 0 | -4 | 2684 / 563 / 179 / 123 | 2684 / 563 / 0 / 302 |
| 0 | 1.5 | 2472 / 484 / 295 / 298 | 2472 / 484 / 6 / 587 |
| 0 | 2.5 | 2472 / 484 / 295 / 298 | 2472 / 484 / 6 / 587 |
| 0 | 4 | 2472 / 484 / 295 / 298 | 2472 / 484 / 6 / 587 |
| 0 | 4.8 | 2472 / 484 / 295 / 298 | 2472 / 484 / 6 / 587 |
| 0 | 5 | 2750 / 799 / 0 / 0 | 2750 / 799 / 0 / 0 |

Within each band the counts do not depend on δ: (LINEAR/√2, LINEAR] fails alike throughout,
and so does (LINEAR, 4.8·LINEAR). At δ = 0.5 on three axes, only the 8 corner touches fail,
and the fix branches mend them. At δ = 5 on x, nothing fails.

## Bugs

Each open bug is an ignored test in verify/forge-exact/fuzz/tests/repros.rs, named here with
its likely kernel site.

### Fixed on a branch, pending merge

| bug | repro | fixed by |
|---|---|---|
| A cube added on the base's corner vertex is built (33 000 mm³) where the union is non-manifold. The 8 box corner patterns and the band corner overlaps are the same bug. | `cube_on_a_corner_vertex_is_non_manifold`, `band_three_axes_corner_overlap_is_a_touch` | fix-touch da13034 |
| A cut touching the top along an edge leaves a slit edge (V10 E13 where V8 E12). | `cut_touching_along_an_edge_leaves_no_slit` | fix-slit f8d8c89 |
| lang/28: the n-ary regen loses a face (NotClosed). | `nary_keeps_every_face_of_a_patterned_prism_cut` | fix-nary 0f2d3fb, bea6c8a |

With all three merged, the four tests pass. The rest still fail.

### Not bugs

coincident_walls 1, 42 and 46 refuse ZeroThickness correctly: the reference finds the stage
each refuses at non-manifold, empty or a sliver (above). Their repros are removed.

### Open

1. **The notched part's ulp variants** (bug 5 of the first comparison; near-coincident.md).
   - Under §6.1 the exact answer is one solid for every variant: V26 E40 F16, 23 439.9988 mm³.
   - Of the 400 ulp seeds, 67 agree, 301 refuse `Internal(SplitCrossed)` on the plate top
     (seed 0) and 32 build EdgeDegenerate (seed 13). The fix branches change none of this.
   - Sites: imprint's `on_edge` (forge-bool boolean.rs:283) misses a box edge that crosses a
     notch edge about 1e-5 out of plane, so `split_face`'s cycle walk (5154) finds the split
     crossed. For EdgeDegenerate, `place_cuts`/`end_vertex` (4561) gives a crossing about 1e-5
     from a corner a vertex of its own.
   - Tests: `notched_part_nudged_builds`, `notched_part_nudged_has_no_degenerate_edge`.
2. **InconsistentClassification with no move** (near_tolerance 24 and 85).
   - The part: the plate and two overlapping blocks, one 1e-5 above the top (just past LINEAR),
     one 1e-6 into it. The reference gives a solid.
   - Site: `classify_specs`' flood (boolean.rs:5861, refusal at 6000). One piece is reached from
     two specs classified differently.
   - Test: `near_tolerance_24_blocks_build` (features 0 1 2).
3. **SplitCrossed with no move** (near_tolerance 53 and 103).
   - Blocks within LINEAR of the plate's top. The site is as in bug 1.
   - Test: `near_tolerance_53_blocks_build` (features 0 2 3).
4. **Walls chained within LINEAR stay apart** (near_tolerance 45 and 111). This is a §6.1
   conformance bug, not an invalid body.
   - The walls sit at x = 50.000005, 50.00001 and 50.00002. They are one class: each
     neighbouring pair is within LINEAR, and the spread, 1.5e-5, is under the cap.
   - The kernel keeps the 1.5e-5 step between the outer two as a face: V38 E57 F21 where the
     reference gives V32 E48 F18.
   - Site: coincidence is decided per pair (`planes_coincident`, forge-geom intersect.rs:845;
     `ssi::nearly_flush`, forge-bool ssi.rs:65), with no transitive class.
   - Test: `near_tolerance_45_chained_walls_are_one`.
5. **Contact left imprinted** (the unmerged class: 1 444 box patterns, 76 family paths, 12 699
   band runs).
   - A touching boolean leaves its contact patch as extra faces of one surface. The body is
     valid and its volume right.
   - Site: `general` (boolean.rs:557). No pass dissolves an edge between two faces of one
     carrier after `apply`.
   - Test: `touching_subtract_leaves_no_imprint` (meets/overlaps/overlaps subtract: V10 E15 F7
     where V8 E12 F6).
   - **Fixed** (branch claude/repo-review-mbdrlj-fix-tidy). `forge_bool::boolean` hands its result
     out unified. Unify decides "one surface" by the boolean's own `ssi::coincident` (it had its
     own half-LINEAR test, which disagreed on near_tolerance pairs). Before a merge it also asks
     that every edge of the face taken lies on the surface kept, as `check` asks
     (`edge_off_face`): the cylinder test bounds axis and radius apart, and a run of faces each
     within tolerance of the next is not within it of the one kept. The same pass joins, at a
     vertex nothing else ends at, pieces of one periodic curve meeting across its start (an
     ellipse's start moves under a mirror) and two traces of one branch (`IntCurve::spliced`:
     the sequential regen traces apart what the n-ary one traces once). Families, boxes, band
     and more: unmerged 10 450 → 0, agree 60 611 → 71 061, nothing else changed; the metamorphic
     `mirror` and `backends: nary 1` redundant-vertex lines are gone.
6. **The band on two and three axes refuses SplitCrossed.** This is 15 660 three-axis runs and
   5 184 two-axis runs: every δ in (LINEAR/√2, LINEAR], both signs, before and after the fixes.
   - Every plane reads coincident (|d| ≤ LINEAR), but B's corner, moved on two axes, is
     √2·δ > LINEAR from the edge it should be on.
   - Sites: `on_edge` (283) and `place_cuts`' `end_vertex` (4561) miss the corner, and
     `split_face` (5154) refuses.
   - Test: `band_two_axes_split_does_not_cross` (meets/meets/overlaps union, x y, 0.9).
7. **The band builds where §6.1 makes a touch.**
   - Before the fixes: 439 runs built a body where, after the snap, B touches A only at an edge
     or a vertex. After the fixes: 67 remain, 58 edge touches and 9 vertex touches, at δ < 0
     (B short of A).
   - Site: the touch test (boolean.rs:1108) and the four-use count (1746) see only a touch that
     the cuts made exact.
   - Test: `band_three_axes_corner_overlap_is_a_touch` (passes after fix-touch; the edge form
     at δ < 0 still fails).
8. **One-axis band: features over LINEAR read as zero thickness.**
   - Before the fixes, δ in (LINEAR, 4.8·LINEAR) on x or z gave 3 177 invalid bodies (EdgeUses,
     an edge on one face) and 2 889 ZeroThickness refusals. Each one-axis sweep has an exact
     solid with a feature 1.5 to 4.8·LINEAR wide.
   - fix-touch turns most of the invalid bodies into refusals: after the fixes, 52 invalid and
     6 014 ZeroThickness. §6.1 builds features past LINEAR, so these refusals are still
     disagreements.
   - Sites: `place_cuts` (4561) merges vertices within its heads' reach, a 2× allowance, while
     the strip between them keeps both edges. The `4.0 * LINEAR` sample break (3022) skips it,
     and the four-use count (1746) refuses.
   - Tests: `band_one_axis_offset_builds_valid` (equal/meets/equal union, x 1.5: EdgeUses
     before, ZeroThickness after) and `band_one_axis_strip_builds` (meets/meets/overlaps union,
     x 1.5).

## Still open in the reference

RefFacts.lean has one unproven assumption: a closed planar B-rep's ray-parity membership is
constant on every cell of its planes. The reference relies on it and nothing here tests it.
