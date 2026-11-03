# Forge (.fg)

A typed, parametric part language for agents, evaluated natively: `.fg` source goes straight to
`forge_model::Feature`s, each built by the kernel as it is made, so anchors and selections read
the part as it stands. No `.fc` text is produced. `fg reference` (src/reference.md) is the full
language; this page is the design as built, the results, and the gaps.

```
shared L = 170
shared rs = 18
shared rp = 13
shared t = 20

sketch plan(on: xy) {
  spindle_c = pt(0, 0)
  pedal_c = pt(L, 0)
  outline = hull(disc(spindle_c, rs), disc(pedal_c, rp))   # lines held tangent by the solver
}
arm = extrude(plan, t)
midplane = rotate(plan.plane, about: axis(plan.spindle_c, plan.pedal_c), by: 90deg)

param pk.a = 40
param pk.ra = 8
...
sketch pk(on: arm.start, at: plan.spindle_c) {           # placed by anchors, never world coords
  outline = hull(disc((a, 0), ra), disc((b, 0), rb))
}
pocket = cut(pk, depth)
pocket_in = fillet(edges(of: pocket, crease: "concave"), r)
breaks = chamfer(edges(between: (extractor, spindle)) + edges(of: pedalbore, shape: "circle"), size)
expect volume = 107177 +- 0.1%
```

## Design as built

- **Grammar.** LL(1), written out in `grammar.ll1`; `tests/grammar.rs` computes FIRST and FOLLOW
  and finds zero conflicts, so every text has one tree. The parser is the grammar by hand, every
  choice on one token. A negative literal is one token; methods and members bind tightest; `:` is
  only for keyword arguments and keyed maps, never a ternary; `in` is not a unit.
- **Ids from syntax only.** The assigned name, the call path (`lug.hub`), range indices
  (`bolt[2]`) and labels of keyed loops (`vtap[lf]`). A loop over a plain list may compute but not
  make features (`PositionalId`). Ids hash to the kernel's history ids, so names in the B-rep
  survive inserts, deletes and moves.
- **Single assignment, no shadowing**, checked dynamically and statically: no local (function
  parameter or binding, loop or comprehension variable, sketch entity) has a top-level name.
  Function bodies see their parameters and top-level names.
- **Dimensions**: Length, Angle, Count, Ratio (with Area, Volume for products). Mixing them is a
  type error; a size never becomes a count.
- **Params**: `shared x` for every feature; `param F.x` local to feature F and what it makes;
  `param F[k].x` for one loop instance; links by full name (`param bore.r = hubsk.r + 2`), shown
  as relations.
- **Provenance.** Every number is driving (exactly one param: its handle), driven (an expression of
  params: no handle) or a constant. Params observed in a decision (loop bounds, indices,
  comparisons, selection filters, the inputs of a sketch the solver moves) are reported as
  structural: editing one re-runs; any other edit only re-reads. `fg run --json` lists each
  param's value, links, structural flag and the features it drives, and each feature dimension's
  source.
- **Anchors by role.** `E.start`, `E.end`, `E.side(entity)`, `R.side(entity)`, `E.axis`, `H.rim`,
  `S.plane`, sketch points. Faces are found by the kernel's role names (`forge_ops::roles`, a commit
  of its own on this branch) through splits (`Name::is_piece_of`) and joins (`joins.aliases`);
  planes carry the sketch's own x and origin, never the face's corner. Selections: `of`
  (lineage), `between` (features or face sets), kind, crease, direction, radius, length. Selecting
  by position (`extreme`) is an error.
- **Sketches** on forge-sketch: points written as numbers are fixed, `near()` is a free guess,
  helpers (`hull`, `slot`, `rect`, `poly`, `path` with `tarc` and splines) add the constraints
  their meaning needs: tangency is held by the solver, never computed into the source.
- **Typed errors and fixes.** Every error is an enum variant with a span and data (no string
  variants); `fixes()` returns typed ops. Lints are the same records with `severity: lint`.
- **Rules**, each its own switch (`Rules`, a `Level` per rule):
  - hard: named features, keyed ids, single assignment, no positional selection; types and
    dimensions, unknown names, arity, absolute references, the call budget (recursion allowed,
    100000 calls and 128 deep per top-level statement);
  - lint: numbers outside params (fix: `LiftLiteral`, local or shared), another feature's param
    used without a link (fixes: `AddLocalParam`, `MakeShared`), world placement after the first
    feature, an `expect` copied from a run.
- **Edit ops** (`edit.rs`), pure functions of (document, op): `SetParam`, `AddLocalParam`,
  `MakeShared`, `SplitInstance`, `AddFeature`, `DeleteFeature`, `MoveFeature`, `LiftLiteral`.
  A feature's block is its statement and its own params written directly above it. Add, Delete
  and Move make the proven pre-check: no name they bind may be read after the point where it stops
  (or starts) being bound, nor in a function body, and an added or moved block reads only names
  bound above it; else `StillRead` names the reader. `apply_ast` is each op on the tree;
  `apply_checked` refuses an op whose result does not check, and reports what a `SetParam`
  reaches (values driven, features, structural).
