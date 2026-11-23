# Import coverage: what the importers now translate (2026-11-23)

The fillet census (research/fillets/census.md) found many real parts that never reach the kernel. This pass ranks
why, per dataset, and translates the biggest causes into features `.fg` already has. Every part that newly
translates is built and held to its source's volume and area. Only a part within 1e-4 of both counts as a match.
A part that translates but is off is reported here, not counted.

Data was staged as research/fillets/HANDOFF.md says, outside the repo: the FreeCAD library's 245 files with fillets
(sha1 checked against map.tsv), CADFS by HTTP range from Hugging Face (VladPyatov/CADFS at dd76a9b,
`sorted(unique_models.json)[::200]`, 1914 parts), and WHUCAD's hand-drawn folders 0100 to 0103 (every 7th of the 2229
parts with a fillet, 319). The baselines reproduce the census: FreeCAD 677 bodies {match 484, untranslated 130,
refused 53, wrong 5, fail 5} at the README's 1e-3, and CADFS 476 fillet parts with 238 untranslated and 164 matching
at 1e-4. No dataset file is in the repository.

"Before" is the tree this branch started from; "after" is this branch. Both runs used the same kernel. The tables
come from `research/import/tally.py`.

## Headline

| set | parts | untranslated before | after | match (1e-4) before | after |
|---|---:|---:|---:|---:|---:|
| FreeCAD library, bodies | 677 | 130 | 52 | 394 | 512 |
| CADFS, all parts | 1914 | 640 | 615 | 1099 | 1111 |
| CADFS, parts with a fillet | 476 | 238 | 228 | 164 | 166 |
| WHUCAD, 1 in 7 with a fillet | 319 | 35 | 29 | (1e-3: STEPs staged for the newly built only) | 0 new |

No part got worse on any set: no match was lost, and nothing that built before stopped building.

The first pass took on the four biggest FreeCAD causes and CADFS's extrude draft. A second pass took on causes that
need only importer work: FreeCAD's Thickness, a mirror plane on a feature's face, a pattern axis on a feature's edge,
CADFS's extrude up to a curved face, and WHUCAD's splines.

In the first pass FreeCAD's matches rose by 112, and only 30 of those were newly translated parts:
- 30 refused before and match now;
- 52 matched only at 1e-3 before (a polyline stood in for a spline) and now match at 1e-4.

Both come from writing sketch splines exactly.

## FreeCAD

| class | before | after |
|---|---:|---:|
| match | 394 | 512 |
| mismatch | 95 | 79 |
| refused | 53 | 29 |
| fail | 5 | 5 |
| untranslated | 130 | 52 |

### The causes this pass took on

| cause (before) | bodies | translated | match | mismatch | refused |
|---|---:|---:|---:|---:|---:|
| a blend along a spline | 30 | 30 | 1 | 29 | 0 |
| no edge where the base's edge lies | 18 | 17 | 17 | 0 | 0 |
| feature AdditivePipe | 17 | 12 | 5 | 6 | 1 |
| feature Draft | 10 | 7 | 7 | 0 | 0 |

**A blend along a spline (30 → 0).** FreeCAD sketches hold B-splines, and the importers wrote each one as a
polyline. A spline edge was then many short edges that no blend could round as one. A clamped, non-periodic
B-spline is now written exactly, as `spline(poles, degree:, knots:, weights:)`, wherever the sketch's arrangement
leaves it whole. A spline the arrangement cuts, or an unclamped one, is still a polyline, and only then is the part
marked approx.

- All 30 bodies translate.
- One matches (ef0a48fc.Body, 8e-7).
- The other 29 are 1.2e-4 to 1.5e-4 off. They are ten copies of one model, three bodies each (pad, fillet, sphere
  groove, fillet).
- **The source disagrees with itself.** FreeCAD's stored pad is 8.6e-6 off the exact area of its own sketch's
  spline. Ours equals that area to 1e-10, by quadrature of the XML's poles and knots. So FreeCAD's stored shapes were
  not made from the spline the file holds. The extra 1e-4 builds up through the fillets and the groove.
- These 29 are reported, not counted.

**No edge where the base's edge lies (18 → 0).** Two different things.

