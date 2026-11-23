# Where the plane ledger's cells go, and per-phase costs (A8 input, 2026-11-23)

Measured on claude/repo-review-mbdrlj-decide-once at 5eb8e9d (proven v1 core, bool-speed's
in-place write merged), switch on, callgrind, the gate's seq regen of each part.

## Cells of the plane arrangement, by fate

Every cell `arrange()` makes on a near class plane's section, counted at `build_ledger`'s cell
loop, by the first test that drops it:

| part | cells made | touch a section-box wall | not open (touch a region wall) | outside every face box | in no face | kept (facets) |
|---|---|---|---|---|---|---|
| coplanar (70 booleans) | 20 986 | 11 612 (55%) | 379 (2%) | 4 484 (21%) | 1 344 (6%) | 3 167 (15%) |
| upto (4) | 1 568 | 834 (53%) | 155 (10%) | 215 (14%) | 148 (9%) | 216 (14%) |
| housing (1) | 195 | 120 (62%) | 0 | 56 (29%) | 0 | 19 (10%) |

- "Section-box wall": the section is the class's faces' box grown, clipped to the region grown by
  1/64 of its size, so the slab between the region's walls (classes) and the section's walls is
  arranged by every crossing class and then thrown away. Trimming it keeps every decision but
  changes the order cells are made in (accepted by the master: switch-on bits are not a baseline).
- "Outside every face box": a cell no face of its class can hold, decided after the cell is built.
- Per arrangement on coplanar (625 calls): 16.0 classes with the 6 walls, 9.5 of them crossing the
  clipped section, 33.6 cells, 46.4 vertices (23.6 meets of three classes, with covectors).

## Ledger sizes per decide-once boolean

| part | booleans | input faces | near | classes + walls | ledger cells | pieces | edges | verts | witnesses |
|---|---|---|---|---|---|---|---|---|---|
| coplanar | 70 | 183 | 10.0 | 15.0 | 231 | 46.7 | 72.4 | 30.1 | 17.1 |
| upto | 4 | 12 | 11.8 | 17.8 | 276 | 54.0 | 103.5 | 53.2 | 14.8 |
| housing | 1 | 12 | 7 | 13 | 103 | 19 | 28 | 12 | 6 |

(upto's booleans are 12-face, all-near: the region is everything.)

## Instructions per decide-once boolean (M)

| phase | coplanar | upto | housing |
|---|---|---|---|
| arrangement (`arrange`) | 1.26 | 1.62 | — |
| rest of build_ledger (facets, in_face, exact points, interning) | 1.20 | 1.45 | — |
| inputs + snap + vertices + region | 1.1 | 0.4 | — |
| witnesses (core's list + rays) | 0.4 | 0.53 | — |
| fill, all | 4.0 | 4.15 | 1.57 |
| core (proven v1, sign lists) | 1.64 | 1.03 | — |
| write (write + build_body + apply + sliver) | 1.0 | 0.14 | — |
| total | 6.0 | 5.5 | 2.2 |

Whole regen, switch on vs off (base): coplanar 0.542G vs 0.209G, upto 57.7M vs 41.0M, housing
133.7M vs ~140.6M (housing's one flat boolean is a small share; the rest takes the old path).
The packed-row core (shelved) took coplanar's core from 1.64M to 0.79M per boolean.
