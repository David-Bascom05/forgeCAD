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

Every count here is under the truth rule (research/import/truth.md), adopted after the first passes. A source's volume
is OCCT's Gauss–Kronrod one, and a source whose measures disagree is "uncertain", counted neither way. Both columns are
judged against the same re-measured truths: the source shapes are the same before and after. The counts under the old
measure are given beside them where they differ.

## Headline

| set | parts | untranslated before | after | match (1e-4) before | after | uncertain after | match after, old measure |
|---|---:|---:|---:|---:|---:|---:|---:|
| FreeCAD library, bodies | 677 | 130 | 39 | 394 | 570 | 20 | 523 |
| CADFS, all parts | 1914 | 640 | 595 | 1108 | 1136 | 4 | 1122 |
| CADFS, parts with a fillet | 476 | 238 | 223 | 164 | 167 | 1 | 168 |
| WHUCAD, 1 in 7 with a fillet | 319 | 35 | 29 | (1e-3: STEPs staged for the newly built only) | 0 new | | |

No part got worse on any set through the importers: no match was lost, and nothing that built before stopped
building. One match was lost to the rule itself. 5ab390cb.Body001 is an exact ellipse pad; both its volumes agree,
but its two areas do not (6e-5), so it is uncertain.

The first pass took on the four biggest FreeCAD causes and CADFS's extrude draft. A second pass took on causes that
need only importer work: FreeCAD's Thickness, a mirror plane on a feature's face, a pattern axis on a feature's edge,
CADFS's extrude up to a curved face, and WHUCAD's splines. A third pass writes sketch ellipses and elliptical arcs
exactly, reads FreeCAD's pad or pocket up to a face, takes a profile that is a binder or a feature's edge or
face, reads CADFS's elliptical arcs, and takes CADFS's pattern and revolve axes from the part's own edges and
faces, and reads its rotating and line-following transforms.

In the first pass FreeCAD's matches rose by 112, and only 30 of those were newly translated parts:
- 30 refused before and match now;
- 52 matched only at 1e-3 before (a polyline stood in for a spline) and now match at 1e-4.

Both come from writing sketch splines exactly.

## FreeCAD

| class | before | after |
|---|---:|---:|
| match | 394 | 570 |
| mismatch | 95 | 12 |
| uncertain | 0 | 20 |
| refused | 53 | 30 |
| fail | 5 | 6 |
| untranslated | 130 | 39 |

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

### The third pass: ellipses written exactly

A sketch's ellipse is `ellipse(center:, major:, b:)` and an elliptical arc is `earc(.., from:, to:, cw:)`. Each is
written exactly while the arrangement leaves it whole. A stretch of one that the arrangement cuts is still a polyline,
and the part is then approx. FreeCAD's records give the major axis at `AngleXU`, with the minor axis turned a quarter
the normal's way; an arc's angles are the ellipse's parameters. Both readers do this, and on these bodies their
volumes agree to 1e-12.

| part | before | after |
|---|---|---|
| FreeCAD 5ab390cb.Body, .Body001 | untranslated (`GeomEllipse`) | match: 2.2e-10 and 2e-15 in volume |
| FreeCAD 82d7764f.PolarPattern004 | untranslated (`GeomArcOfEllipse`) | match: 5.9e-5 in volume, 8.8e-6 in area. The residual is the truth's measurement, not the part (below). |
| CADFS 00737513 | approx, 8.0e-5 / 1.0e-4 | match: 2.3e-5 / 6.5e-8 |
| CADFS 00298092 | approx, a match | a match, no longer approx (2.3e-11) |
| CADFS 00340940, 00879304, 00910674 | approx, 5.6e-4 to 6.1e-4 off | 1.0e-4 to 1.6e-4 off: **the STEP's ellipse is the approximation**. 00340940 is a 25 by 12.5 ellipse extruded 76.2. We build π·25·12.5·76.2 = 74809.175 to 1e-15; Onshape's STEP measures 74797.390. Reported, not counted. |

The other 21 CADFS parts with an ellipse come out as before: one already matched (00472699), two are refused at a
fillet, and 18 stop on another cause.