- **Building.** `Builder` keeps every stage; `fg serve` keeps them across runs and reuses a stage
  while the history (features and ids) is unchanged up to it, so an edit rebuilds from the first
  changed feature. The finished part's measures are kept with its stage.

`SplitInstance` writes `param bpsk[1,1].w = <bpsk.w's right side>`: the instance's own param,
looked up before the shared one. That is the proof's `w = [q0..qn][i]` with every entry but one
still reading `w`: the same numbers until the new param is edited, and then one instance changes.

## Results

The three fair-test parts, ported by hand from their .fc versions (`parts/`), build with the
.fc part's exact volume, face and edge counts, valid, with no errors or lints, and no number
outside params, no world placement after the first sketch, no positional selection:

| part | .fc volume | Forge volume | faces | edges | features |
|---|---|---|---|---|---|
| crank | 107177 | 107177.04 | 65 | 141 | 21 |
| bracket | 183449.2 | 183449.24 | 137 | 316 | 32 |
| manifold | 687248.8 | 687248.76 | 209 | 484 | 46 |

Bench parts (tools/bench-parts, lane fg-shapes, `parts/bench_*.fg`), against the .fc less the
lines that wait on other lanes (threads, text, `in BODY`); patterns are loops. Cold `fg run` is
the process, median of 5; warm is an unchanged rerun in `fg serve`.

| part | left out | .fc volume | Forge volume | faces | edges | cold | warm |
|---|---|---|---|---|---|---|---|
| jack | thread, text | 54989.1 | 54989.12 | 166 | 478 | 70 ms | 1.9 ms |
| quadarm | (nothing measured) | 19376 | 19375.96 | 73 | 167 | 12 ms | 1.0 ms |
| volute | (cosmetic threads) | 802514.1 | 802514.14 | 70 | 165 | 109 ms | 3.2 ms |
| manifold | nothing | 884334.5 | 884334.47 | 78 | 192 | 49 ms | 3.0 ms |
| guard | cuts `in guard` | 2442097.7 | 2442097.69 | 29 | 60 | 9 ms | 0.1 ms |
| gearbox | threads | 1000499.5 | 1000499.54 | 159 | 449 | 52 ms | 6.4 ms |
| frame | threads | 423092.7 | 423092.68 | 129 | 367 (.fc 369) | 75 ms | 3.6 ms |

Frame's two edges: the kernel's finish at the axle slot joins edges according to what the face
names hash to, so the count follows feature ids (the .fc itself gives 77 or 76 edges on a cut-down
copy with only one feature's `id` changed); Forge's ids hash from names, the .fc's from lines.

Timings (release, niced, the machine shared with other agents; median of 5 for cold):

| part | cold `fg run` (process) | serve: first | unchanged | last param edited | first param edited |
|---|---|---|---|---|---|
| crank | 435 ms | 433 ms (kernel 208) | 1.2 ms | 224 ms (1 rebuilt) | 429 ms (22 rebuilt) |
| bracket | 17 ms | 17 ms (kernel 9) | 1.7 ms | 5 ms (1 rebuilt) | 14 ms (31 rebuilt) |
| manifold | 97 ms | 97 ms (kernel 26) | 7.6 ms | 69 ms (1 rebuilt) | 94 ms (45 rebuilt) |

Parse, static checks and evaluation take under 2 ms on every part. On the crank, measuring the
finished part (exact mass properties of its blends and the validity check) takes 220 ms, which
is all of a warm rebuild after a late edit.

Tests (`cargo test -p forge-lang`):
- `ports`: the three parts, parity as above.
- `typecheck`: 16 tests of errors and lints, their spans and fixes, and that fixes apply.
- `edits`: on the bracket, a size changed, the bolt holes moved, a drilled dimple inserted above
  other features, a fillet moved and deleted, deleting the web refused (`StillRead`, the lug
  plane reads it), one of four pockets split off and narrowed, a wrong-dimension value refused.
  Every feature not added or deleted keeps its id, and the part builds. Editing the shared web
  thickness `tw` changes what reads it (`web`, `hubsk`, `boresk` through its link) and, without
  reading it, `hub` and `bore` (their sketches read it) and **`wpsk` and `wpocket`: the web pocket
  sits on `web.end`, so it moves with `tw` though it never reads it.** Provenance says what a param
  drives; anchors carry what sits on it.
- `props`: 400 random documents, four random typed ops each: the edited text parses to the tree
  the op makes, SetParam changes only the param's right side, a document that checked still
  checks after any op that applied, and ids of features not added or deleted are unchanged.
- `grammar`: zero LL(1) conflicts.
- forge-ops `roles`: the builders name caps and walls by role.

## Sheet metal (`src/sheet.rs`)

`sheet`, `flange`, `cutout` and `unfold` on forge-sheet's wall-and-joint graph; the shop as data
(`kfactor`, `ktable`, `bend_table`, `per_t`, `stock`, `shop`). Each sheet feature is a feature with
its own id; the sheet is rebuilt whole after each (tens of microseconds) and its body replaced in
the part. The history holds a marker for each (a `Param` equal exactly when the step is), and
features that make nothing pass a sheet part's bodies on (forge-model cannot step them). Anchors
read the kernel's roles (`forge_sheet::Roles`): wall and bend skins, each trim's side faces and
edges, the same on the folded body and on an unfold's (its names salted by the unfold's id).
Ordinary features on a sheet part are refused, typed, both ways round; reliefs are refused until
decided (sheet-metal/sheet-crate/reliefs.md). `fg run --dxf` writes the flat pattern.

