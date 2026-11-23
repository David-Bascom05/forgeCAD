# How often a proven float screen could not decide: today's path, measured

The master asked for this on 2026-11-23, before any wrapped step is replaced. The question is how
often each screen planned in curved-builder-mapping.md would fail to decide, on plate, patterns,
housing and the fuzz corpus.

## Method

- forge-bool's decision log (feature `decisions`, off by default) already records every tolerance
  decision of today's path whose measured value is within `BAND` = 100·LINEAR of its threshold. It
  records the fact's kind, the site, the measure (a distance or a sine) and the limit.
- `FORGE_DECISIONS_DUMP=1` prints each decision and each plan's close. It is new, and compiled only
  under `decisions`.
- A decision outside the band is far from any threshold, and a proven f64 screen decides it the
  same way today's path does.
- Inside the band, each decision falls in one of three buckets by |measure|. Lengths are in mm on
  parts about 100 mm across, so f64 noise is about 1e-14.

| bucket | meaning for a float screen |
|---|---|
| ≤ 1e-12 | the quantity is zero in exact terms (a coincidence); no sign screen can decide it |
| 1e-12 … 1e-9 | float noise of one fact computed two ways; a screen with a 1e-9 bound fails |
| 1e-9 … limit (LINEAR or the tolerance) | a real near-miss: a screen decides it, and the answer differs from today's tolerance answer (a change to list) |

## Results (`regen1 PART MODE`, and `fuzz1`–`fuzz40` seq)

| run | decisions in band | ≤1e-12 | 1e-12…1e-9 | 1e-9…limit | plans | plans with a ≤1e-9 decision | plans with a near-miss |
|---|---|---|---|---|---|---|---|
| plate seq | 5,224 | 5,207 | 4 | **0** | 441 | 221 | **0** |
| plate nary1 | 6,588 | 6,498 | 78 | **0** | 3 | 2 | 0 |
| patterns seq | 39,338 | 38,921 | 0 | **0** | 393 | 197 | 0 |
| patterns nary1 | 47,603 | 47,235 | 58 | **0** | 3 | 2 | 0 |
| housing seq | 7,611 | 7,605 | 0 | **0** | 373 | 187 | 0 |
| housing nary1 | 37,372 | 37,319 | 0 | **0** | 3 | 2 | 0 |
| fuzz 1–40 seq | 121,312 | 120,460 | 477 | **58** | 6,440 | 2,482 | **16** |

(Each sequential boolean closes two plans, so 221 of plate's 441 plans is about every curved
boolean. The rows' remainders are decisions past their limit, or decisions with no measure.)

The sites, in plate seq; patterns and housing look the same:

| fact | site | count | exactly 0.0 |
|---|---|---|---|
| point on edge | `on_edge` (B:2833: a hit against the edge it came from; B:2523) | 3,975 | 2,662 |
| two cuts, one vertex | `place_cuts` clusters | 1,128 | 492 |
| two faces, one surface | `unify` SAME / `ssi::coincident` | 76 | 76 |
| vertex on surface | coincident faces ⇒ their vertices | 10 | 10 |

In patterns, "vertex on surface (derived)" adds 14,921, and "point at vertex" (a cut at its own
edge's end) adds 2,010.

## What this says

1. **True near-misses are rare.** None occur on plate, patterns or housing, and the fuzz corpus has
   58 decisions in 16 of 6,440 plans. Those are the cases where a proven screen would answer
   differently from LINEAR, and they would be listed per step, as the master's rule requires.
2. **Coincidences are on every curved boolean.** All of them come from construction:
   - a cut point tested against the edge it was computed on;
   - two cuts that are one vertex;
   - faces on one surface;
   - a vertex of a face lying on a coincident face's surface.

   A plain sign screen cannot decide a quantity that is zero, so a design that sends these to the
   exact fallback (resultants, isolation) would run it on every boolean, at the costs in
   curved-cost-estimate.md. **That is not acceptable, and the fallback is not where these belong.**
3. **They are decided by provenance and keys, as flat decide-once decides them:**
   - a point made as curve ∩ edge is on that edge by construction;
   - two cuts of one curve at one edge, or at the edges of one vertex, are one vertex by their keys
     (the vertex is the meet of its three carriers);
   - two faces lie on one surface when their exact carrier classes are equal. Stage (a)'s
     `carrier_classes` already gives this, by exact comparison of the stored frames: a few big
     integer operations, microseconds, no resultants.

   The float screens then decide only what isn't a key identity: crossings of distinct carriers,
   angle order, area sign. By these numbers, those are clear of zero, except the fuzz corpus's 58
   decisions and the noise bucket (1e-12…1e-9, which is one fact computed two ways, so again a key
   identity).
4. **The exact fallback is therefore needed only for true tangencies and coincidences that no key
   explains.** On the gate parts its rate is 0 in this log. On the fuzz corpus it is at most 16
   plans in 6,440. Both are to be confirmed once the keyed decisions replace each wrapped step.

## Next, per the master's order

Replace one wrapped step at a time. The first is `two faces one surface` / `ssi::coincident` with
exact class keys. Each step keeps bits identical or lists its changes, has 0 worse reliability,
and is at most today's cost at equal threads.
