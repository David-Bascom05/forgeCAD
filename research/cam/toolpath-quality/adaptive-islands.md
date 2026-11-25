# Adaptive round islands: where the time goes

TOOLPATH-QUALITY, 2026-11-25. Status: one change shipped (crumbs in the second round); the rest
recorded for the long-term pass design (adaptive-transitions.md, "passes planned for their
transitions").

## The breakdown

deep_islands, steel, rough op, one-way, `plan_scurve` set 1 (750 mm/s², 2·10⁴ mm/s³, δ 0.02),
hops on, by the speed each block runs at (cutting feed against the link feed of 165 mm/s):

| | adaptive | clear |
|---|---|---|
| cutting | 9.4 m, 541 s planned | 8.4 m, 642 s |
| links at the link feed | 843 blocks, 8.6 m: 52 s at full feed, 168 s planned | 38 blocks, 4 s |
| rapids | 204, 57 s | 6, 4 s |
| whole op | 857 s | 694 s |

Adaptive cuts in about 100 s less than clear and loses about 225 s on links and rapids. The
links are short (10 mm mean) and never reach the link feed at this acceleration. At first-set
feed (t_min) the two ops are equal (666 s against 679 s).

The short blocks counted in the earlier micro-move work (11-27 s lost, the "crescent fans" at
the pass ends) are a small part of this.

## Why so many links

The level makes 968 passes (deep_boss: 66). 765 of them cut under 1 mm, 90 mm in all. Nearly all
come from the level's second round of sweeps. The first round takes crumbs back; three crumbs in
a row end a sweep direction, and the corner crumbs in the stock's margin end each direction
early. Material is then left away from the walls, so the level goes round again with every pass
kept, and in the islands' lees that round made hundreds of passes of one to three steps.

## What was tried (analysis branch `cam/ship-scurve`, switches)

| change | deep_islands one-way / two-way, steel, set 1 | other parts |
|---|---|---|
| none | 857 / 936 s | |
| crumbs taken back in the second round too (cut under D/6) | 856 / 849 s | unchanged |
| crumbs never count toward a stall (`FORGE_CRUMB_NOSTALL`) | material left (54 of 99 cm³) | material left |
| the same, and a direction goes on while material is left near the walls | 1,016 / 1,040 s | boss 353 → 1,053 s, slot 68 → 505 s |
| retrace links pulled tight (`FORGE_RETRACE_PULL`) | 862 / 925 s | within 1 s |

- **Crumbs in the second round (shipped).** Equal volume, every op certified. The two-way passes
  gain most (−9.4 % steel, −11.3 % aluminium); one-way only −0.2 / −1.4 %, because its tiny
  passes mostly started where the tool already was (no link).
- **The stall rule is load-bearing.** Without it the first round spends itself on corner crumbs
  and the level is ended by the left-area rule (CAM-CORE's area while its tie fix is open).
- **Pulling retraces tight does not pay.** A pulled leg must keep every point it skips in sight
  of its start (else the pass and its link could wind round an island, against the winding
  condition). Along curved passes that stops the pull after a few millimetres, so the link has as
  many blocks as before.

## What is left for the long-term design

One-way: about 110 retrace returns, each back along a pass that curls round an island. Two-way:
the turnarounds. Both are per-pass costs, so the lever is fewer, longer passes round islands
(trochoidal passes in the narrow lees, peels only in the open), not smoother links. That is the
"passes planned for their transitions" design in adaptive-transitions.md.