**CADFS's elliptical arcs.** The FeatureScript record of an elliptical arc holds no geometry
(`skEllipticalArc(sketch, "E0", {})`); the solved curve is only in the dataset's sketch record, under a UUID. The
reader now takes the one unused sketch record whose entities are the program's, kind and construction flag in order,
and reads each arc from it by position. Each line's ends must be that record's too, to 1e-6. The arc's centre, major
axis and parameter range are recorded. Its two semi-axes are solved from the points recorded at the start, a
quarter, half way and the end, and a fit off by more than 1e-9 is refused.

| part | after |
|---|---|
| 00565164 | match: 3.1e-6 / 6.6e-6 |
| 00869959 | match: 4.9e-6 / 8.5e-6 |
| 00044536 | refused: an intersect that comes out empty (feature 9), past the arc |
| 00541379 | untranslated: sketch text, past the arc |
| 00124542, 00588035 | untranslated: no one sketch record has the program's entities (the arc stays unread) |

### The truth's own measurement error

82d7764f's 5.9e-5 was traced feature by feature against FreeCAD's stored shapes. Each matches ours until Fillet002,
a radius-2 fillet along the elliptical cut's edges: there the stored shape is 2.0 smaller in volume than ours. That
2.0 is not in the geometry. It is the gap between two ways OCCT measures the same stored shape:

- `BRepGProp::VolumeProperties` as the truth tools call it (no precision given) gives 27743.68;
- with a precision of 1e-9 it gives 27745.69, ours to 1e-6.

A part of my own settles which is right. A 100 by 60 by 10 block has an elliptical hole (20 by 10), and both of the
hole's edges are rounded at 2. The rolling ball between a plane and a wall square to it removes (1 - π/4)r² of
section, at a centroid x̄ = r(10 - 3π)/(3(4 - π)) from the edge. By Pappus the volume is therefore exactly
V = block - hole - 2(1 - π/4)r²(L + 2πx̄), with L the ellipse's perimeter: 53545.663021384.

| measure | value | off |
|---|---|---|
| Forge's kernel | 53545.663021377 | 1.3e-13 |
| OCCT's fillet, measured as the truth tools do | 53545.683154 | 3.8e-7 |
| OCCT's fillet, measured with a precision of 1e-9 | 53545.663563 | 1.0e-8 |
| OCCT's fillet, Gauss–Kronrod (1e-9) | 53545.662984 | 7.0e-10 |

The default measure is not wrong everywhere, and the precise one is not right everywhere. On 5ab390cb.Body001 (an
ellipse 20 by 15 padded 0.4, exactly π·20·15·0.4 = 376.991118) the default is exact, while the precise call gives
376.990968 and moves the area by 6e-5. OCCT's Gauss–Kronrod integrator (`VolumePropertiesGK`, 1e-9, spans on) is a
third, independent measure. It agrees with the right one in all three cases above: 376.991118, 26028.8318 (82d7764f,
ours 5e-8 off), 53545.663 (my part, 7e-10).

**This reaches beyond ellipses.** Of the 79 FreeCAD bodies counted as mismatches, the two OCCT measures of the
stored shape disagree by over 1e-5 on 68. Against the precise one, 48 of the 79 are within 1e-4. They are mostly 5
bodies of one model (Body020, 024, 027, 028, 031) copied in 11 files, off by 1.0e-4 to 1.5e-4 under the default
measure and by 1e-7 to 4e-6 under the precise one. On the other 20 where they disagree, the default is the nearer to
ours (Body030 and 033 of that model, and c106d15f.Body): there the precise measure is off by up to 1.2e-2. The CADFS STEP truths are measured in the same way.

Choosing, part by part, whichever measure agrees with ours would be no test, so the rule was fixed first, and the
owner adopted it (research/import/truth.md). Both sets were then re-measured with the truth tools themselves. The
importers' output was the same in both runs, built for built and volume for volume; only the truths moved.

| set | class | old measure | the rule |
|---|---|---:|---:|
| FreeCAD | match | 520 | 567 |
| FreeCAD | mismatch | 79 | 12 |
| FreeCAD | uncertain | | 20 |
| CADFS | match | 1114 | 1126 |
| CADFS | mismatch | 83 | 68 |
| CADFS | uncertain | | 3 |

(FreeCAD here is before the profile pass below, which adds 3 matches.)

- **FreeCAD:** 48 mismatches become matches, 19 become uncertain, and 1 match becomes uncertain.
  - The 19 are Body030 and Body033 of one model, copied in 10 files. Their default volume is 1.3e-4 from the
    Gauss–Kronrod one and their precise volume 1.2e-2, so neither agrees.
  - The 1 is 5ab390cb.Body001.
