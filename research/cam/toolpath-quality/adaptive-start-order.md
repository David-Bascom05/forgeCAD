# Adaptive: the order passes start in (parked)

Status: **parked** (coordinator, 2026-11-24). The band order stays.

## What adaptive does

The next pass starts at a parked centre next to material. Candidates are ranked by band across
the sweep direction first (bands r/2 wide, the lowest first), then by distance from the tool.
The sweep's front therefore moves across the level band by band.

A level moves on to the next sweep direction (reverse, then the two across) only while material
more than 4r from the walls is left. What is left nearer the walls goes to the cleanup rings
and the wall pass.

## What was tried

All on the analysis branch `cam/ship-scurve` behind environment switches:
- `FORGE_ORDER_NEAR`: nearest first, no band.
- `FORGE_BAND_K`: a soft band, each band costing K mm² of squared distance.
- `FORGE_LEFT_D`: the stop rule's distance, in r.
- `FORGE_REGION`: the tool's own connected cleared region first.

Conditions: steel, rough op only, `plan_scurve` set 1 (750 mm/s², 2·10⁴ mm/s³, δ 0.02), hops on,
an exact stop at every G0. Volumes are the rough op's; the full-volume rows equal clear's.

| order | deep_islands | deep_boss | deep_open_slot |
|---|---|---|---|
| band (shipped) | 867 s | 355 s | 68.1 s |
| nearest first | 795 s | 356 s, 24 % of the volume left (54,940 of 72,390 mm³) | 66.1 s |
| nearest first, stop at 2r | 805 s | 638 s | 66.1 s |
| nearest first, stop at r | 805 s | 670 s | 66.1 s |
| band, stop at 2r | 867 s | 597 s | 68.1 s |
| band, stop at r | 867 s | 656 s | 68.1 s |
| soft band, K = 10³ to 10⁹ | 867 s | 355 s | 68.1 s |
| soft band, K = 10² | 886 s | 426 s | 68.1 s |
| soft band, K = 10 | 886 s | 56,801 mm³ (incomplete) | 52.4 s |
| soft band, K = 1 | 87,294 mm³ (incomplete) | 65,183 mm³ (incomplete) | 66.1 s |
| own region first (before hops, G0 blended) | 915 s (band: 888 s) | 404 s (band: 371 s) | 68.3 s |

## Why

- **Nearest first leaves material.** On deep_boss's narrow ring round the boss, almost all the
  material is within 4r of a wall. Nearest-first passes stall early. The level does not go round
  again, because nothing is more than 4r from the walls, and the rings cannot enter where no
  vertex is clear.
- **Completing it costs more than it saves.** Stopping at r or 2r instead of 4r finishes the
  level, but by more sweeps. On deep_boss that is +80 %, against about −7 % on islands. The band
  order leaves its remainder where the rings take it cheaply.
- **Own region first is no help.** The cleared area round the islands is one connected region,
  so the tool is never walled in. It retracts because the way round through the cleared area is
  longer than a retract costs.

## If it is reopened

The one design seen to get both results is per level: plan with both orders, keep the faster
(complete) one. It doubles adaptive's generation time; deep_islands already takes about 39 s to
build. Worth it only once generation is cheaper.
