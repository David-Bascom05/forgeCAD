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

Counts: real parts = find-parts, 8409 parts, on main d33b84aa; history = 100 seeds, 1685 lines
tried; scoreboard = 841 parts.

| class | count | root cause | blocks | state |
|---|---|---|---|---|
| butted cylinders of one radius | 237 real parts | a closed edge two bodies share: its direction was read from its ends, one vertex, so one use ran the wrong way: open shell, or false ZeroThickness | shafts and tubes built in steps from a joint | fixed (`canon` in boolean.rs, nary.rs) |
| lost cut, silent wrong volume | 2 real parts, wrong with no error | a stretch whose middle lands within tolerance of a boundary with its ends on that boundary was taken to run along it, and no edge was made | any model carrying float noise | fixed (`inside` in `trim` samples on) |
| classification floods across a shared edge | 76 real parts (one signature, 2 of 3 repros traced to this and near-flush below) | a tool's edge on a part's edge: being inside the tool changes across it | pockets ending on a rib's root | fixed |
| nearly flush surfaces | part of the 76 and of the 65 below | plane on plane or tangent to a cylinder within LINEAR over the faces, tilted past `ANGULAR` (9-digit rounding), crossing in curves the tolerance cannot tell apart | imported or rounded data | fixed (`ssi::nearly_flush`) |
| probe on a touch | 65 real parts (2 of 3 repros), 6 scoreboard | the one probe of an untouched face lands on a tangent line; also a whole edge imprinted because its middle touched | cross holes ending on a tangent plane | fixed, except a tool's corners touching a circle (repro e6ca2) |
| flat ends named alike | 46 real parts | flush vertices were not ranked by place: DuplicateName | a flat by an arc on the rod's own circle | fixed |
| pinch at a vertex in a face split | 22 scoreboard (ball_poles) | own edges tangent at a vertex never counted as a tie | balls touching on a face | fixed |
| next face past a part-changing feature | 35 history | refused outright | `next` after any fillet | fixed one-body; multi-body still refused (23, needs each feature's pick where the extent is resolved) |
| crossed split, hair imprint | 16 real parts (1 of 3 repros) | imprint between the same vertices a hair from the boundary | near tangent tool sides | fixed for that repro |
| fillet at any radius: NoBlend 28, Overflow 18, Corner 20 | 66 history, of 202 fillets tried | to trace: variable blends next to variable blends, faces of coils, sweeps and lofts; setbacks where one blend ends | filleting anything but plain edges | open, next |
| shell at any thickness: Collapses 15, Rebuild 8 | 23 history, of 39 shells | to trace: a drill point's cone apex; concave faces offset outward | shelling drilled parts | open |
| chamfer at any size: TooLarge 12 | 12 history | to trace | | open |
| chamfer Unsupported: curved non-circle 6, vertex of other than 3 faces 3, straight on a curved face 4 | 13 history | no such chamfer written | chamfering a tilted hole's rim | open |
| n-ary refuses several bodies in `finish` | 37 history | the history check calls the one-body n-ary regen: a test gap, the n-ary path goes unchecked on those | | open |

Remaining real-part signatures not yet traced: invalid FaceWrongSide 4, VertexFans 3, EdgeOffFace
n-ary 7, Euler 1, VertexTolerance 1, EdgeDegenerate 2, DuplicatePoint mesh 6, `two vertices of one
body meet at a cut` 3, `hole outside every region` 1.

## Correctly refused

| refusal | count | why it is right | message |
|---|---|---|---|
| Empty | 76 scoreboard | nothing left: OCCT's volume is 0 on all 76. The scoreboard counted these as failures | clear |
| ZeroThickness | 102 scoreboard | material meets itself along a line or at a point; OCCT returns a non-manifold body. Whether to allow such bodies is a design decision (below) | says where |
| fillet Overflow, chamfer TooLarge, shell Rebuild, when smaller builds | 23 + 6 + some history | the size does not fit | now gives the size that builds |
| generator misses: `NotOnFace`, `NoEntity`, `NotCrossed`, `NeverHits`, `BadSource` | ~75 history | the random line asked for something absent | could say more |

## Open questions for David

- Should a profile whose hole touches its outline (21 real parts: `circle 0 0 40 circle 0 -30 10`)
  build? It pinches along a line: non-manifold, Fusion keeps it. So should ZeroThickness results.
- `this leaves nothing of the part` where a cut removes one whole body of several (3 real parts):
  delete the body as Fusion does, or refuse?

## Not done

The OCCT replay of all 495 refused histories did not finish, so "any size" above is measured by
trying smaller, not yet by OCCT.