- 15 are fillets outside any body on a `Part::Extrusion`, and 2 are on a `Part::Cut` of cylinders. The chain never
  built the base, so the fillet looked for edges on nothing.
  - Fixed: a fillet or chamfer outside a body acts on its `Base`.
  - Fixed: `Part::Extrusion` is now a Part solid. In the older form `Dir`'s length is the distance; in the newer one
    `LengthFwd`/`LengthRev` run along `Dir`, the sketch normal, or a slant.
  - All 17 match, to 9e-8 or better.
- The 18th (33eb6124.Body003) now finds its edge but no Forge selection names it apart from its neighbours.
- Once its splines were exact, the spline model above met this cause as well. FreeCAD had split a sphere cut's edge
  at a vertex we don't have. The match now also takes their edge when it lies on ours, but only if the edges they
  picked cover all of ours, so we never round more than they did.

**AdditivePipe (17 → 0 under that name).**

- 11 are Frenet sweeps along a `Part::Helix` held by a shape binder (thread models). A helix's Frenet frame turns
  with the screw motion, so the profile is wound rigidly about the axis: that is `coil(.., axis:, pitch:, height:,
  left:)`.
  - Pitch, height and hand are measured from the helix the binder stored, not from the helix's properties (a binder
    keeps the shape as it was bound).
  - 5 match, to 1.3e-8.
  - 6 are 6.7% off. They are three copies of one model (pitch 1.5, height 32). The helix and the binder both hold
    32 of rise, but FreeCAD's stored pipe stops at 30: a stale stored shape, or OpenCascade stopping a turn and a
    third short. Reported, not counted.
- 6 are Standard or Frenet sweeps along a sketch's lines and arcs. These become `sweep`, a planar path carried
  without twist (Python reader only; see "Rust").
  - One translates, and the kernel then refuses a later feature (d45d0db3.Body001, `Extent: Ambiguous`).
  - 2 have the profile inside the path, 1 has a path with a corner (Transition), and 1 goes through sections.
  - 1 stops earlier, on a sketch with no closed region.

**Draft (10 → 0 under that name).** `draft(FACES, angle:, neutral: PLANE)`.

- Each FreeCAD `FaceN` is found as the one face of ours that all its edges bound. A disc on a cylinder's end shares
  all its edges with the side face; then the face bounded by nothing else is the one.
- The neutral plane is its face's plane. The pull is that face's outward normal, or a line edge's direction, and
  `Reversed` turns it.
- All 7 that reach it match to 6e-15. The other 3 stop on a Boolean, a Part loft, or a pattern's faces not found.

### The second pass: importer work only

| cause (before) | bodies | translated | match | mismatch | refused |
|---|---:|---:|---:|---:|---:|
| a mirror in PartDesign::Pocket / Pad | 6 | 6 | 1 | 0 | 5 |
| a pattern axis on PartDesign::Pad / Pocket | 6 | 4 | 4 | 0 | 0 |
| feature Thickness | 5 | 2 | 1 | 1 | 0 |

