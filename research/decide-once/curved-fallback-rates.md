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

## The 36 unkeyed coincidences (a guess site pending provenance keys)

At 14ffda7/dfb4692, `same_carrier` keys every `true` of `ssi::coincident` on plate, patterns and
housing except these 36, all in patterns seq at "trim run-on: the face across the edge" (B:2700).
They are vertical planes (z = 12 on the edge), not axis-aligned, so the key (axis-aligned planes
only) does not apply. Their normals differ by 0 to 8e-16 and their offsets (`off`) by 1e-14 to
8e-14: one or a few ulps. Most likely they are pattern copies or rotated walls placed in floats.

The Lean coordinator's review: these are a real F1 gap, not merely "the tolerance for now".
exactOf reads them as distinct parallel planes an ulp apart, so the exact result would carry a
sliver or a two-sided face pair that the code's merged result lacks, and Refines_δ fails. The
theorem's hypothesis must exclude them (FaithfulInput with keyed carriers) until provenance keys,
the master's (B), make their carriers equal by key. Listed by face name
(`FORGE_DECISIONS_DUMP=1 regen1 patterns seq`, `decision-unkeyed` lines):

| # | face A | face B | A's normal | B's normal differs by | A's origin | off | sine |
|---|---|---|---|---|---|---|---|
| 1 | `#f7f93040c3ed2b8f` | `#ed8508bd22592eab` | (-0.013000, 0.999916, 0) | 8.0e-16 | (175.070, 152.651, 12) | 1.9e-14 | 8.0e-16 |
| 2 | `#27a87601a5ad961b` | `#6f3ed77bf7f2d19c` | (0.013000, -0.999916, 0) | 8.0e-16 | (138.925, 148.181, 12) | 4.7e-14 | 8.0e-16 |
| 3 | `#37fd6c301e78fcb3` | `#b61da3a739040e80` | (-0.186436, 0.982467, 0) | 8.3e-16 | (173.849, 161.305, 12) | 4.1e-14 | 8.3e-16 |
| 4 | `#5d3670225dec2cfe` | `#c03310a5f5da154d` | (0.186436, -0.982467, 0) | 8.3e-16 | (139.029, 150.627, 12) | 7.6e-14 | 8.3e-16 |
| 5 | `#f118764b06167ee3` | `#e64f7745e6f8b85e` | (-0.354207, 0.935167, 0) | 0.0e+00 | (171.143, 169.616, 12) | 3.9e-14 | 0.0e+00 |
| 6 | `#323d6b8c7855836c` | `#ab025331d3be7608` | (0.354207, -0.935167, 0) | 0.0e+00 | (138.707, 153.053, 12) | 5.0e-14 | 0.0e+00 |
| 7 | `#92e19a64ef73ddd0` | `#09fcadb46c35cb6b` | (-0.511216, 0.859452, 0) | 1.1e-16 | (167.036, 177.331, 12) | 3.9e-14 | 5.6e-17 |
| 8 | `#011859ed30b6c438` | `#ca852520820f282d` | (0.511216, -0.859452, 0) | 5.6e-16 | (137.969, 155.387, 12) | 8.2e-14 | 7.2e-16 |
| 9 | `#6628a33640671349` | `#9ec43fe0f1dedc7f` | (-0.652692, 0.757624, 0) | 2.2e-16 | (161.652, 184.215, 12) | 2.1e-14 | 2.2e-16 |
| 10 | `#a82a7c76abc9f8fd` | `#7838a6369491744b` | (0.652692, -0.757624, 0) | 2.2e-16 | (136.836, 157.557, 12) | 1.1e-14 | 2.2e-16 |
| 11 | `#a55a601904b3e532` | `#0b1952c480d8c596` | (-0.774336, 0.632775, 0) | 8.9e-16 | (155.153, 190.060, 12) | 5.0e-14 | 1.2e-15 |
| 12 | `#2c23666ec252a8e4` | `#ee8ea353498135a9` | (0.774336, -0.632775, 0) | 5.6e-16 | (135.344, 159.498, 12) | 5.0e-14 | 7.2e-16 |
| 13 | `#a528b3959233b41a` | `#021797126cd1d252` | (-0.872452, 0.488700, 0) | 0.0e+00 | (147.739, 194.687, 12) | 3.6e-15 | 0.0e+00 |
| 14 | `#d334f717d62920af` | `#e5574cef711ab928` | (0.872452, -0.488700, 0) | 0.0e+00 | (133.538, 161.150, 12) | 3.6e-15 | 0.0e+00 |
| 15 | `#a43e106bebf18df4` | `#914f008d1f433f10` | (-0.944059, 0.329776, 0) | 0.0e+00 | (139.634, 197.957, 12) | 1.1e-14 | 0.0e+00 |
| 16 | `#9cff830b8f8c01e0` | `#357e24490e27d549` | (0.944059, -0.329776, 0) | 3.3e-16 | (131.472, 162.463, 12) | 1.8e-14 | 3.3e-16 |
| 17 | `#28466fbe25dc0cf4` | `#d477c71c1b257602` | (-0.986982, 0.160831, 0) | 8.3e-17 | (131.084, 199.769, 12) | 8.9e-15 | 8.3e-17 |
| 18 | `#5ef66027915dcb64` | `#a2e4ec6482581d89` | (0.986982, -0.160831, 0) | 4.4e-16 | (129.210, 163.397, 12) | 3.2e-14 | 4.4e-16 |
| 19 | `#938a025d05edb5db` | `#c4a34ebf4b99e11c` | (-0.999916, -0.013000, 0) | 4.0e-16 | (122.349, 200.070, 12) | 7.9e-15 | 4.0e-16 |
| 20 | `#2a1fe26f0d4c0641` | `#329910b5489b7475` | (0.999916, 0.013000, -0) | 4.0e-16 | (126.819, 163.925, 12) | 2.2e-14 | 4.0e-16 |
| 21 | `#ed173e120e24eb65` | `#0016c6312f843b4f` | (-0.982467, -0.186436, 0) | 2.2e-16 | (113.695, 198.849, 12) | 1.2e-14 | 2.2e-16 |
| 22 | `#81e923f849d55014` | `#fbc85559fd14400f` | (0.982467, 0.186436, -0) | 3.6e-16 | (124.373, 164.029, 12) | 2.7e-14 | 3.6e-16 |
| 23 | `#bbb4e9794c2082e9` | `#2ee6dfdfa469312f` | (-0.935167, -0.354207, 0) | 5.6e-16 | (105.384, 196.143, 12) | 2.5e-14 | 6.1e-16 |
| 24 | `#cd30a9863e39fa32` | `#0b2d12f5974ccf4e` | (0.935167, 0.354207, -0) | 3.3e-16 | (121.947, 163.707, 12) | 3.9e-14 | 3.3e-16 |
| 25 | `#3537e440e4781b5c` | `#f8098d80fcf9b21a` | (-0.859452, -0.511216, 0) | 2.2e-16 | (97.669, 192.036, 12) | 1.4e-14 | 2.2e-16 |
| 26 | `#3026c063b6202a99` | `#b63f42d1a3c96b3e` | (0.859452, 0.511216, -0) | 4.4e-16 | (119.613, 162.969, 12) | 1.4e-14 | 6.1e-16 |
| 27 | `#eb30f05e30d95ac9` | `#e60252b58511248d` | (-0.757624, -0.652692, 0) | 8.9e-16 | (90.785, 186.652, 12) | 7.1e-15 | 1.1e-15 |
| 28 | `#0d5b1d5e50b5b79d` | `#4994e8c820011f67` | (0.757624, 0.652692, -0) | 5.6e-16 | (117.443, 161.836, 12) | 1.4e-14 | 6.1e-16 |
| 29 | `#e66373cd303882b6` | `#65bfa81065b09e99` | (-0.632775, -0.774336, 0) | 3.3e-16 | (84.940, 180.153, 12) | 2.1e-14 | 3.3e-16 |
| 30 | `#1b7d481e94abf373` | `#c237bbfa8711498f` | (0.632775, 0.774336, -0) | 3.3e-16 | (115.502, 160.344, 12) | 2.8e-14 | 3.3e-16 |
| 31 | `#480bbf93008598dc` | `#a8938fa3e92b172d` | (-0.488700, -0.872452, 0) | 4.4e-16 | (80.313, 172.739, 12) | 3.2e-14 | 5.0e-16 |
| 32 | `#44d5deb090faba59` | `#6f40f555c6ce2452` | (0.488700, 0.872452, -0) | 4.4e-16 | (113.850, 158.538, 12) | 5.0e-14 | 5.0e-16 |
| 33 | `#fcd873a85665f35b` | `#8ffdeb01544caea6` | (-0.329776, -0.944059, 0) | 1.7e-16 | (77.043, 164.634, 12) | 1.1e-14 | 1.7e-16 |
| 34 | `#735de16f332c484c` | `#6769c5b8bd3d0a01` | (0.329776, 0.944059, -0) | 1.7e-16 | (112.537, 156.472, 12) | 3.6e-15 | 1.7e-16 |
| 35 | `#c5aeba3281e97406` | `#fffebb800288ec6a` | (-0.160831, -0.986982, 0) | 7.5e-16 | (75.231, 156.084, 12) | 1.6e-14 | 7.8e-16 |
| 36 | `#e889097946e9e457` | `#65107f5ebb245331` | (0.160831, 0.986982, -0) | 7.5e-16 | (111.603, 154.210, 12) | 5.0e-14 | 7.8e-16 |
