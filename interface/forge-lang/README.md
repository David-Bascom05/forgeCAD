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

- **Grammar.** LL(1), written out in `grammar.ll1`, the one source of the syntax.
  `tests/grammar.rs` computes nullable, FIRST and FOLLOW, finds zero conflicts (so every text has
  one tree), and generates `src/syntax/table.rs`: those sets and the LL(1) table. The parser in
  `syntax.rs` is a stack driver over that table plus one tree-building action per rule. A syntax
  change is: edit `grammar.ll1`, run `FG_REGEN=1 cargo test -p forge-lang --test grammar` to
  rewrite the table, then write the actions for the changed rules (the match over rules is
  exhaustive, so the compiler lists them). `the_table_is_current` fails when the table is stale.
  Syntax errors carry the exact set of tokens that were expected. A negative literal is one token;
  methods and members bind tightest; `:` is only for keyword arguments and keyed maps, never a
  ternary; `in` is not a unit; a comprehension has one item.
  - **Proven.** The same sets are the certificate of the Lean check on branch `forge-lang-proof`
    (`research/forge-lang/lean`, P6 in `research/forge-lang/ASSUMPTIONS.md`):
    `python3 gen_grammar.py <grammar.ll1> > ForgeLang/LL1/Forge.lean && lake build` re-checks
    LL(1), hence unambiguity and the parser's round trip, in the kernel. The lexer assumptions
    the proof needs are tested in `tests/lexer.rs`: lexing is deterministic, tokens and the kept
    whitespace and comments (`lex_lossless`) rebuild the source bytes, and every literal SetParam
    writes lexes back as one number (NaN and infinity are refused as `NotFinite`).
  - **Same trees as the hand parser it replaced** (`tests/equiv.rs` against `tests/old_parser`):
    identical trees, or the same error at the same span, on every `.fg`, string literal and doc
    code block in the crate, the fair-test models, 50000 random sentences of the grammar, and
    random mutations of all of them. The one difference found was a bug: `[a, b for x in y]`
    parsed, dropping `a`; the grammar now rejects it. Parse time is the hand parser's or a few
    percent less (bracket 105 us, crank 84 us, manifold 180 us, lexing included).
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

The holes lane's bench parts (tools/bench-parts, ported in parts/bench), against their .fc
parts built and measured the same way (`tests/bench_ports.rs`), and their timings (release, niced,
shared machine; cold the median of 5 `fg run` processes, warm from `fg serve`):

| part | volume | faces | edges | bodies | cold | serve unchanged | last param edited |
|---|---|---|---|---|---|---|---|
| valve | 302785.44 | 204 | 524 | 1 | 58 ms | 1.1 ms | 36 ms (3 rebuilt) |
| hub | 233035.40 | 46 | 114 | 1 | 7 ms | 0.6 ms | 2.1 ms (1) |
| clamp | 187986.77 | 65 | 140 | 4 | 7 ms | 0.5 ms | 3.0 ms (5) |
| conrod | 81458.13 | 113 | 284 | 2 | 28 ms | 0.5 ms | 22 ms (2) |
| heatsink | 82389.83 | 438 | 1168 | 1 | 22 ms | 1.1 ms | 8.5 ms (1) |
| bracket | 230513.21 | 37 | 88 | 1 | 5 ms | 0.6 ms | 1.8 ms (1) |
| insert | 293840.50 | 42 | 112 | 1 | 8 ms | 0.8 ms | |
| crank | 116672.18 | 119 | 315 | 1 | 53 ms | 2.2 ms | 33 ms (1) |

All eight match their .fc parts exactly (volume to 1e-6, faces, edges, bodies).

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
- `grammar`: zero LL(1) conflicts; the generated table is current; every random sentence of the
  grammar parses.
- `lexer`: determinism, lossless tokens and trivia, SetParam literals as one number.
- `equiv`: the table parser against the hand parser (see Grammar above).
- forge-ops `roles`: the builders name caps and walls by role.

## Sheet metal (`src/sheet.rs`)

