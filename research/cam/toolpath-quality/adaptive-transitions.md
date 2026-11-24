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