- **CADFS:** 12 mismatches become matches and 3 become uncertain (00027022, 00294728, 00659354). The fillet subset is
  unchanged.
- **The copy rule** (truth.md, adopted after this), for CADFS parts whose STEP holds exact copies:
  - 3 mismatches become matches (00293826, 00724634, 00846949);
  - 1 match becomes uncertain (00647071, a part with a fillet, so the fillet subset goes from 167 to 166).

  The headline is under both rules.

### Up to a face (FreeCAD, 5 → 0)

A pad or pocket of type 3 runs up to a face of an earlier feature. That face is found on our part by its edges, as for
Draft and Thickness. Where it is a plane square to the extrude, as in all 5 here, it becomes a depth from the
sketch's plane. `Offset` is read as moving the face along the extrude's way. That reading was measured, not assumed:
16233ee2's two pockets, each with an Offset of 1, match to 8e-16. A face that is not square to the extrude stays unsupported. `upto(FACES)`
would take it, but no body here has one to check it by.

| part | after |
|---|---|
| 758792f7.Pocket003, 77b19abc.Body, 859a5a2f.Body, b2f17299.Body, 16233ee2.Body | match: 1.9e-12, 2.6e-10, 2.6e-10, 5.2e-11 (area 1.4e-9), 8.3e-16 |

**No kernel gap in a two-sided pocket.** The Rust reader used to write a two-sided pocket as one depth from its back
end, a workaround its comment put down to the kernel refusing a two-sided cut. Nothing here reproduces that refusal.
A block cut by `two(a, b)` with every sign, from its face and from a sketch inside the material, builds, and so does
77b19abc's pocket written either way. The workaround is gone, and both readers now write `two(..)`. A new test of
my own (a pocket of type 4 with Lengths 6 and 1) holds the result to its volume.

77b19abc still differs between the readers by 8e-9, and both match. The cause is earlier, at t4: two near-half-circle
arcs whose centres the two readers refit 7.6e-6 apart (by a half circle the ends barely fix the centre).

### A profile that is a binder, or an edge or face of a feature (FreeCAD, 7 → 0 under that name)

A pad or pocket's profile can be a ShapeBinder, or one edge or face of a feature (`Pocket027` `Edge200`, `Fillet006`
`Face4`). Its wires are read from the stored shape, as a Draft object's already were. The plane it lies in comes from
those wires: through their first sample, with x towards the farthest sample. Such a profile has no side of its own, so
the way it cuts is the feature's stored Direction, turned by Reversed. That reading was measured, not assumed:
1833f02f's Pocket028 is the one such feature stored reversed, and it cuts only that way. A profile without a stored
Direction stays unsupported.

| part | after |
|---|---|
| 1833f02f.Body008, 1fe89907.Body, d45d0db3.Body007 | match: 1.3e-12, 4.2e-16, 1.9e-12 |
| 1833f02f.Body009 | refused by the Python reader's file (`Fillet Overflow`); the Rust reader's builds and matches to 6e-10 (below) |
| 1833f02f.Body023 | **fail**: `Boolean Internal(RetryInvalid)` at the pocket from `Edge200` |
| b709cdfa.Body | untranslated: a pocket from a face with no Direction stored |
| d45d0db3.Body016 | untranslated: a blend along a spline, past the profile |

**Two kernel findings from these.**

- **1833f02f.Body023 → drill_coincident_lug_hole.fg.** Its pocket (a circle of radius 2 cut 15 along the stored
  direction) is a judge fail, `Internal(RetryInvalid)`. Delta-debugged and restated in round numbers:
  - a tube, counterbored;
  - a slot-shaped lug on its side with a hole of radius 2;
  - the counterbore cut again through the lug;
  - a drill of radius 2 down that hole, its wall coincident with the hole's.

  Moved 0.3 off the hole, or without the second counterbore, it builds. The test is
  `a_drill_down_a_lug_hole_builds` (ignored, open).
- **lug_join_loops_cross.fg**, seen while reducing that one. A slot-shaped lug, extruded 15 into the side of a solid
  cylinder, joins to an invalid body (`LoopsCross`), though its sketch alone extrudes. Extruded 10, or with the second
  arc of radius 3.5 or 4.5, it builds. The test is `a_lug_joined_to_a_cylinder_builds_valid` (ignored, open).
