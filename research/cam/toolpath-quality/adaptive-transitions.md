# Adaptive: continuous pass transitions (design)

TOOLPATH-QUALITY, 2026-11-24. Status: design, being built on cam/ship-scurve, then cam/adaptive-only.

## Why

After the arc/line fit and the crumb take-back (cam/adaptive-only 6b8c43c9), adaptive's passes are
long arcs and lines, but every pass still ends, links and starts again: a short last piece at
the cap or the wall, a retrace of one or two moves, a short leg, and a short first piece while
the next pass turns toward material in air. Those are the 7-24 % of cutting moves under 1 mm
(tests/guards.rs ratchet) and most of what keeps a jerk-limited control off its programmed feed.

## What

Two-way passes end near the next pass's start. Join them with one tangent loop instead of
end, link, start:

1. At a pass's end the tool heads `u`; the next pass starts at `p` heading about `-u`.
2. The loop is a turn of radius `R` (R >= r, and R >= F^2/A where the machine's A is known)
   on the side away from the material, then a straight run, then a turn of the same radius into
   the next pass's heading: a "D"-shaped turnaround (two arcs and a line, G1-continuous, built by
   the shared fitter's pieces). Where p is about 2R across, it is one 180 deg arc.
3. The loop must lie in the cleared area: sampled at the map's cell, every centre allowed and
   its disk clear of material (the link rule's test, `Field::link_clear`, on the arc samples).
   Else the old transition (straight link, retrace, path, retract) is used.
4. The next pass begins where the loop ends, already on its heading; its leading air steps are
   dropped (the loop has done the turning).
5. Feeds: the loop runs at the link feed where it meets no material at all (the engage rule
   keeps a move's own feed only in air), so the cut passes keep their feed; the control blends
   the tangent joins without stopping.

One-way passes end far from the next start (the sweep returns to its start side), so their links
stay as they are; the loop is for `passes: "two_way"`, the mode to recommend once it wins.

## How we will know

On deep_boss / deep_islands / deep_open_slot (steel and aluminium), against the current two-way:
the share of cutting time at >= 90 % F under plan_scurve (up), the count of cutting moves under
1 mm (down; the ratchet tightened to the new level), plan_scurve time (down), and as always equal
volume (tests/guards.rs) and every op certified.

## Result (2026-11-24): parked

Built on cam/adaptive-loops (40df387a; analysis base, `FORGE_ADAPTIVE_LOOPS=1`,
`FORGE_ADAPTIVE_LOOP_R`): two-way pass end -> next start as a Dubins path (arcs of radius R and
lines, tangent), the start slid forward along its heading (its first steps are air), leaving the
pass end forwards or reversed (one stop where the pass ended against the cap), every sample an
allowed centre whose disk is clear or that moves away from material (no front engagement).

| R | transitions looped (deep_boss two-way) | plan_scurve set 1, deep_boss two-way | deep_islands two-way | cutting/link blocks under 1 mm |
|---|---|---|---|---|
| none (6b8c43c9) | - | 392.8 s | 1,033.6 s | 52 / 436 |
| >= r (3 mm) | 20 of 253 | 393.4 s | 1,023.7 s | 56 / 472 |
| 1 mm | 43 of 109 | 289.6 s, but 6,630 mm3 left uncut (open) | 912.2 s | 43 / 525 |
| 0.5 mm | 192 of 265 | 360.8 s (-8 %) | 816.5 s (-21 %) | 201 / 1,159 |

Equal volume except as noted; every op certified. The structural limit: a loop of R >= r needs
about 2R + r (9 mm) of cleared width, and this algorithm's passes lie r/2 apart with their ends
pressed against the cap or a wall. Loops that fit (R 0.5 mm, a link in air: a control holds
about 19 mm/s round it at A = 750 mm/s^2) win clearly under plan_scurve but are themselves
sub-mm blocks. To be judged again under CAM-MACHINE's block-rate / look-ahead model: if they
still win there they ship, and the micro-move ratchet then counts time lost to block rate
rather than raw sub-mm counts.

## Long term (not built): passes planned for their transitions

The real fix is in pass generation, not in the links: plan the passes so that each ends where a
loop of R >= max(r, F^2/A) fits. Two ways that fit this simulation:
- trochoidal passes in narrow and corner regions (each loop of the trochoid is itself the
  transition, the engagement held by the loop's pitch), with peels only in open regions;
- peels that stop a loop's width (2R + r) short of the cap or the wall and turn there, the strip
  left by the turn taken by the next pass or a ring.
Either changes what the simulation steers, so the cap, the peel test and the winding test are
re-derived for it; a redesign, scheduled after islands.
