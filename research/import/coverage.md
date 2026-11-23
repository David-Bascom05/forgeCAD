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
| FreeCAD library, bodies | 677 | 130 | 64 | 394 | 506 |
| CADFS, all parts | 1914 | 640 | 627 | 1099 | 1106 |
| CADFS, parts with a fillet | 476 | 238 | 234 | 164 | 164 |
| WHUCAD, 1 in 7 with a fillet | 319 | 35 | 29 | (1e-3: STEPs staged for the newly built only) | 0 new |

No part got worse on any set: no match was lost, and nothing that built before stopped building.

FreeCAD's matches rise by 112, and only 30 of those are newly translated parts:
- 30 refused before and match now;
- 52 matched only at 1e-3 before (a polyline stood in for a spline) and now match at 1e-4.

Both come from writing sketch splines exactly.

## FreeCAD

| class | before | after |
|---|---:|---:|
| match | 394 | 506 |
| mismatch | 95 | 78 |
| refused | 53 | 24 |
| fail | 5 | 5 |
| untranslated | 130 | 64 |

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

### What still stops FreeCAD (64 bodies)

| cause | bodies | `.fg` feature it maps to, or what is missing |
|---|---:|---|
| a profile not from a sketch | 7 | importer: Draft-workbench and binder wires as profiles (they read as stored wires) |
| a mirror in PartDesign::Pocket / Pad | 6 | `pattern(.., layout: mirror(on: PLANE))` exists; importer: the plane of a feature's face |
| a pattern axis on PartDesign::Pad / Pocket | 6 | `pattern` exists; importer: a feature's straight edge as the axis |
| feature Thickness | 5 | `shell(t, open: FACES, out:)` exists; importer: the open faces (`face_of` now finds them) |
| a chain through PartDesign::FeaturePython | 4 | scripted features: no form, and they are never run |
| an empty body | 4 | nothing to make |
| a Part tree through / chain through Part::Loft, Part::Sweep, Part::Fillet, Part::FeaturePython | 7 | `loft`, `sweep` and `fillet` exist; importer: the Part workbench forms |
| a hole with its thread in the shape | 3 | `thread` exists; importer |
| sketch curve GeomEllipse / GeomArcOfEllipse | 3 | `ellipse`, `earc` exist; importer |
| a pad with TaperAngle | 2 | `extrude(.., taper:)` exists; importer |
| a pad or pocket of type 3 (up to face), a pad with Offset | 3 | `extrude(.., upto(FACES), offset)` exists; importer |
| feature Boolean | 2 | `combine` exists; importer |
| a pipe from inside its path | 2 | two sweeps from the profile, as the CADFS reader already does for closed paths |
| a sketch with no closed region | 2 | importer: older sketches' wiring |
| a pipe through sections | 1 | **missing: a sweep through intermediate sections** |
| a path with a corner | 1 | **missing: sweep transitions at a path's corners** (kernel `Kink`) |
| feature AdditiveEllipsoid | 1 | **missing: an ellipsoid (or a scaled ball)** |
| an unequal chamfer | 1 | `chamfer(.., d2:, face:)` exists; importer |
| sketch of over 400 curves | 1 | the importer's own cap |
| other references (an edge no selection names, no face where the base's face lies, a pattern of a feature outside the chain) | 3 | importer |

## CADFS

All 1914 sampled parts:

| class | before | after |
|---|---:|---:|
| match | 1099 | 1106 |
| mismatch | 81 | 82 |
| refused | 85 | 88 |
| fail | 8 | 10 |
| timeout | 1 | 1 |
| untranslated | 640 | 627 |

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

### What still stops CADFS

Ranked for the parts with a fillet (all 1914 parts in brackets):

| cause | fillet parts (all) | `.fg` feature it maps to, or what is missing |
|---|---:|---|
| a region the query does not settle | 51 (119) | importer: the meaning of Onshape's `TD` weight on an imprint edge is not known (README: reading it as "the piece beside the vertex" gave 1 match and 16 wrong) |
| a surface extrude / a thin or surface extrude / a surface revolve | 19 (73) | **missing: surface bodies, and a thin-walled extrude** |
| sketch text | 17 (59) | **missing: text in a sketch** |
| a reference to an entity the sketch no longer has, imprint edge source / bounds nothing | 29 (110) | importer: queries naming entities the program no longer has |
| an unequal chamfer | 11 (23) | `chamfer(.., d2: | angle:, face: FACE)` exists. Tried and set aside: 15 of the 23 are single edges, where which face takes `width1` needs Onshape's per-edge rule. Reading it as the one face every edge bounds matched none of the rest. |
| up to a curved face | 10 (19) | `extrude(.., upto(FACES))` exists; importer: the face selection |
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
     refused to built). So CADFS's fit splines stay polylines, and only WHUCAD's are written exactly. The CADFS rows
     in the tables above are with polylines.
2. Two fails, restated in my own numbers as fixtures in interface/forge-lang/tests/kernel_parts, each with an
   ignored test in kernel_misc.rs that states what is due:
   - CADFS 00713562 → taper_collapses.fg: a hexagonal pit whose 80° taper meets before its depth builds an invalid
     body (`EdgeDegenerate`, a judge fail) where a refusal is due. At 70° it builds.
   - CADFS 00981084 → tapered_lobe_bore.fg: a 1°-tapered lobe pocket (an arc of radius 3 and two lines), and a bore
     of radius 3 whose wall runs within 0.15 of the lobe's cone. The boolean stops on `InconsistentClassification`
     (cone against plane). A smaller bore, or the lobe untapered, builds. WHUCAD 100002274 fails the same way.
   - The tests are `a_bore_beside_a_tapered_lobe_builds` and `a_taper_that_collapses_is_refused_cleanly`.
   - In a search of my own (a 1°-tapered rounded rectangle under a ball cut), a 30 by 10 pocket of corner radius 4
     also fails the boolean, as `SplitCrossed` on the sphere: a third internal error, not yet reduced.
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
a draft, a helix pipe whose binder holds a different helix from the helix's own properties, and a fillet outside a
body on a `Part::Extrusion`.

## Reproduce

As research/find-parts/recreate/README.md, with the importers of each tree. The tallies:

```
python3 -I research/import/tally.py freecad FREECAD/runs/before FREECAD/runs/after
python3 -I research/import/tally.py cadfs CADFS/runs/before CADFS/runs/after --truth CADFS/truth.jsonl [--fillet]
python3 -I research/import/tally.py whucad WHUCAD/runs/before WHUCAD/runs/after --truth TRUTH.jsonl
```

CADFS's truth is step_truth.py over every part either run built.