`sheet`, `flange`, `cutout` and `unfold` on forge-sheet's wall-and-joint graph; the shop as data
(`kfactor`, `ktable`, `bend_table`, `per_t`, `stock`, `shop`). Each sheet feature is a feature with
its own id; the sheet is rebuilt whole after each (tens of microseconds) and its body replaced in
the part. The history holds a marker for each (a `Param` equal exactly when the step is), and
features that make nothing pass a sheet part's bodies on (forge-model cannot step them). Anchors
read the kernel's roles (`forge_sheet::Roles`): wall and bend skins, each trim's side faces and
edges, the same on the folded body and on an unfold's (its names salted by the unfold's id).
Ordinary features on a sheet part are refused, typed, both ways round. Bend reliefs (rect,
obround, tear, or none) are data on each flange, the shop's default added where a wall needs one;
flanges stand outside, inside or bend-outside their edge; corners where two flanges meet get a
circular or square relief (default just clear of both bends). The reliefs and corners cut are
listed in the report as typed data. Every build checks the folded clash and the flat overlap; each
refuses its own output and warns on the other, a flat overlap with height fixes. `fg run --dxf`
writes the flat pattern.

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

## Assemblies (`src/asm.rs`)

A document whose first line is `assembly` makes instances of part files and subassemblies
(`part`, `subassembly`), joints and loose constraints between their references, relations, fits
and fit rules, and is solved by forge-asm (kernel/forge-asm). Ids follow the id rules (name, call
path, range index, labels), hashed to forge-asm's ids; blame follows document order. A reference
(`bolt[3].shank.axis`) is evaluated in the part file, so anchors, role names and sketch points all
work; `anchor(ref, kind:, size:, depth:)` in a part file declares a fit feature, `anchor_kind()` a
kind of its own. Drives are params, so motion is a `SetParam`; `fg serve` keeps the solve and the
interference check, so a drive edit re-solves only its partition and re-tests only what moved.
Every forge-asm result is a typed `Error::Asm` or `Warning::Asm` with fixes (a conflict deletes the
statement left out, a limit sets its drive, an interference adds a `fit`). `expect` gains the
assembly measures and `at P = V`. `fg run --render` draws the posed assembly.

Tests (`tests/assemblies.rs`): the four-bar against its closed form through SetParam (and
incrementally), textbook mobility (four-bar 1, slider-crank 1, gear pair 1 and its ratio), a
bolted plate under one M6 fit rule with a shallow-engagement warning, a conflict and a limit with
their fixes applied, ids and poses unchanged by an insert and a move, `expect .. at`, and the
5,000-instance targets through `run` (5,031 instances: 230 plates of 20 bolts under one rule, 50
four-bars, a sliding table). Measured, best of three, the machine at load 20 on 16 cores:

| | target | through `run` |
|---|---|---|
| anchor check | < 10 ms | 5.7 ms |
| overconstraint re-check after one joint edit | < 10 ms | 3.8 ms |
| overlap, cold | < 1 s | 140 ms |
| overlap after a drag | < 50 ms | 7.2 ms |

The whole run of an edit (parse, evaluating 5,000 statements, the solve, the check) is about
130 ms; a cold run about 290 ms, of which loading the parts is 60 ms.
## In the app (fg-ui)

`forge-render` opens `.fg` parts beside `.fc` ones. Every edit is a typed op through
`edit::apply_checked`; a pick becomes the anchor text Forge names it by (`anchors.rs`: each face
and edge of the finished part with the anchor the evaluator resolves back to it, e.g. `base.end`,
`boss.side(top.c)`, `edges(between: (base.end, base.side(plan.outline[0])))`), never a coordinate
or a hash; a pick no anchor names is the typed refusal `NoStableName`. Tests:
`forge-render/tests/fg.rs`, `fg_tools.rs`, `fg_sheet.rs`.

Every tool and command the app has for `.fc` parts, and what it does on a `.fg` part:

| tool / command | .fc | .fg | how, on .fg |
|---|---|---|---|
| Open, New, Save, Undo, Redo | yes | yes | Save writes the text (Ctrl+S); undo/redo step the text the ops made |
| Feature tree (select, open/close groups) | yes | yes | Forge ids: loops `bpsk[0,1]`, calls `lug.hub` nested |
| Tree: delete, reorder | yes | yes | DeleteFeature, MoveFeature (up/down); a loop has no name to move: typed NotAStatement |
| Tree: rename | yes | yes | Rename |
| Tree: suppress / unsuppress | yes | no | no Forge op for a feature left out of the build |
| End-of-part marker (history scrubbing) | yes | no | no Forge op or run option for building part of the history |
| Edit a feature (its dialog again) | yes | params only | its params in the panel (SetParam); its picks: no op replaces a positional argument (SetArg sets keyword ones) |
| Params panel | yes (.fc dims) | yes | driving editable (SetParam), driven read-only with what drives them, structural marked |
| Dimension drag on the model | yes | no | values edit in the panel; no dimension placement on the model yet |
| Errors, lint, fix buttons | flags only | yes | inline, each fix a typed op |
| Start Sketch (on a face or an origin plane) | yes | yes | the face's anchor (`base.end`) or `xy`/`xz`/`yz`; its plane from the evaluator |
| Sketch: line, circle, arc, rectangle, spline, point | yes | yes | written as a Forge block, every number a param of the sketch, drawn points `near(..)` |
| Sketch: dimension, coincident, parallel, perpendicular, tangent | yes | yes | distance, radius, angle, coincident, parallel, perpendicular, tangent, horizontal, vertical, midpoint, equal, fix |
| Sketch: hdist / vdist | yes | rect sides only | Forge has no horizontal/vertical distance: a rect's become its size params; others refused, typed |
| Sketch: project geometry | yes | no | refused, typed (write `project(..)` in the source) |
| Sketch: trim, offset, mirror | off | off | |
| Edit an existing sketch | yes | no | no op replaces a sketch block's rows yet |
| Extrude (add, cut) | yes | yes | `extrude(P, d)` / `cut(P, d)`, P a sketch, loop or closed entity chosen |
| Revolve | yes | yes | `revolve(P, axis: L, angle: ang)`, L a line of the sketch |
| Hole | yes | yes | `hole(at: S.points, d: d, depth: thru|depth)`; seats, fastener sizes, to-next: not in the form yet |
| Fillet, Chamfer | yes | yes | edges picked: `edges(between: (A, B))`, several joined with `+` |
| Shell | yes | yes | open faces picked |
| Draft | yes | yes | faces picked, neutral plane a face, a sketch or an origin plane |
| Thread | yes | yes | `thread(face: F, spec: "M6", length: "full")`, F a cylinder picked |
| Combine | yes | yes | target and tools from the tree's names |
| Work plane, axis, point | yes | yes | `workplane(offset: F, d)`, `workaxis(along: F)`, `workpoint(mid: E)` |
| Linear pattern | yes | yes | `pattern(of: F, layout: lin(along: D, n, step))`, D a face's `.normal` or a line's `.dir` |
| Circular pattern, mirror | off | off | (Forge has `circ`, `mirror`: no form yet) |
| Sweep, Loft, Coil, Rib | yes | yes | profiles, paths and axes from the sketches' names |
| Sheet metal: sheet, flange, cutout, unfold | - | yes | flange on an edge picked (`base.edge(plan.outline[1])`, `F.tip`) |
| Sheet panel: report, flat/folded, DXF, shop | - | yes | the sheet's report as forge-lang gives it (bends, reliefs, corners, and whatever comes), shop violations with fix buttons |
| Units | yes | no | a Forge part is mm and degrees: refused, typed |
| Measure (distance, wall, mass), Check | off | off | the panel shows volume, faces, validity |
| Section, lines, view cube, orbit/pan/zoom, fit, look at, origin planes | yes | yes | the view's own |
| Export STEP / DXF | CLI | CLI; DXF also in the panel | `forgecad export F.fg [dxf]` |

## Gaps, and kernel changes it would want

- **Measuring is the warm path's cost** (220 ms of the crank's 224 ms after a late edit). Mass
  properties per face, cached by face name and invalidated per stage, would make it proportional
  to what changed.
- **Role names**: extrudes, revolves, and (fg-shapes) the skinned walls and caps of sweeps, coils
  and lofts (`roles::skin_side`), a shell's offset skin and rims (`roles::offset`, `rim`); holes,
  threads and text (fg-holes: `forge_model::hole::roles`, `thread::roles`, `text::roles`).
  Fillets' faces have names but no public role API; `H.rim` is the loop nearest the mouth
  where the hole's faces meet the part's.
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
- Holes, threads, patterns, work features, bodies, direct edits, text, imports and sketch
  projections are wired (reference.md); the holes lane's bench parts are ported in parts/bench
  (tests/bench_ports.rs).
- Sheet metal not built in the kernel, so not in Forge: rip and tangent joints, closed and
  overlapping corners (typed refusals), closing bend cycles, ordinary features on a sheet.
- The LL(1) check is a FIRST/FOLLOW checker in the test, not lalrpop or grmtools (no new
  dependencies in the workspace).