- **1833f02f.Body009 → fillet_flush_notch.fg and its twin.** Its fillet builds from the Rust reader's file and is
  refused (`Overflow`) from the Python reader's. The files name the same two edges at the same radius (2.2), and the
  part before the fillet has the same volume in both, to 1.6e-12. They differ only in how the numbers are written:
  Rust rounds them to ten decimals and sets its sketch planes elsewhere, and Python writes every digit.
  - Delta-debugged, the edges are the upright ones where a notch in a boss meets a wall flush with the disc under it.
    The boss's wall and the disc's are one face.
  - Restated in round numbers (a disc of radius 16, the boss's notches of radius 2.75), a round of only 0.05 is
    refused as Overflow at one notch, while the same round at its mirror twin builds.
  - The printed-digit flip itself did not survive the restating: with the sketch solved, 10 to 12 decimals give the
    same verdict. The mirror pair shows the same fault, a verdict that turns on rounding and not on the shape.
  - Tests: `a_round_at_a_notch_in_a_flush_wall_builds_like_its_twin` (ignored, open), and
    `the_flush_notch_twin_rounds` (active), which keeps the pair a pair.

### What still stops FreeCAD (39 bodies)

| cause | bodies | `.fg` feature it maps to, or what is missing |
|---|---:|---|
| a chain through PartDesign::FeaturePython | 4 | scripted features: no form, and they are never run |
| an empty body | 4 | nothing to make |
| a Part tree through / chain through Part::Loft, Part::Sweep, Part::Fillet, Part::FeaturePython | 7 | `loft`, `sweep` and `fillet` exist; importer: the Part workbench forms |
| a hole with its thread in the shape | 3 | `thread` exists; importer |
| a pad with TaperAngle | 2 | `extrude(.., taper:)` exists; importer, once FreeCAD's sign is measured |
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
| a pad from a shape with no direction stored | 1 | importer: a face profile's way, where the feature stored none |
| a blend along a spline | 1 | importer: a spline written as a polyline under a fillet (past a binder profile) |

## CADFS

All 1914 sampled parts:

| class | before | after |
|---|---:|---:|
| match | 1108 | 1136 |
| mismatch | 69 | 69 |
| uncertain | 3 | 4 |
| refused | 85 | 98 |
| fail | 8 | 11 |
| timeout | 1 | 1 |
| untranslated | 640 | 595 |

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

**An axis on the part (13 → 2 under that name).** A circular pattern's or a revolve's axis can be one of the part's
own edges or faces rather than a sketch line. It is now read from the part as built:
- a straight edge: the line through it;
- a cylinder's face or a circle's edge swept from a sketch: the sketch's normal through the circle's centre;
- a revolved cylinder or cone: the revolve's axis;
- a single closed circle otherwise: the normal through the circle its samples lie on.

Which way Onshape runs such an axis is not known here. So it is taken only where the way cannot matter: a full or
symmetric revolve, or a pattern spread evenly over a whole turn.

| part | after |
|---|---|
| 00501978, 00718900, 00754089 | match (to 2.4e-13 or better) |
| 00677337 | refused: `Fillet Corner` |
| 00670276 | mismatch: 1.5e-4 (approx: a polyline stands in for a spline) |
| 00293826, 00724634, 00846949 | match under the copy rule (truth.md): 4.5e-9, 2.5e-13, 5.1e-5. Each is a pattern of new bodies that spins each copy onto the last, and the STEP holds the same copies. |
| 5 others | another cause (a surface revolve, a pattern's copy 2, more than one edge 2) |

**Transforms (11 → 0 under those names; 4 scales stay).** A transform of type ROTATION is a `spin` about its axis,
right-handed about the axis's way. A sketch line runs from its start, as the circular patterns read it; an axis on
the part counts only at 0° or 180°. TRANSLATION_DISTANCE is a `slide` along a line by the distance.
TRANSLATION_ENTITY slides by a sketch line itself, from its start to its end. Each is turned by its opposite flag.

| part | after |
|---|---|
| 00106358 (45°), 00379141 (135°), 00443458 (30°) | match: 2.2e-16, 1.9e-15, 3.8e-6 |
| 00455425 | refused: `NoBody` at a later combine |
| 4 others | an axis on a solid's edge at an angle that needs its way (4) |
| 00575516, 00646531 | a reference of many (the axis or line named by two queries) |

**The rotation's sense is not measured by these.** Turned the other way, all three matching parts come out with the
same volume, area and box, so the reading of the sense rests on the circular patterns' and is still open. A part
where the sense changes the shape would show it as a mismatch, not as a false match. SCALE_UNIFORMLY (4 parts) has
no form. **Language gap: a scale motion** for `movebody`/`copybody` (00194324, 00371302, 00593192, 00743818). It is a
language change, so it needs the Lean model extended, and is left for the language work.

**A disambiguation naming an entity the sketch no longer has (6 → 0 under that name).** A query's disambiguation can
list, beside the entities that bound a face, one the sketch no longer has. These are UUIDs found nowhere in the
program or its sketch records, ids from an older draft. They are now passed over where the entities the sketch still
has remain, and a choice those leave open stays unresolved further on. Such parts are noted `stale-disambiguation`.

| part | after |
|---|---|
| 00595973 | match: 1.1e-11 |
| 00077264, 00098356 | refused: `Shell Collapses`, `Shell Rebuild` |
| 00005492, 00091915, 00910163 | another cause further on (OFFSET_FACE of an unknown feature, imprint edges 2) |

A full sweep of all 1914 parts, against the one before, moves only these six. One part moves within its class:
00035608, a mismatch, comes nearer, from 1.6e-1 to 3.7e-5 in volume (1.5e-4 in area). It too had a stale entity
there, which had sent its query down a fallback.

**A fillet's own faces and edges (12 → 2 under "of an unknown feature"; 7 more stop on narrower causes).** A BLEND_FACE
query names the round's face along one rounded edge, and a BLEND_EDGE names where that face meets another
(`blendedInto`). When the reader writes a fillet, it now keeps the samples of the edges it rounds and the faces that
were there before. On a later query:
- the rounded edge is the one the query's `blendedFrom` edge names;
- its face is the one, of the faces the fillet made, lying nearest that edge (by its samples' mean distance), and
  clearly nearest, at half the next one's distance or less;
- the BLEND_EDGE is what that face shares with the `blendedInto` faces.

| part | after |
|---|---|
| 00582591 | match: 1.8e-13 |
| 00026369 | refused: `Fillet NoBlend` |
| 00339994 | sketch text, further on |
| 00079785, 00608424, 00770496 | BLEND_FACE with no edge it came from (its `blendedFrom` empty) |
| 00310838, 00531768 | no face of the round clearly nearest the edge |
| 00259389, 00834324 | no edge into the face, or not one rounded edge |
| 00632978, 00756896 | the blend of a feature the reader does not record (not a fillet) |

A full sweep of all 1914 parts moves only these, and no volume.

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
| sketch curve skEllipticalArc | 1 (2) | read from the dataset's sketch record where one record has the program's entities (above); in these 2, none does |
| a pattern of many bodies, a mirror in a curved face | 5 (13) | importer; a curved mirror has no form |

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

Both readers have the exact splines and ellipses, the split-edge match, Draft, the helix pipe as a coil, `Part::Extrusion` and
the loose blend's base. Their parts build to the same volumes (checked on five of the changed bodies, to 1e-12).

The sweep along a sketch's lines and arcs is in the Python reader only. It translated one body, which the kernel
refuses later.

The Rust reader writes a spline's degree, knots and weights as the sketch's params (`count(..)`, `ratio(..)`), so
its parts stay lint-free. New tests in interface/forge-import/tests/fcstd.rs use small documents written there, each
built and its volume worked out by hand: a rational spline side, an edge stored in pieces (both picked; one alone),
a draft, a helix pipe whose binder holds a different helix from the helix's own properties, a fillet outside a
body on a `Part::Extrusion`, and a periodic spline (its poles wrapped, its volume checked against the closed uniform
cubic's own area), two elliptical arcs of opposite normals around an elliptical hole, a pocket up to a face with
an Offset, a two-sided pocket, and a pocket whose profile is a face of the pad.

## Reproduce

As research/find-parts/recreate/README.md, with the importers of each tree. The tallies:

```
python3 -I research/import/tally.py freecad FREECAD/runs/before FREECAD/runs/after
python3 -I research/import/tally.py cadfs CADFS/runs/before CADFS/runs/after --truth CADFS/truth.jsonl [--fillet]
python3 -I research/import/tally.py whucad WHUCAD/runs/before WHUCAD/runs/after --truth TRUTH.jsonl
```

CADFS's truth is step_truth.py over every part either run built.
