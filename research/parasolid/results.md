# Parasolid reader: results

What `forge_step::import` makes of real transmit files, checked against the STEP export of the
same model where there is one. Run 2026-11-24/25 on parasolid/xt-reader @11abf0c4, so this predates
the SP-curve-on-blend fallback, fcbba1ec. Tools are in this directory (README.md):
`corpus.sh xtcompare`, totalled by `summarize.py`.

## The Haas corpus (SolidWorks 2017 to 2025, Parasolid 29 to 36.1)

146 archives on branch haas-3d-models. 144 hold an XT, 196 XT files in all, all text form.
Each file was read unplaced (`XT_UNPLACED=1`, each body in its own part, as a STEP read leaves an
assembly's parts). `XT_NOCHECK=1` skipped forge-topo's `check()`, whose memory ran away on HA2TS
(KERNEL-STRESS's op-budget now stops it with `Problem::TooLong`). Solids are paired with the
STEP's by box and volume.

| | |
|---|---|
| XT files read | 192 of 196 (4 timed out at 30 min each, below) |
| solids from the XT | 14,148 |
| solids from the STEP | 10,864 |
| paired | 10,425 |
| volume agrees (1e-4) | 9,735 of 10,425 (93.4%) |
| volume not measured (refusal or budget on either side) | 542 |
| area agrees | 9,784 (93.9%) |
| area per surface kind agrees | 9,607 (92.2%) |
| box agrees | 9,916 (95.1%) |
| time, XT against STEP | 3,991 s against 1,287 s (3.1x) |

The XT side has more solids than the STEP because the STEP exports leave out sheet and wire
bodies and merge or drop some parts. Most of the unpaired XT solids are bodies that the STEP does
not have.

Disagreements: 10,425 − 9,735 − 542 = 148 pairs.
- Most are near misses, 1e-4 to 6e-4 (the cobot arm links, gm-1, HPB). In each the XT's integrals
  and mesh agree with each other, and the gap is between the two exports' geometry.
- The large ones (VF-1 5.6, HRP-2 1.9, TM-1/TM-2 0.92, GM2-5AX 2.6e3) are in files whose XT holds
  many more solids than its STEP (GM2-5AX: 698 against 535), and are pairing mistakes rather
  than geometry. This is not proven case by case.

Refused, by class (1,682 in all):

| count | refusal | status |
|---|---|---|
| 1,238 | a sheet body | as STEP import: only solids are read |
| 12 | a wire body | as above |
| 232 | its surfaces would take too long to measure | the measure budget: blends rolling on B-spline surfaces, NURBS nearest-point searches. Fitting such faces to NURBS (forge-step fit) would help; not done |
| 48 | a closed spline surface face with one rim and no pole | open: a loop's winding on a closed spline surface counted wrong. One cause, an edge past a periodic B-curve's closing point, is fixed (11abf0c4), and these remain |
| 45 | no place on the rim for a second meridian | open: a sphere-like cap whose rim is cut by holes everywhere a seam could go |
| 32 | an SP-curve in a blend's parameters | fallback added after this run (fcbba1ec): the edge is traced where its faces meet |
| 27 | a cliff-edge blend with no cliff | open: the blend's cliff is not on either face's surface |
| 20 | the 2D curve leaves its surface | open: an SP-curve's parameters outside the surface's domain (trimmed B-surfaces) |
| 13 / 6 | a closed cone / cylinder face with one rim and no pole | open: same class as the 48. HA5C traced: two edges on a 308-segment closed curve meet at its closing point, and the loop's winding comes out 0 |
| 6 | a face over a span of its parameters below 1e-9 | a degenerate face, refused |
| 3 | no place on the second rim for a seam | as the 45 |

Timeouts, 30 min each (the reader, the STEP read and both measures together): ROBOT PKG-1 7KG
FRONT and SIDE, ST-10/ST-15, UMC-750 30/50/70 PKT. These are large assemblies. Which part of the 30 minutes
the XT read takes, rather than the STEP read or the measures, has not been measured yet.

## Onshape exports (52 public files, Parasolid 29 to 38.1)

Fetched read-only from public GitHub repositories, as the coordinator approved. Each file's
header says `APPL=Onshape`, and one says `ONSHAPE BY PTC INC`. Six MIT-licensed ones are fixtures
(tests/data/xt/onshape).

| | |
|---|---|
| read clean (every solid checked, nothing refused) | 44 of 52 |
| sheet bodies, refused as STEP does | 4 |
| kernel cases, sent to KERNEL-STRESS | 3: mass silently wrong on a closed B-surface band (worst class), two "loops draw no region" |
| measure budget | 1 (after the SP-curve fallback got it past its refusal) |

What the Onshape files needed:
- **Schemas not carried.** Onshape's translation service (the route a REST import would use)
  writes `SCH_xxx_37102` with no embedded schema. `known.rs` now carries 11 schemas' layouts
  (28101 to 38101), each node type's record taken from files that do carry it (the Onshape set
  and this corpus, 537 records with no disagreement between files). A schema nothing has shown
  reads by the nearest.
- **A periodic B-curve's closing point.** An edge running past it took the other arc, and is
  now split there.
- **The import check's chord.** It is taken from the edges' box: a closed extrusion's face box
  is ±1e6 mm (forge-topo procedural_box, with KERNEL-STRESS).

## Not checked here

- **Binary .x_b.** It is read by the same code with the binary lexer, and the fixtures
  (block, cylinder, multi-part, nested assembly) cover it. No corpus file is binary, and the
  public Onshape samples searched were all text.
- **Colours.** They are read from the file but have nowhere to go: neither `Solid` nor `Body`
  has a colour.
- **Placed comparison against CAM-HAAS's STEP placement** (machines/haas-model-import
  @29bd8a32). This run compares unplaced, as each body stands in its own part.
