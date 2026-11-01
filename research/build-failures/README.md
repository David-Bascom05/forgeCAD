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
