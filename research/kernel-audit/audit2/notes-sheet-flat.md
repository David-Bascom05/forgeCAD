# Sheet-metal flat pattern export: audit (read-only)

Repo /home/user/forgeCAD, branch claude/repo-review-mbdrlj-audit2, 2026-11-22. Nothing under kernel/ or interface/ was changed.
Scratch artefacts: scratchpad/out/*.dxf|*.dwg (10 shipped sheet parts + 2 probes), scratchpad/parts/{shallow,ell}.fg, scratchpad/scripts/dxfchk.py.

## 0. Short answer

The flat pattern is **not** recovered from the flat solid or a tessellation. `Strips::pattern`
(kernel/forge-sheet/src/build.rs:2982-3028) walks the exact 2D region loops of every strip (walls and
bend zones, already in developed coordinates) and maps each piece into the flat by the strip's rigid
motion (`developed`, build.rs:3881-3913). Lines stay lines, arcs stay arcs (centre/radius/angle),
splines are exact NURBS sub-curves (`Spline::stretch`, forge-geom/src/spline.rs:947), ellipses arrive
as rational quadratic NURBS (verified: ellipse cutout -> two SPLINE entities, weights 1, 0.7071.., 1).
Nothing is chorded. Contour chaining is **topological** (loop order + the `twin` map across tangent
lines, build.rs:3031-3040), not endpoint-epsilon chaining. Bend lines and the zone widths come from the
same rigid motions and the same `ba` that built the flat body, so they follow the bend rule exactly by
construction (`rn = ba/theta`, build.rs:1543; bend line at u = ba/2, build.rs:3016-3017).

DWG numbers are written bit-exact (raw RD/BD/DD doubles, forge-dwg/src/bits.rs:63,96-123); DXF numbers
use Rust `{:?}`, the shortest round-trip repr (pattern.rs:516-519), so every coordinate reads back to the
bit — except arc angles, which DXF needs in degrees (`to_degrees`, pattern.rs:588-589), so arcs are exact
only to rounding in DXF (the test admits this, tests/pattern.rs:254).

The case-patching is modest and local: the "is this the same line/circle?" merge in `joined`
(build.rs:3916-3943) uses three unrelated tolerances; the tangent-line twin match is a nearest-within-TOL
search (build.rs:1838-1863); and **`bounds()` treats every arc as its full circle** (pattern.rs:643-646),
which drives `outer_contour` (pattern.rs:134-143), the part-note position and the DWG limits. That last
one is a real, reproduced bug (section 1.1). The big tolerance problems of the boolean (README sections
0-1) do not appear in this path.

The forge-dwg geometry modules the task named (draft/curve.rs, draft/intersect.rs, scene/) are **not on
the export path**: `dwg_layered` (pattern.rs:663-815) only builds a `forge_dwg::Doc` and calls
`forge_dwg::write` (write.rs:296), which uses bits.rs. The draft/ and scene/ code serves the GUI DWG
editor and viewer (interface/forge-render/src/shell/dwgcmd.rs). It is full of case-patching; see section 1.6.

## 1. Case-patching (ranked)

### 1.1 `bounds()` by full circles -> wrong outer contour, misplaced note, huge limits (bug, reproduced)
- pattern.rs:628-654 `bounds`: `Curve2::Arc { c, r, .. } => add(c±r)`, which is the whole circle and
  ignores the arc's sweep. The doc comment says "never inside the curves", so it is a conservative box,
  not the curve's box.
- pattern.rs:134-143 `outer_contour` = "the contour whose **box** is largest". It is a heuristic for a
  fact that is known exactly: the loops are oriented (outer CCW, holes CW, asserted in
  tests/pattern.rs:296), and a sheet must be one piece (SheetSplit, build.rs:1904-1928). So the outer
  contour is the single one with positive signed area. A box comparison plus a box that is too big
  breaks it.
- Reproduced with scratchpad/parts/shallow.fg: a 10x5 window whose bottom is a shallow arc of R 495
  in a 68.4 x 60 blank. The window's "box" is 990 x 990, so `outer_contour` picks the window. With
  `dxf_layered`/`dwg_layered` and split outer/inner layers (the GUI exposes this,
  interface/forge-render/src/gui_sheet.rs:231-245; the Inventor-style IV_OUTER_PROFILE test is
  tests/pattern.rs:518-), the blank outline would go to the interior layer and the window to the
  outer one.
  It is already visible in the default DXF: the part note is written at **(-480.0, -971.0)** for a
  blank that lies in [0, 68.4] x [0, 60] (pattern.rs:423-430, scratchpad/out/shallow.dxf TEXT entities).
  The DWG `limits` come from the same box (pattern.rs:807-812), and so does forge-dwg's own extents
  (write.rs:593-596).
- Fix: outer = the contour with positive signed area (`area_term` exists in the test); for the box,
  use the arc's real extremes (its ends plus each axis crossing inside the sweep).

### 1.2 `joined`: one fact ("same carrier") decided with three unrelated tolerances
build.rs:3916-3943 (used by `push_joined` 3945 and the wrap-around join 3001-3006):
- Lines: `|u×v| <= 1e-12·|u||v|` (a relative angle of 1e-12, unrelated to `tol::ANGULAR`=1e-11)
  **and** `|b - b2| <= TOL` (TOL = LINEAR = 1e-5) **and** `u·v > 0`.
- Arcs: centres within TOL, radii within TOL, `|sin(½·angle gap)| <= 1e-12` (an angular test), and
  total sweep `<= TAU·(1+1e-12)`.
- The DXF and DWG writers then decide "whole circle" separately with `>= TAU·(1-1e-12)`
  (pattern.rs:574 and 700, the same expression written twice) and the test with a third copy
  (tests/pattern.rs:260).
- Effect: the merge restores a hole cut into pieces at tangent lines into one CIRCLE (holes.fg: 3
  CIRCLEs, interface/forge-lang/tests/sheet.rs:337-345). Whether pieces merge depends on rounding
  in the chained rigid motions (`dev[bi] = dev[wi]·ja·shift·jb⁻¹`, build.rs:1136; strips 1168-1169),
  which grows with bend depth. Failing to merge is harmless geometry (two arcs instead of one), but
  entity counts and CIRCLE-vs-ARC are not invariant. The angle tests are not scaled to TOL/r, so a
  large-radius arc can match within TOL in position and still fail the 1e-12 angle test.
  The principled form: the pieces come from one source segment (`Bound.id`, build.rs:1743-1747), so
  merge by **source identity and adjacency**, not by geometry.
- Splines are never merged (`_ => None`): a closed spline cutout exports as 2 open SPLINEs
  (spline.fg, ell.fg). Each is a Bezier-piece rebuild with interior knots of full multiplicity
  (`stretch` -> `from_pieces`; 11 knots for 7 control points, degree 3). The curves are exact, but
  the C2 source is written as C0 knot vectors, and the pieces are split at region-boolean break
  points and at every knot inside a bend (build.rs:1731-1736).

### 1.3 Tangent-line twin matching: nearest within TOL (build.rs:1834-1878)
For each bend-zone tangent stretch, it picks the wall stretch with the smallest
`max(|a-y1|, |b-y0|)` and accepts it if that is <= TOL. Otherwise it refuses with TangentMismatch
(an honest refusal). Two issues:
- It is not one-to-one: two bend stretches can pick the same wall stretch, and the later insert
  overwrites the earlier one. The "every wall stretch found" pass (1867-1878) only catches a wall
  stretch left without a match, not a double match.
- The fact being decided (these two pieces are the two sides of one cut of the tangent line) is
  known symbolically from the cut's source; here it is recovered from coordinates.

The pattern walk relies on `self.twin[&(l,i)]` (3036), which panics if a key is missing.

### 1.4 Arc split at 0.99π in bends; knot cuts filtered by 1e-12 (build.rs:1722-1736)
`parts = ceil(turn / (0.99·π))`: a magic fraction, so the ruled side wall stays regular in the
folded body. The pattern re-merges the pieces through 1.2. Knot cuts closer than 1e-12 (in
parameter) to an end are dropped. `flat_in_bend` (3960-3976) picks "the span holding the middle"
because "cut at a knot to rounding, a sliver of the next span may come along". That is a rounding
patch on the folded wrap, not on the pattern.

### 1.5 Silent drop and fallbacks
- `pattern()` does `if let Some(c) = developed(..)` (build.rs:2996-2998): a spline piece whose
  `stretch` returns None is **left out without a word**, so the contour comes out open. In practice
  that means an empty or zero-width range, but no error or warning is raised.
- DWG spline weights: `w.get(i).copied().unwrap_or(1.0)` (write.rs:704) quietly fills missing weights.
- `piece_box` returns a ±1e300 box when `flat_in_bend` fails (build.rs:4025-4030), a "never prunes"
  fallback (clash check, not the export).
- `tangent_lines` with a zero-length bend returns the degenerate line twice (pattern.rs:149-151).
- Note-angle flip at ±π/2 ± 1e-9 (pattern.rs:413-417, duplicated at 769-773). Cosmetic.
- `RADIUS_MATCH = 1e-9` (shop.rs:44) matches allowance-table rows by radius (and a single row by
  angle, shop.rs:133). This is a third "same number" tolerance, next to TOL and 1e-12.
- `flat_overlap` uses `LIMIT = 1e-6` mm² of area (build.rs:2713), an absolute area constant that is
  not derived from TOL. It decides refuse versus touch. A region-boolean error inside a pair counts
  as "no overlap", unless no real overlap is found anywhere, in which case it becomes
  FlatOverlapUndecided (2843-2864).

### 1.6 forge-dwg draft/ and scene/ (not on the export path; GUI editor/viewer)
- draft/intersect.rs: tangency is decided three ways: `ta×tb < 1e-7` (409), `|dg/ds| < 1e-6` (450),
  `ta×tb < 1e-6` (554). Duplicate merging uses `tol·10` (545), and a conic filter uses `tol·1e3` (401).
  Numeric curves are met "by subdivision of both into chords followed by Newton" (header, 8; `chords`
  462).
- draft/curve.rs:1450-1484 `scan_roots`: sign changes over sample parameters, plus "near-zero minima"
  with `b < 1e-3·scale` and `|g| <= 1e-9·scale`, and dedup at 1e-12. Two roots between two samples
  are missed. This is sampling, not certified isolation (the README's rank-4 pattern again).
- scene/geom.rs: display tessellation by chord tolerance (arc_segments 151, nurbs_pts 252). This is
  fine because it is only for drawing.

## 2. Honest refusals (good)
- Joint lengths differ by more than TOL -> JointLengths (build.rs:1070-1078). Bend angle out of
  (ANGULAR, π or 2π - ANGULAR) -> BendAngle (1082-1089). Bend rule off its table -> BendRule/NoRow
  (shop.rs:53-80).
- Spline of degree > 3 on a bend -> TrimCurveUnsupported (build.rs:1704-1716).
- Bend end without relief -> NeedsRelief; a cut on a mold line -> TangentMismatch; a side along a
  tangent line -> TrimAlongBendLine (1676-1792); twins not found -> TangentMismatch (1848-1878); a
  disconnected sheet -> SheetSplit (1904-1928).
- Flat overlap, touch (with a suggested seam) or undecided -> FlatOverlap / FlatTouch /
  FlatOverlapUndecided (build.rs:2712-2930). These are exact area booleans, not sampled.
- `Sheet::pattern` refuses through the flat build's error (interface/forge-lang/src/sheet.rs:4629-4632).

## 3. Legitimate exact special cases
- Per-kind `developed` (build.rs:3881-3913): line by its ends, arc by centre plus rotation angle,
  spline by control-point transform. A rigid motion maps each kind exactly; this is not a patch.
- Reversed spline: knots mirrored `lo+hi-k`, points and weights reversed (3894-3902). Exact.
- Whole-turn arc -> CIRCLE; arcs with negative sweep -> DXF/DWG CCW (lo, hi) (pattern.rs:574-590, 700-714).
- Hem allowance override and hem setback (build.rs:1091, 1112-1115): explicit definitions, not
  patches.
- DD "patch on default" double encoding (bits.rs:108-123) is lossless.

## 4. Precision
- Exact curves, no chords, no polylines (Curve2 has no polyline variant; pattern.rs:9-28).
- DXF: shortest round-trip floats (`{:?}`), so values come back to the bit (test
  `dxf_round_trip_against_a_reader`, tests/pattern.rs:237-310, compares with `assert_eq`). Arc angles
  are in degrees, so only to rounding. Side effects of exact printing: rounding noise is written
  faithfully, e.g. jogged.dxf has `5.179731399943796e-16` where 0 is meant, and seams.dxf has
  `0.9999999999999997`. Small numbers print in exponent form (`1e-10`, `5.18e-16`). Rust's form has
  no explicit "+" or two-digit exponent. AutoCAD and most readers accept it, but it is worth one check
  against the strict readers the shops use.
- DWG: bit-exact. `dwgroundtrip` on all 10 exported flats: byte-identical re-write, all objects the
  same. Arcs are stored in radians, with no conversion.
- Endpoint closure: each piece's end is computed from its own data (line ends from vertex starts, arc
  ends from c + r·angle, spline ends from control points), so contours close to rounding. Measured on
  the 10 shipped parts: worst end gap 3.6e-15 mm, and 6.9e-14 for the R495 probe. The tests use 1e-9
  (tests/pattern.rs:285-290).
- Bend lines: `developed.apply((ba/2, 0))` to `(ba/2, length)` (build.rs:3016-3017). The test checks
  them bit-for-bit (`assert_eq!(b.a, mid)`, tests/pattern.rs:304-308) and checks area × t against
  the flat body's volume. The zone width is `ba` by construction (`rn = ba/θ`), so bend lines follow
  the rule exactly. Tangent lines are **rebuilt** as centre ± n·ba/2 with a normalised n
  (pattern.rs:146-162), not taken from the zone's own boundary (`developed.apply((0,y))`,
  `((ba,y))`), so they agree only to rounding (the test uses 1e-9, tests/pattern.rs:552-553). Bend
  lines always span the whole mold line, even where cuts or reliefs remove the zone's material (a
  fidelity gap, not a patch).
- DXF and DWG writers duplicate the layer, note and mark logic (pattern.rs:178-495 vs 663-815),
  including the 1e-9 note flip and the 1e-12 whole-circle test. Two copies of one definition.

## 5. History
`git log --oneline` on pattern.rs, build.rs and forge-dwg/src/{write,bits}.rs shows only
**979190b** "Install ForgeCAD launcher and update from latest main" (2026-11-07). The files arrived
in one import (`--all` has 219 commits, none earlier for these paths). So no per-part fix commits
can be identified. The only in-code hints are the rounding comments at build.rs:3963-3964 (knot
sliver) and 1722 (0.99π), plus a stale test comment, "the outer contour and five holes" against
`assert_eq!(.., 4)` (tests/pattern.rs:291-292).

## 6. Tests and tools run
- `cargo test -p forge-dwg`: lib 145 passed, tests/corpus.rs 4 passed, tests/scene.rs 3 passed,
  doctests 0. 0 failed, 0 ignored.
- `cargo test -p forge-sheet --test pattern`: 7 passed, 0 failed, 0 ignored.
- Examples (21 in kernel/forge-dwg/examples, all CLI tools, none fuzzers). Ran:
  - `dwgexport_test` (every shape through the R2000 writer) -> `dwgcheck`: all object types decode
    and re-encode the same.
  - `dwgcheck` on tests/data/mix_{2000..2018}.dwg: all ok. raw classes 502/503/506/507 decode but
    are not re-encoded "same", as expected for raw.
  - `dwgroundtrip` on the corpus: every object, header and class the same, with smaller files (it
    re-packs). On the 10 exported flats it is byte-identical.
- No fuzz directory for forge-dwg or forge-sheet (fuzz dirs exist only under research/* and
  verify/forge-exact).
- `fg run parts/sheet/{holes,spline,hems,box,jogged,u,punched,formed,seams,hat}.fg --dxf --dwg`: all
  exported. Entity census per file in the dxfchk output above.