Hand-written parts (`parts/sheet/`), each clean (no errors, warnings or lints) and matching
forge-sheet's closed forms: L, U (up 90 and down 60, unfolded), Z (a lip off a flange's tip),
an open box (four flanges off a notched base, open corners), an L with three holes across its
bend, an L with a spline window in its flange. Folded volume within the mass error of the closed
form; flat volume likewise; V_fold - V_flat = sum theta len t^2 (1/2 - K); the DXF read back gives
the flat body's volume (area times t); K leaves the folded body the same to the bit.

| part | cold `fg run` | serve: first | unchanged | last param edited | first param (t) edited |
|---|---|---|---|---|---|
| l | 1.6 ms | 0.44 ms | 0.18 ms | 0.25 ms (1 rebuilt) | 0.26 ms (2 rebuilt) |
| u | 2.0 ms | 0.86 ms | 0.36 ms | 0.59 ms (1) | 0.63 ms (4) |
| z | 1.8 ms | 0.62 ms | 0.31 ms | 0.44 ms (1) | 0.45 ms (3) |
| box | 3.6 ms | 2.44 ms | 1.60 ms | 2.13 ms (7) | 1.97 ms (7) |
| holes | 5.3 ms | 4.01 ms | 0.61 ms | 3.82 ms (2) | 3.67 ms (4) |
| spline | 3.1 ms | 1.94 ms | 0.39 ms | 1.73 ms (2) | 1.67 ms (4) |

Tests: `sheet` (the parts against closed forms, the DXF, K, anchors folded and flat, names under
edits, every refusal typed and its fix applied), `sheet_props` (300 random sheet parts with four
random typed edits each: text and tree agree, ids hold, a checked document still checks unless the
edit put a feature after the unfold or gave a size the sheet cannot take; 120 random parts built
valid, the fold/flat identity, a height edit renaming no face).

## Gaps, and kernel changes it would want

- **Measuring is the warm path's cost** (220 ms of the crank's 224 ms after a late edit). Mass
  properties per face, cached by face name and invalidated per stage, would make it proportional
  to what changed.
- **Role names**: extrudes, revolves, and (fg-shapes) the skinned walls and caps of sweeps, coils
  and lofts (`roles::skin_side`), a shell's offset skin and rims (`roles::offset`, `rim`). Holes'
  walls and floors (`hole::MOUTH`, `BORE`, `BOTTOM` are private; lane fg-holes) and fillets' faces
  have names but no public role API; `H.rim` is the loop nearest the mouth where the hole's faces
  (by lineage) meet the part's.
- **Lineage ("made by") is forge-agent's** (`select::lineage`, by name and by overlap on one
  surface). The kernel could carry it, as it carries names.
- **One chamfer size per feature** (`ChamferSize` is per feature): groups with two sizes need two
  features.
- **Faces picked by geometry** (`faces(of: F, normal: ..)`) carry the kernel face frame's x and
  origin when sketched on; only role anchors carry the sketch's. Use role anchors.
- **`check` cannot resolve geometric anchors**: without building, `E.side(..)` planes and
  selections are unknown, and what depends on them is checked for types only.
- Shapes (branch fg-shapes, `src/shapes.rs`): sweep, coil, loft, shell out, draft, offset_faces,
  thicken, rib, facefillet, fullround, ball/sphere; see the reference. Not wired by this lane:
  thread, text, patterns, holes' options, bodies (lane fg-holes).
- Sheet metal not built in the kernel, so not in Forge: reliefs (waiting on a decision), rip and
  tangent joints, corner treatments (corners stay open), closing bend cycles, hole and slot
  checks against bends and edges, PEM hardware, ordinary features on a sheet. A flange's
  "material inside/outside" position is not an option: the flange turns on the edge itself.
- The LL(1) check is a FIRST/FOLLOW checker in the test, not lalrpop or grmtools (no new
  dependencies in the workspace).
