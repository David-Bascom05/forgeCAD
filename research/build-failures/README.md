# Why parts will not build

The census of what ForgeCAD refuses or gets wrong, ranked, and which refusals are right. Kept up to
date as classes are fixed.

## How to rerun

- History census: `cargo run --release -p forge-agent --example buildfail -- 0 100 12 OUT_DIR [all]`:
  every feature kind's tries and refusals over random histories, every message counted, the
  smallest history of each shrunk (`all`: every history as found, for OCCT).
  `buildfail shrink F.fc` shrinks a part file's refusal.
- A refused finishing feature's message says whether size is the reason: `; it builds up to radius
  R`, or `; no smaller radius builds either`. `Model` tries the feature smaller (`size_limit`).
- OCCT on the refusals: `oracle dir:OUT_DIR > parts.json`, then `tools/occt/oracle.py parts.json`. A part file's
  refused line is exported too, so `ours-error` with OCCT `ok` is a refusal OCCT does not share.
  OCCT's own results still need judging (`--tiebreak`, Monte Carlo).
- Scoreboard: `cargo run --release -p forge-model --example reliability -- --show`.
- Real parts: `tools/find-parts` (Fusion 360 Gallery timelines, sketch + extrude; Fusion's volume
  is the truth). Its data is non-commercial research only: tests here restate geometry in their
  own numbers.

## Should have built, ranked

Counts: real parts = find-parts, Fusion 360 Gallery, ~8600 parts, main d33b84aa built 91.2%; history
= 100 seeds, 1685 lines tried; scoreboard = 841 parts; pipe = 200 traced-spine fillets cut through
(research/pipe-speed/fuzz/gen.py), 33 valid before the fence.

| class | count | root cause | blocks | state |
|---|---|---|---|---|
| butted cylinders of one radius | 237 real parts | a closed edge two bodies share: its direction was read from its ends, one vertex, so one use ran the wrong way: open shell, or false ZeroThickness | shafts and tubes built in steps from a joint | fixed (`canon` in boolean.rs, nary.rs) |
| crossing a traced fillet's spring missed | 97 of 200 pipe parts; now 140 of 200 valid | `across()` used a plane at the edge's middle for a curved spring, the neighbour being tangent there | any cut through a fillet on a non-circular edge | fixed (`Fence` in blend.rs, plus the spring against the other surface) |
| lost cut, silent wrong volume | 3 real parts, wrong with no error | a stretch whose middle lands within tolerance of a boundary with its ends on that boundary was taken to run along it, and no edge was made | any model carrying float noise | fixed (`inside` in `trim`: along an edge only if that edge holds its middle and both ends). Noise check, 8188 parts at 3e-7: no volume moved over 0.01% |
| mass silently wrong on an intersection curve with a tip | 8 of 300 ruled lofts (research/pipe-speed/fuzz/gen_ruled.py), up to 1%; 0 of 8420 real parts (extrudes only) | a plane across a V cuts the curve twice: a piece made finer solved Newton from a far seed and landed on the other arm, as exact as the right one, so no gap and no error estimate saw it; and a chain step holding the whole turn at the tip, where the chord distance doubles back. The second reaches every intersection curve, the first any procedural surface against an analytic one | any face of a loft cut by a cylinder that grazes a corner | fixed in `kernel/forge-topo/src/mass.rs`; forge-proof `mass` holds it |
| through cut short on a loft, sweep or coil | silent, wrong volume; CADFS 00098048 at 7.5% | the reach a through cut sizes itself by counted extrudes, revolves and balls only | any `thru` after a loft or sweep | fixed: `hole::reach`, bounds without building |
| arc ending on the axis, centre across it | 58 real parts, invalid | the signed centre was clamped to 0: a sphere | a lemon or nose | fixed; apple and lemon build as a torus reaching through its axis; booleans on one refused (98 + 6 parts stop there) |
| classification floods across a shared edge | part of 76 real parts | a tool's edge on a part's edge: being inside the tool changes across it | pockets ending on a rib's root | fixed |
| nearly flush surfaces | part of 76 and 65 real parts | plane on plane or tangent to a cylinder within LINEAR over the faces, tilted past `ANGULAR` (9-digit rounding), crossing in curves the tolerance cannot tell apart | imported or rounded data | fixed (`ssi::nearly_flush`) |
| probe on a touch | part of 65 real parts, 6 scoreboard | the one probe of an untouched face lands on a tangent line; also a whole edge imprinted because its middle touched | cross holes ending on a tangent plane | fixed, except a tool's corners touching a circle (repro e6ca2) |
| flat ends named alike | 46 real parts | flush vertices were not ranked by place: DuplicateName | a flat by an arc on the rod's own circle | fixed |
| next face past a part-changing feature | 35 history | refused outright | `next` after any fillet | fixed for one body; multi-body still refused (23): the extent does not see each feature's pick |
| pinch at a vertex in a face split | 22 scoreboard | own edges tangent at a vertex never counted as a tie | balls touching on a face | fixed; a ulp nudge of it is invalid (tools/proof/known.txt) where it was refused |
| fillet off over an edge and back | 11 refusals, 8 valid of 200 pipe | the spring leaves its face over one edge and returns over it; the edge's middle goes | a boss near a rim | fixed (`SplitFirst`, `chain_loops`) |
| shell towards a cone's point | part of 15 history | a pointed cone was taken to collapse | a pointed boss shelled in, a drill point shelled out | fixed |
| hair kink in a straight side | noise repro; 162 noise failures carry this signature, how many this root is not known | a neighbour at a shallow angle loses the crossing | noisy imports | fixed for that repro (`across`) |
| fillet Corner, pipe (now the fillet agent's) | 28 of 200 (13 at any radius) | no rule closes the vertex: `ball_setbacks`, a cold edge with no trim point, a seam the spring does not cross | concave chains across a box's convex edge | open |
| fillet Overflow, pipe (now the fillet agent's) | 14 of 200 | odd crossings of the body's edges (a chain across a convex edge) and others | same | open |
| classification, pipe | 11 of 200 | a curve's crossing of a spring or rim still missed | flats lying along a contact | open |
| fillet at any radius, history: NoBlend 28 (mostly variable radius), Overflow 18, Corner 20 | 66 of 202 | a variable law through a line and an arc: the guide (a spline) bends on the line's long step | variable fillets past a filleted corner | open: drawing knots in moved NoBlend to BadRadius only, dropped |
| chamfer at any size: TooLarge 12; Unsupported 13 | history | not traced | chamfering a tilted hole's rim | open |
| shell at any thickness, other: Rebuild 8 | history | not traced | | open |
| n-ary refuses several bodies in `finish` | 37 history | the history check called the one-body n-ary regen: a test gap | | fixed: every body on both regens, none differ in 34 seeds |
| finder's synthetic list | 62 of 69 still fail | threads, coils and lofts against other tools; direct edits; see repros/find-synth | | open |

Fusion rate: 95.0% (8194 of 8625, the 2 pinched finals counted correct) on 7bc265a2, against 91.3% on
main d33b84aa; 36 wrong-volume parts, most multi-body translation, 6 new and unjudged. Top refusal: 113
ZeroThickness, some of them false (fixes on fusion-1), some true pinches main built unknowingly. Noise on
3b1d1c47: 309 of 8188 built parts stop building at 3e-7 (162 classification, 49 EdgeUses, 47
ZeroThickness), no volume moved over 0.01%.

## Fusion 360 Gallery, what changed the rate

`tools/find-parts/realparts.py` against every design: 91.2% to 98.3% so far. Most of the first wrong volumes
were the importer. Volumes are compared with the dataset's; a design's own STEP (read by OCCT, `tools/find-parts/stepvol.py`)
settles a disagreement.

| class | parts | root cause | state |
|---|---|---|---|
| join merged nothing | most of 33 wrong | importer: a join adds to the bodies Fusion names and takes in those it says went; a region to the body whose faces lie in it | fixed, importer |
| profile off its sketch's plane | 8, 6 untranslated | importer: sketch z varying means the profile lies in a plane of its own | fixed, importer |
| tapered extrude | 129 untranslated | no form: `extrude N taper A`, an extrude drafted, or slabs between heights where the offset wipes a segment out (`forge_ops::taper`) | fixed; 4 parts open: a reflex vertex meeting another side splits the region (not written), one collapse |
| elliptical arcs without ends | 22 untranslated | ends from neighbours, the side from the feature's wall | 12 fixed; 10 say too little: **the dataset omits them** |
| false ZeroThickness | 16 of 28 sampled | faces touching tangent, facing the same way, each ending at the line: one skin runs on (`flows_on`). Flush below and parting above: a groove rooted there (`crack_tip`) | fixed |
| knife edge in a profile | 30 of the failures held one | a line and arc doubling back, within tolerance of each other for a stretch. Blunted where the sides part, only when the boolean refuses the exact tool (a tip may meet other faces exactly) | partly: fails when a later feature is what trips on it |
| faces merged off their surface | 7 (n-ary) | "on one surface" is not transitive: unify keeps pieces within a tolerance of the one kept | fixed |
| hole's probe past its host | | a step of 1e-3 of a long segment in v is wide in u: a fraction of the hole's width on curved faces | fixed |
| rim tangent to a flush face's edge | | imprint's middle sample on the boundary only touched it | fixed |
| cross hole tangent to a tube | | a curve leaving an edge at a shallow angle "crosses" anywhere along a stretch: at a neighbouring cut, or one vertex for out and back | fixed |
| chain of cuts within tolerance | | a vertex near two cuts not near each other ended in two vertices | fixed |
| seam that steps, hole on it | | a merged bore's span exceeds a period; a loop's window chosen from the widest loop, by the outline | fixed |
| curve reaching a smooth edge | | it grazes what lies across: a cut if it leaves the face there | fixed |

Open, by signature: the numbers in the last `fusion-N ready`. `verify/forge-proof/examples/pinch.rs` counts a
ZeroThickness result's material on a sphere about the point; a tangent contact's gap can fall between its
samples, so one piece there is not proof it is false: the shrunk part decides.

## Correctly refused

David's rule: what cannot exist is the solid, not the sketch. A profile's loops may touch; the
result decides.

| refusal | count | why it is right | message |
|---|---|---|---|
| Empty, whole part | 76 scoreboard | nothing left: OCCT's volume is 0 on all 76. The scoreboard counted these as failures | clear |
| ZeroThickness, one solid pinched | 2 real finals; 102 scoreboard | material meets itself along a line or at a point. Some real-part refusals are false (same-facing faces, on fusion-1) | says where, and the line |
| fillet Overflow, chamfer TooLarge, shell Rebuild, when smaller builds | history, pipe | the size does not fit, or a rule above is missing; OCCT builds some | gives the size that builds |
| generator misses: `NotOnFace`, `NoEntity`, `NotCrossed`, `NeverHits`, `BadSource` | ~75 history | the random line asked for something absent | could say more |

A cut taking one whole body of several deletes it (David); `build` names the body and the line.

## Open

- A profile whose loops touch twice divides into separate solids: one body's faces cannot hold that
  (6 real parts, with afd074a3fe89393ef). Refused with the point until then.
- Pinched only at an intermediate step (147009_977ee378_0000): refused; raised with David.

## Not done

The OCCT replay of all 495 refused histories did not finish, so "any size" for the history classes is
measured by trying smaller, not yet by OCCT. On the pipe set OCCT did run: 142 agree, 3 differ in one
surface's area, 23 OCCT fails too or is invalid, 31 we fail and it builds, 1 does not mesh (138 valid then; 139 now).

## Volume checks

forge-proof properties, tests only, deep tier. `volume`: each cut or add's volume change against 1200 points
tested against the part before and the tool's solids, no boolean involved; 5 sigma of the sampling (a point on a
boundary counts half, or a tool a few microns thick loses its share). `mass`: mass against two meshes a quarter of
the sag apart, the volume and each face's area, the slack from the meshes' difference and the fine mesh's sag. Both
were set by running them on the Fusion parts, the bench parts and fuzz seeds; every disagreement left was traced by
a third, finer mesh or by hand. `verify/forge-proof/examples/propdir.rs` runs either over a directory.

## Fusion 360 Gallery, second batch

Each from a failing part, shrunk, then restated in own numbers in `interface/forge-agent/tests/fusion_parts.rs`:

- `check.rs`, vertex tolerance: 1.1 x LINEAR. Six parts had a vertex grown 1e-14 to 9e-10 mm past LINEAR by rounding, next to an edge under two tolerances long. The same edge is legal at LINEAR exactly, so the rule stepped on rounding. 137053 is 1.7e-7 mm past (1.017 x LINEAR), so 1.001 was not enough.
- `plane_cylinder`, lines under cos 1e-6: a wall 1e-6 off over 21 mm gave a 25.4 radius cylinder an ellipse millions of radii long (41032). Error of the tangent lines is (t cos)^2 / r: 4e-8 at t = 1000, r = 25.
- `clear_step`, floor 2 x LINEAR, halving: the floor of a D-hole 8e-5 wide (137053). Undoing it changed instructions on patterns, coplanar and housing by under 0.03%.
- Shallow crossings floor at 1.2 tolerances, down from 4: lowered in steps against the failing list; 2 left 39306_0031.

Tried and dropped: retrying a classification conflict with cut heads merged at two tolerances (no part built; two conflicts became ZeroThickness refusals, since the crossing beside the head still needs an edge on the other face); a chain search for a crossing on a tangent edge (26086: the cut appears, but settling moves it 6e-3).