- **A mirror plane, or a pattern's axis, on a feature's face or edge.** Read straight from that feature's stored
  shape, with no matching on our part:
  - the plane through the face's edge samples, when they are flat to 1e-7;
  - the line from a straight edge's first sample to its last.
  - All 5 that build match to 2.4e-12. The other 5 mirrors (one model's copies) now reach a fillet the kernel
    refuses (`Overflow`). The sixth axis body stops on a fillet edge it can't find.
- **Thickness** → `shell(t, open: FACES, out:)`.
  - The open faces are found as for Draft. `Reversed` (FreeCAD's "inwards", set on all 5) shells inward.
  - Only the skin mode.
  - 66344020.Body002 matches to 3e-15. 5e699f88.Body is 6.4e-4 off: a 1-thick shell over fillet faces, not yet
    explained.
  - Of the other 3: two stop on a pad up to a face, one on Mode 2 (a Thickness made by intersection).
- **Pad TaperAngle** was tried and not kept. FreeCAD's sign can't be measured in this set: one of its two bodies
  stops on a later cut, and the other doesn't translate. So it stays unsupported rather than guessed.

### What still stops FreeCAD (52 bodies)

| cause | bodies | `.fg` feature it maps to, or what is missing |
|---|---:|---|
| a profile not from a sketch | 7 | importer: Draft-workbench and binder wires as profiles (they read as stored wires) |
| a chain through PartDesign::FeaturePython | 4 | scripted features: no form, and they are never run |
| an empty body | 4 | nothing to make |
| a Part tree through / chain through Part::Loft, Part::Sweep, Part::Fillet, Part::FeaturePython | 7 | `loft`, `sweep` and `fillet` exist; importer: the Part workbench forms |
| a hole with its thread in the shape | 3 | `thread` exists; importer |
| sketch curve GeomEllipse / GeomArcOfEllipse | 3 | `ellipse`, `earc` exist; importer |
| a pad with TaperAngle | 2 | `extrude(.., taper:)` exists; importer, once FreeCAD's sign is measured |
| a pad or pocket of type 3 (up to face), a pad with Offset | 5 | `extrude(.., upto(FACES), offset)` exists; importer |
| a thickness of mode 2 (intersection) | 1 | importer |
| feature Boolean | 2 | `combine` exists; importer |
| a pipe from inside its path | 2 | two sweeps from the profile, as the CADFS reader already does for closed paths |
| a sketch with no closed region | 2 | importer: older sketches' wiring |
| a pipe through sections | 1 | **missing: a sweep through intermediate sections** |
| a path with a corner | 1 | **missing: sweep transitions at a path's corners** (kernel `Kink`) |
| feature AdditiveEllipsoid | 1 | **missing: an ellipsoid (or a scaled ball)** |
| an unequal chamfer | 1 | `chamfer(.., d2:, face:)` exists; importer |
| sketch of over 400 curves | 1 | the importer's own cap |
| other references (an edge no selection names 2, no edge where the base's edge lies 1, no face where the base's face lies 1, a pattern of a feature outside the chain 1) | 5 | importer |

## CADFS

All 1914 sampled parts:

| class | before | after |
|---|---:|---:|
| match | 1099 | 1111 |
| mismatch | 81 | 84 |
| refused | 85 | 92 |
| fail | 8 | 11 |
| timeout | 1 | 1 |
| untranslated | 640 | 615 |

**Draft on an extrude (24 → 0 under that name).** Onshape's extrude draft is Forge's `taper:`, on a depth. Its sign
was read from the STEP: with the pull-direction flag set, Onshape narrows the extrude as Forge's positive taper does
(00047196 matches to 9e-16). Without the flag it widens (00249812 1.6e-12, 00438509 9e-13).

| part | after |
|---|---|
| 00047196, 00249812, 00438509, 00717715, 00812539, 00091440, 00933092 | match (to 4.8e-5 or better) |
| 00165907 | mismatch, 5.1e-2 in volume: not yet explained |
| 00277536, 00515265, 00714299 | refused: `Fillet NoBlend`, `Sweep TooTight`, `Shell Collapses` |
| 00713562 | **fail**: a tapered cut whose draft is invalid (`EdgeDegenerate`) |
| 00981084 | **fail**: `Boolean InconsistentClassification`, at a cone, at feature 24 |
| 11 others | another cause after the draft (a region the query does not settle 3, a later feature's carrier 2, a depth other than BLIND 2, ...) |

Among the 476 parts with a fillet, 9 had this cause. 4 now reach the kernel, and all 4 are refused or fail: they are
kernel findings, not matches.

**Up to a curved face (19 → 0 under that name).** An extrude up to a face that isn't planar is now
`extrude(.., upto(FACES))`, the faces found on the query's carrier and named on the part as built. A depth, a
second direction or a draft along with it is still unsupported (Forge has no `taper` with `upto`).

| part | after |
|---|---|
| 00066533, 00147689, 00152806, 00202714, 00488382 | match (to 5e-13 or better) |
| 00127497, 00287804 | mismatch: 2.0e-4 and 4.1e-2 in volume |
| 00171925, 00442938 | refused: `Extent NoFace`, `NoNextFace` |
| 00680307 | refused: `Fillet Overflow` |
| 00850232 | refused: `Boolean ZeroThickness` |
| 00308391 | **fail**: `NotClosed` at the cut |
| 7 others | another cause (sketch text 2, unequal chamfer, entities that do not meet, no sketch record, a cap at an unknown depth, an add into several bodies) |

Both CADFS changes act only where the importer used to refuse. The merged run's every change of class is one of
those 43 parts.

### What still stops CADFS

Ranked for the parts with a fillet (all 1914 parts in brackets):

| cause | fillet parts (all) | `.fg` feature it maps to, or what is missing |
|---|---:|---|
| a region the query does not settle | 51 (119) | importer: the meaning of Onshape's `TD` weight on an imprint edge is not known (README: reading it as "the piece beside the vertex" gave 1 match and 16 wrong) |
| a surface extrude / a thin or surface extrude / a surface revolve | 19 (73) | **missing: surface bodies, and a thin-walled extrude** |
| sketch text | 17 (59) | **missing: text in a sketch** |
| a reference to an entity the sketch no longer has, imprint edge source / bounds nothing | 29 (110) | importer: queries naming entities the program no longer has |
| an unequal chamfer | 11 (23) | `chamfer(.., d2: | angle:, face: FACE)` exists. Tried and set aside: 15 of the 23 are single edges, where which face takes `width1` needs Onshape's per-edge rule. Reading it as the one face every edge bounds matched none of the rest. |
| a variable or conic fillet | 9 (9) | all 9 are conic: **missing: a conic cross-section fillet** (`vary` covers variable radii) |
| BLEND_EDGE / BLEND_FACE / OFFSET_* of an unknown feature, no edge or face on the carrier | 24 | importer: queries on blends' and offsets' own faces |
| feature deleteBodies | 4 (17) | importer (bodies dropped from the part) |
| sketch curve skEllipticalArc | 2 (6) | `earc` exists; importer |
| a pattern of many bodies, a mirror in a curved face, an axis on a solid's edge | 7 (18) | importer; a curved mirror has no form |

## WHUCAD

**A spline in a sketch (6 → 0).** CATIA's sketch spline (a `Spline` row and its `SCP` points) is written as Forge's
cubic through the same points, `spline([..], closed: false)`. None was taken on before. The other WHUCAD parts build
exactly as before (no volume moved).

| part | after (against CATIA's STEP, 1e-3 for the quantised data) |
|---|---|
| 100000495 | mismatch: volume 1.1e-3, area 7.3e-4 |
| 100001494 | mismatch: volume 1.6e-4, area 1.5e-3 |
| 100001782 | mismatch: volume 8.6e-2 |
| 100000526 | refused: `Shell Rebuild` |
| 100001734 | refused: `Fillet Overflow` |
| 100002274 | **fail**: `Boolean InconsistentClassification` |

None counts: CATIA interpolates its spline its own way, and the values are quantised. The two near ones differ by the
spline's shape, not by anything else in the part.

| cause still stopping WHUCAD | parts | maps to |
|---|---:|---|
| a face of Shell / Chamfer / EdgeFillet | 12 | importer: CATIA's names for blends' and shells' own faces |
| nothing selected, no such face, no edge where the faces meet | 7 | importer |
| an unequal chamfer | 4 | `chamfer(.., d2:)`; importer |
| a mirror in a curved face | 4 | no form |
| feature Draft | 2 | `draft`; importer (CATIA's selection) |

## Kernel and language findings from the newly translated parts

1. **Not a kernel bug: our writer.** A through-points `spline([..])` is closed unless it says `closed: false` (or has
   an end slope), as the reference says. fgw wrote open fit splines with neither, so a loop of lines and such a
   spline was a closed spline beside an open chain of lines: `SketchProfile: open chain`. fgw now writes
   `closed: false`. Profile chaining (forge-sketch's `chain_loops`) already joins ends within `tol::LINEAR`, and
   needs no change. The Lean model (research/forge-lang/lean) does not model sketch profile chaining or spline
   defaults: it models syntax, scope and placements.
   - That fix unblocks WHUCAD's splines (above).
   - It also let CADFS's fit splines be written exactly instead of as polylines. Tried on all 1914 parts, and not
     kept, because it regressed parts:
     - 00316638 went from match to refused (`Shell Rebuild`);
     - 00468533 and 00893869 went from off to a mesh failure (`ConstraintThroughPoint`);
     - 00553987 went from off to refused (a self-intersecting profile);
     - 4 refused parts became untranslated: the importer finds no edge on the spline faces' carriers.
   - It would have gained 4 (00261814 and 00527258 from off to match, 00533379 from fail to match, 00359539 from
     refused to built). For now CADFS's fit splines stay polylines, and only WHUCAD's are written exactly. The CADFS
     rows in the tables above are with polylines.
   - **Spline policy (the owner's direction).** The spline form is not chosen by which matches more of another
     kernel's parts. The definitive spec of `.fg` splines is being written in Lean (branch
     claude/repo-review-mbdrlj-spline-spec), and the kernel will be matched to it. The importers' rule will then
     be:
     - where the source gives control points, knots and weights (FreeCAD's B-splines), write them exactly, as now;
     - where it gives only fit points (CADFS, WHUCAD), write the spec's interpolant, with a typed note that the
       source's own interpolation may differ.

     WHUCAD's opt-in stays as it is until then.
   - **The rule as agreed with the spline spec** (branch claude/repo-review-mbdrlj-spline-spec):
     - Control data is written exactly: `spline(poles, degree:, knots:, weights:)`.
       - A periodic source has its poles wrapped (its n poles, then the first `degree` again). Its one period of
         knots is run on periodically to the full unclamped vector of n + 2·degree + 1. The curve's domain is the
         source's first to last knot.
       - An unclamped source keeps its own knots and poles. Only a clamped one has its end poles set to the loop's
         ends.
       - Both readers do this. The periodic form was checked against OCCT's own periodic curves (degrees 1 to 5,
         rational and not, first knots of multiplicity above 1: within 1e-9). The FreeCAD sample has none to gain:
         its 850 B-spline records are all clamped, and the readers give each the same output as before.
     - A fit-point source (WHUCAD's SCP; CADFS's `skFitSpline` where its opt-in is on) is written as the spec's
       centripetal C2 interpolant through the same points. Each one carries a typed note,
       `{"FitSplineReinterpolated": {"entity", "source"}}`: the source's own curve through those points may differ
       between them. Where the source gives end derivatives, they are written as `start_slope:`/`end_slope:` in
       the source's direction exactly. Their magnitude then means a rate per Forge's parameter, not the source's.
     - Through-points splines are to become open by default on the spec's branch. Until that merges, fgw keeps
       writing `closed: false`.
   - The regressions above are kernel or importer faults, not reasons to keep polylines:
     - Shell `Rebuild` → shell_revolved_spline.fg;
     - the mesh failures → mesh_wavy_spline.fg (below);
     - 00553987's `SelfIntersecting` is semantics. The source's own fit points cross the revolve axis at x = 0, and
       Forge's cubic through them reaches x = −0.6, so the refusal is right for that curve.
     - **The missing edges were the importer's.** A spline face's carrier measured distance to the sampled chords,
       and an exactly written spline's faces lie off them by the chords' sag. Now a carrier measures distance to the
       exact curve wherever the spline is written exactly (`sketch2d.carrier_dist`). That curve is the spec's
       centripetal C2 cubic, ported from forge-geom's `Spline::through`; it agrees with the kernel's faces to 3e-13,
       with end slopes and without. With fit splines written exactly, all 4 parts now translate (00123784 and
       00539691 now refused at a fillet, 00154651 refused, 00335925 built at 1.2e-3).
2. Two fails, restated in my own numbers as fixtures in interface/forge-lang/tests/kernel_parts, each with an
   ignored test in kernel_misc.rs that states what is due:
   - CADFS 00713562 → taper_collapses.fg: a hexagonal pit whose 80° taper meets before its depth builds an invalid
     body (`EdgeDegenerate`, a judge fail) where a refusal is due. At 70° it builds.
   - CADFS 00981084 → tapered_lobe_bore.fg: a 1°-tapered lobe pocket (an arc of radius 3 and two lines), and a bore
     of radius 3 whose wall runs within 0.15 of the lobe's cone. The boolean stops on `InconsistentClassification`
     (cone against plane). A smaller bore, or the lobe untapered, builds.
   - The tests are `a_bore_beside_a_tapered_lobe_builds` and `a_taper_that_collapses_is_refused_cleanly`.
   - In a search of my own, a ball cut over a 1°-tapered 30 by 10 rounded-rectangle pocket fails as `SplitCrossed`
     on the sphere → ball_over_tapered_pocket.fg.
   - The shelled spline cup (CADFS 00316638, WHUCAD 100000526) → shell_revolved_spline.fg. The wavy spline wall
     that builds but fails to mesh (CADFS 00468533, 00893869) → mesh_wavy_spline.fg.
   - WHUCAD 100002274 (`InconsistentClassification`) is **tolerance-scale**: a plate 0.0156 thick and a 0.0158 box
     on a plane tilted 1.4°. It is left for the decide-once near-coincident work, not reduced further. WHUCAD
     100001734 (`Fillet Overflow`) is a correct refusal: its bump stands 0.0016 above the rim, a quantisation
     sliver, too small a step for the 0.0078 round.
   - FreeCAD 50c1d207 (`Fillet Overflow`) → fillet_tangent_chain_scoop.fg: a fillet of 1.5 carried along a tangent
     chain (a concave arc, a straight side, a top arc) into an end face trimmed by a scoop. FreeCAD rounds it.
   - CADFS 00308391 (`NotClosed` at the cut) → drill_from_tangent_plane.fg: a drill down from a plane tangent to
     a round crown, to a bore's surface. Starting 1 above the crown, it builds.
   - The three fillets on exactly written spline walls (CADFS 00123784 `NoBlend`; 00539691 and 00154651
     `Overflow`) → fillet_spline_walls_noblend.fg, fillet_wavy_plate_notch.fg, fillet_crescent_both_edges.fg,
     left for the fillet spec to judge.
   - **CADFS 00850232 (`Boolean ZeroThickness`) is a correct refusal** → ring_cut_tangent.fg. A ring is cut
     through a block from its side, its outer circle tangent to the block's top and bottom. The two sides left
     meet only along the tangent lines. Onshape builds it, as a body that is non-manifold there. Per the owner's
     decision (ROADMAP, "Decisions"), Forge refuses non-manifold results, as mainstream mechanical CAD does. Its
     test, `a_ring_cut_tangent_to_two_faces_is_refused_at_the_tangent_lines`, is active and asserts that refusal.
3. Refusals newly reached:
   - CADFS 00277536 (`Fillet NoBlend`), 00515265 (`Sweep TooTight`), 00714299 (`Shell Collapses`);
   - FreeCAD d45d0db3.Body001 (`Extent Ambiguous` at feature 17, after its sweep).

## Rust (interface/forge-import) and Python (tools/find-parts/recreate)

Both readers have the exact splines, the split-edge match, Draft, the helix pipe as a coil, `Part::Extrusion` and
the loose blend's base. Their parts build to the same volumes (checked on five of the changed bodies, to 1e-12).

The sweep along a sketch's lines and arcs is in the Python reader only. It translated one body, which the kernel
refuses later.

The Rust reader writes a spline's degree, knots and weights as the sketch's params (`count(..)`, `ratio(..)`), so
its parts stay lint-free. New tests in interface/forge-import/tests/fcstd.rs use small documents written there, each
built and its volume worked out by hand: a rational spline side, an edge stored in pieces (both picked; one alone),
a draft, a helix pipe whose binder holds a different helix from the helix's own properties, a fillet outside a
body on a `Part::Extrusion`, and a periodic spline (its poles wrapped, its volume checked against the closed uniform
cubic's own area).

## Reproduce

As research/find-parts/recreate/README.md, with the importers of each tree. The tallies:

```
python3 -I research/import/tally.py freecad FREECAD/runs/before FREECAD/runs/after
python3 -I research/import/tally.py cadfs CADFS/runs/before CADFS/runs/after --truth CADFS/truth.jsonl [--fillet]
python3 -I research/import/tally.py whucad WHUCAD/runs/before WHUCAD/runs/after --truth TRUTH.jsonl
```

CADFS's truth is step_truth.py over every part either run built.
