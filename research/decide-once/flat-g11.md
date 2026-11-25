# Flat G11: the tolerance decisions the flat booleans reach

old-path-statements.md §G, G11: "the many LINEAR / RIM tolerance tests ... each is a guess unless it
is a proven screen". This note narrows G11 to what the **flat** corpus reaches, with each site's
outcomes counted on inputs the exact reference judges, as the worklist for the flat G11 proofs.

## Method (FLAT-SITES, 2026-11-24, main f04c533)

- Build: `exact-fuzz` with branch coverage (nightly-2026-09-17, `-C instrument-coverage
  -Z coverage-options=branch`, release, x86-64-v3), as research/coverage-2026-11-23 did for the
  whole corpus (its `tools/tolsites.py`, unchanged).
- Run: `exact-fuzz` over all its flat sources (families, plane_plane, notched, suite, lang, more):
  sequential agree 236, nary agree 227, ambiguous 67 (+400 ulp), out of scope 143, wrong 0,
  refused 0. Every boolean counted here is one the exact reference judged, and the kernel agreed.
- Sites: every forge-bool branch condition whose line names tol::LINEAR / ANGULAR, RIM, a `.tol`
  or a literal 1e-N (164 sites). On this corpus: **76 live** (both outcomes taken), 50 one-sided,
  38 never evaluated. (The whole corpus, gate + bench + fuzz + tests: 100 live, 51 one-sided, 13 never.)
- Caveat: the `lang` family includes round features (circles and cylinders); ssi.rs's radius tests
  and the conic sites below are reached through those, not through planes alone.

## Triage classes (to fill per site)

- **S** proven screen: the test only filters, an exact test decides (cite the theorem).
- **T** tolerance decision correct under δ-Faithful (the TolOn pattern: the result valid and each
  entity within δ of its exact meet; cite the lemma and the measurement).
- **P** provenance: the decision is exact on the point's exact key (cite the key and its proof).
- **U** UNKNOWN: none yet (the open G11 rows; bring with numbers).

## The 76 live flat sites (evaluations, true, false, minority %, source)

```
  3911014 classify.rs:1234:12        578887  3332127  14.801  if di.abs() < 1e-300 {
   603074 boolean.rs:289:8           489286   113788  18.868  if d <= edge.tol {
   440190 boolean.rs:3057:28         361141    79049  17.958  if let Some(((s, tol), h)) = on
   361141 boolean.rs:3058:28         187112   174029  48.189  && other.grow(tol::LINEAR).contains(h.point)
   338924 boolean.rs:2877:36           3149   335775   0.929  if a.dir.get().cross(b.dir.get()).length() <= 1e-6)
   262579 boolean.rs:5095:12         136339   126240  48.077  if di.abs() < 1e-300 {
   244534 nary.rs:1514:16             58916   185618  24.093  if vx.point.distance(c.point) <= vx.tol.max(c.tol) {
   180672 unify.rs:298:21             36296   144376  20.089  if ka.0 == 0 || ka.0 != kb.0 || (ka.1 - kb.1).abs() > 1e-3 {
   151248 boolean.rs:3521:16          29615   121633  19.580  if f != 0.5 && curve.eval(t0).distance(p).min(curve.eval(t1).distance(p)) <= 4.0 * tol::LINEAR 
   144376 unify.rs:298:37            125909    18467  12.791  if ka.0 == 0 || ka.0 != kb.0 || (ka.1 - kb.1).abs() > 1e-3 {
   122771 boolean.rs:3611:12              8   122763   0.007  if t1 - t0 <= 1e-12 {
    99275 boolean.rs:4728:20            196    99079   0.197  if w[1] - w[0] <= 1e-12 * (1.0 + edge.range.width()) {
    98840 shape.rs:234:8              36910    61930  37.343  if sweep.abs() >= dmath::TAU - 1e-12 {
    79282 boolean.rs:2911:32          77886     1396   1.761  if curve.eval(t).distance(vx.point) <= vx.tol {
    78374 boolean.rs:3638:31          63559    14815  18.903  let near = |j: usize| j != i && cuts[j].point.distance(cuts[i].point) <= cuts[i].tol.max(cuts[j
    64131 boolean.rs:2257:16             84    64047   0.131  && d > bodies[sd].vertex(v).tol
    49928 boolean.rs:5019:16          36616    13312  26.662  if t.curve.eval(s).distance(vx.point) <= vx.tol {
    46700 boolean.rs:4695:32          40737     5963  12.769  if let Some((s2, tol)) = on_edge(db, e2, h.point) {
    32380 nary.rs:3576:19             32320       60   0.185  while step > tol::LINEAR {
    30736 shape.rs:131:20             16924    13812  44.938  if sweep.abs() < dmath::TAU - 1e-12 {
    29615 boolean.rs:3521:28             18    29597   0.061  if f != 0.5 && curve.eval(t0).distance(p).min(curve.eval(t1).distance(p)) <= 4.0 * tol::LINEAR 
    28830 boolean.rs:2715:32          16812    12018  41.686  if let Some((s, tol)) = on_edge(body, e, h.point) {
    21408 boolean.rs:3337:12           2927    18481  13.672  if gap <= 1.2 * cuts[i].tol.max(cuts[j].tol) || gap > 0.3 {
    20102 classify.rs:1104:45            28    20074   0.139  if side.abs() <= tol::LINEAR || along.abs() <= 10.0 * tol::LINEAR {
    18963 nary.rs:1073:18              2947    16016  15.541  .map(|b| shapes.is_none() || !outside(&b.grow(tol::LINEAR)))
    18481 boolean.rs:3337:57          17556      925   5.005  if gap <= 1.2 * cuts[i].tol.max(cuts[j].tol) || gap > 0.3 {
    17498 classify.rs:730:20           9884     7614  43.514  && self.body.face(f).surface.distance(p) <= tol::LINEAR
    16812 boolean.rs:2718:32           7970     8842  47.407  if other.grow(tol::LINEAR).contains(h.point)
    16542 classify.rs:1213:8           1171    15371   7.079  if disc <= 1e-12 * (bb * bb + (4.0 * aa * cc).abs()) || aa == 0.0 {
    15520 boolean.rs:4265:12            151    15369   0.973  if disc.abs() <= 1e-6 * r * r {
    12865 boolean.rs:5864:31           9367     3498  27.190  .find(|k| c + k >= lo - 1e-9 && c + k <= hi + 1e-9)
     7086 ssi.rs:121:40                7036       50   0.706  if tilt <= tol::ANGULAR || tilt > NEAR {
     6556 boolean.rs:5075:20              4     6552   0.061  if let Some((s, tol)) = on_edge(bodies[to], e2, c.point) {
     5126 boolean.rs:3504:28           5080       46   0.897  && curve.eval(t0).distance(p).min(curve.eval(t1).distance(p)) > 4.0 * tol::LINEAR
     4318 boolean.rs:3077:24           3896      422   9.773  if !body.edge_box(e).grow(RIM).contains(p)
     4192 nary.rs:879:20                161     4031   3.841  if outside(&overlap(&index[i].bbox, &index[j].bbox).grow(tol::LINEAR)) {
     4129 nary.rs:2038:14              3586      543  13.151  (!any_intersect && other > side && l < -1e-3).then_some(true)
     3909 nary.rs:3461:24              2572     1337  34.203  if o.curve.eval(t).distance(mid) <= tol::LINEAR {
     3587 boolean.rs:3207:20           2269     1318  36.744  if w[1] - w[0] <= 1e-12 * (1.0 + edge.range.width()) {
     3586 nary.rs:2038:32              1081     2505  30.145  (!any_intersect && other > side && l < -1e-3).then_some(true)
     3382 nary.rs:2894:12              3378        4   0.118  if f0.abs().max(f1.abs()).max(f2.abs()) <= 1e-12 {
     3310 nary.rs:1240:32              3225       85   2.568  if let Some((_, tol)) = on_edge(body, e, hit.point) {
     2972 boolean.rs:3193:24           2438      534  17.968  if let Some((s, tol)) = on_edge(body, e, p) {
     2951 classify.rs:1089:24            23     2928   0.779  if at.abs() <= tol::LINEAR && (l.origin - frame.origin).dot(radial) > 0.0 {
     2162 boolean.rs:4289:16             24     2138   1.110  if d <= 1e-3 * r || (d - (r1 + r2)).abs() <= 1e-3 * r || (d - (r1 - r2).abs()).abs() <= 1e-3 * 
     2039 classify.rs:582:24           1823      216  10.593  && t > e.range.lo + 1e-9 * (1.0 + e.range.width())
     1865 boolean.rs:1939:16           1836       29   1.555  if other.curve.eval(t).distance(mid) <= tol::LINEAR {
     1828 ssi.rs:335:8                 1053      775  42.396  if a.cross(d.frame.z()).length() > tol::ANGULAR {
     1817 boolean.rs:4408:68             52     1765   2.862  matches!((normal(f), normal(g)), (Some(x), Some(y)) if x.cross(y).length() <= 1e-6)
     1676 ssi.rs:321:8                  416     1260  24.821  if a.cross(d.frame.z()).length() > tol::ANGULAR {
     1260 ssi.rs:326:5                  558      702  44.286  off.length() <= tol::LINEAR && (c.radius - d.radius).abs() <= tol::LINEAR
     1053 ssi.rs:387:8                   30     1023   2.849  if gap > tol::LINEAR {
     1023 ssi.rs:414:8                  824      199  19.453  if (r - d.radius).abs() > tol::LINEAR || (w.dot(n) / len).abs() > tol::LINEAR {
      906 boolean.rs:3270:20             18      888   1.987  && (0..2).all(|k| index[k].face_box(faces[k]).grow(tol::LINEAR).contains(p))
      896 boolean.rs:2513:20             12      884   1.339  if s.abs() <= tol::ANGULAR || s * sign < 0.0 {
      884 boolean.rs:2513:47            220      664  24.887  if s.abs() <= tol::ANGULAR || s * sign < 0.0 {
      775 ssi.rs:346:8                   10      765   1.290  if dist <= tol::LINEAR {
      765 ssi.rs:353:8                   55      710   7.190  if dist > r1 + r2 + tol::LINEAR || dist < (r1 - r2).abs() - tol::LINEAR {
      710 ssi.rs:353:40                 228      482  32.113  if dist > r1 + r2 + tol::LINEAR || dist < (r1 - r2).abs() - tol::LINEAR {
      636 boolean.rs:3079:29            157      479  24.686  x.on == CutOn::Edge(side, e) && x.point.distance(p) <= x.tol.max(tol::LINEAR)
      495 nary.rs:1316:24               391      104  21.010  if sg.curve.eval(t).distance(p) <= tol::LINEAR
      422 boolean.rs:3088:28            420        2   0.474  if edge.curve.eval(edge.curve.closest(p, edge.range)).distance(p) > RIM {
      295 classify.rs:839:24            247       48  16.271  .find(|&y| y >= w.lo - 1e-12 && y <= w.hi + 1e-12)
      201 boolean.rs:2985:24             26      175  12.935  if !body.edge_box(e).grow(1e-3).contains(h.point) || on_edge(body, e, h.point).is_some() {
      199 ssi.rs:414:46                 158       41  20.603  if (r - d.radius).abs() > tol::LINEAR || (w.dot(n) / len).abs() > tol::LINEAR {
      175 boolean.rs:2985:74            173        2   1.143  if !body.edge_box(e).grow(1e-3).contains(h.point) || on_edge(body, e, h.point).is_some() {
      157 boolean.rs:4679:32             61       96  38.854  && !hits.iter().any(|h| h.point.distance(p) <= tol::LINEAR)
      106 classify.rs:593:32              2      104   1.887  && box_distance(b, p) <= tol::LINEAR
       57 boolean.rs:2952:25             41       16  28.070  g * inward >= 0.0 || g.abs() <= 1e-7
       52 classify.rs:588:24             36       16  30.769  && d > 1e-3 * tol::LINEAR
       44 shape.rs:493:20                14       30  31.818  if c.frame.z().cross(frame.z()).length() > 1e-12 || off > 1e-6 {
       32 boolean.rs:4984:16             16       16  50.000  if !out.is_empty() && deep <= tol::LINEAR && !rung.chords {
       30 shape.rs:493:69                15       15  50.000  if c.frame.z().cross(frame.z()).length() > 1e-12 || off > 1e-6 {
       25 boolean.rs:3359:24             20        5  20.000  && let Some((s, tol)) = onto(&cuts[from], cuts[to].point)
       23 classify.rs:1089:51             2       21   8.696  if at.abs() <= tol::LINEAR && (l.origin - frame.origin).dot(radial) > 0.0 {
       16 boolean.rs:4984:58              8        8  50.000  if !out.is_empty() && deep <= tol::LINEAR && !rung.chords {
```

## One-sided on the flat corpus (50) and never evaluated (38)

Listed in flat-g11-sites.tsv beside this note (the full tolsites.py output). A one-sided site is a
tolerance test whose other outcome the flat corpus never needs: its proof obligation is that
outcome's soundness, with no evidence either way; a never-evaluated site is curved-only or dead
on flat input.

## Triage, batch 1 (the twelve busiest live flat sites; line numbers at main f04c533)

An added class **N**: the test only names (the edge or vertex names E.names asks about), and no
geometry depends on it.

| site | class | the argument, and what is still owed |
|---|---|---|
| classify.rs:1234 `di.abs() < 1e-300` (ray_hits_box slab) | S | `ray_hits_box` is a filter (a box missed by the exact ray needs no test). A component below 1e-300 is taken as parallel: to cross a slab whose gap is at least the coordinates' resolution under InputSize (no nonzero below INPUT_FLOOR = 2^-480 ≫ 1e-300) the ray would go past every box of the model on the other axes first. Owed: that sentence as a lemma over InputSize. |
| boolean.rs:5095 `di.abs() < 1e-300` | S | the same guard, in boolean.rs's box test: same lemma. |
| boolean.rs:289 `on_edge`'s `d <= edge.tol` | T | the base tolerance predicate: a cut accepted here is within `edge.tol` of the stored edge, itself within δ of its exact meet (Faithful input), so within δ + `edge.tol` of it. Owed: the δ-accumulation lemma (the core of class T: how far a chain of tolerance merges may carry a vertex, and the bound that keeps it ≤ δ in the result), then each T site cites it. |
| boolean.rs:3057–3058 trim's on-edge accept (with the box test) | T | `on_edge` (above), plus the box-only accept proven harmless (G14.lean, given D4). |
| boolean.rs:2877 a line within 1e-6 of a straight edge's direction, both its ends within `vx.tol` of the curve: they run together | T | proven geometry: `TolOn.near_segment` gives the whole edge within `vx.tol` of the curve's line when both its ends are. Owed: cite it here, and the δ budget (vx.tol ≤ δ). |
| boolean.rs:2911 an edge's end within `vx.tol` of the curve is a cut at that vertex | T | a vertex merge within its own tolerance: the δ-accumulation lemma. |
| boolean.rs:5019 `coincide`: an edge's end within `vx.tol` of the other's carrier | T | same. |
| nary.rs:1514 a vertex and a cut within `max(vx.tol, c.tol)` are one | T | same, with both tolerances: the lemma must take the larger. |
| unify.rs:298 the two faces' keys differ by > 1e-3: not one surface | S | a loose pre-filter before `same_surface` decides. Owed: that two faces of one surface (within tolerance) have keys within 1e-3 (a bound on the key's sensitivity under InputSize), else the filter could drop a true merge. |
| boolean.rs:3521 trim_in's sampler skips samples within 4·LINEAR of a stretch end | U | chooses where the stretch is sampled, not what a sample says. Whether the samples kept decide the stretch exactly is the open part of trim_in (B2's `off_near_ends` covers one shape). |
| boolean.rs:3611 a stretch with `t1 - t0 <= 1e-12` is skipped | T | for a line (t in mm) the stretch is shorter than 1e-12 ≪ MIN_LENGTH = LINEAR: dropping it moves nothing by more than δ. Owed: the same for a conic's parameter (radius × angle). |
| boolean.rs:3638 `edge_name`'s `near` (cuts within each other's tolerance named together) | N | names only: E.names, not geometry. |

So far: 3 S (each owing one bound lemma), 7 T (all resting on one δ-accumulation lemma plus
`near_segment`), 1 U, 1 N. The δ-accumulation lemma is the next proof: it would close the T class
for every site that merges within a stored tolerance.

## Triage, batch 2 (the next twelve busiest live flat sites; line numbers at main f04c533)

| site | class | the argument, and what is still owed |
|---|---|---|
| boolean.rs:4728 an imprint stretch with `w[1] - w[0] <= 1e-12 (1 + width)` is skipped | T | as boolean.rs:3611: a stretch that short moves nothing by more than δ. Owed: the same bound for a conic's parameter (radius × angle). |
| boolean.rs:2257 `grown_tols` records a vertex's reach only where `d > vertex.tol` | S | exact bookkeeping: `grown_tol` takes the max of the vertex's tolerance and every recorded reach, so skipping a reach within the tolerance gives the same max. Nothing is owed beyond that sentence. |
| boolean.rs:4695 imprint: a hit of another body's edge accepted by `on_edge` | T | the base predicate (boolean.rs:289): the δ-accumulation lemma. |
| boolean.rs:2715 trim on a plane: `plane_meet`'s hits accepted by `on_edge` | T | as boolean.rs:3057 (`on_edge`, then the box-only accept of G14.lean). |
| boolean.rs:3337 (first test) two neighbouring cuts within `1.2 max(tol)` are left for the dedup | T | they are merged by the dedup below, within the larger tolerance: the δ-accumulation lemma, taking the larger tolerance (as nary.rs:1514). Owed: why 1.2 and not 1.0 (the slack between this test and the dedup's own `reach`). |
| boolean.rs:3337 (second test) cuts more than 0.3 mm apart are never one shallow pass | U | an assumption, kept as one (coordinator's ruling, 2026-11-25). See "The 0.3 mm shallow-pass bound" below. |
| boolean.rs:3521 (second column) trim_in's sampler, the 4·LINEAR end test on the golden-section samples | U | batch 1's 3521, its other operand: it chooses where samples go, not what they say. Same open part of trim_in. |
| shape.rs:234 `on_sweep`: a sweep within 1e-12 rad of a whole turn is whole | S | shape.rs is a screen: In or Out only clear of the boundary by a margin, else Near, and the caller asks the B-rep (shape.rs's header, nary.rs:509, 1971, 2145). A gap of 1e-12 rad is r·1e-12 of arc. Owed: r·1e-12 ≤ the margin under InputSize (r ≤ the model box). |
| shape.rs:131 the crossing count's ends test, for an arc not within 1e-12 rad of whole | S | same screen: a ray grazing an arc's end returns None (undecided, the caller asks the B-rep). Same lemma. |
| classify.rs:1104 a ray hit within LINEAR of the plane through the axis, or 10·LINEAR of the point along the ray, is inconclusive (None) | S | None only declines: the caller casts another ray or asks another test. Owed: that a hit outside those margins is counted rightly (the parity is exact up to rounding: the margins exceed the evaluation error under InputSize). |
| nary.rs:1073 a new edge whose box, grown by LINEAR, the shapes find surely outside the result is dropped | S | `outside` is `box_out` with SHAPE_MARGIN over every holder: a filter. Owed: `box_out`'s soundness at that margin (the shapes lemma; it covers nary.rs:669 too). |
| nary.rs:3576 the interior probe's step shrinks to LINEAR | U | it chooses where a face is sampled (points clear of every edge by the step less a slack), not what the sample says. Whether a point clear by `step - slack` is surely inside the face is owed: the slack is `min(1e-3 step, 0.1 LINEAR)`, below the tolerance. |

Batch 2: 5 S (shape.rs's margin lemma covers three of them; 2257 needs no lemma), 4 T (the
δ-accumulation lemma again), 3 U: the 0.3 mm shallow bound and two sampler choices (trim_in's and
the n-ary interior probe's). With batch 1, of the 24 busiest: 8 S, 11 T, 4 U, 1 N. The
δ-accumulation lemma would close 11; shape.rs's margin lemma 3; the 0.3 mm bound is the one site
that rests on an unstated input-size assumption.

## The 0.3 mm shallow-pass bound (boolean.rs:3337, second test): an explicit U

**What it decides.** Two neighbouring cuts of a curve, the curve's midpoint between them on one
cut's edge: within 0.3 mm they are collapsed to one touch point; past it both are kept and trim
reads the stretch as running along the edge. The code's comment justified 0.3 by tangency (a curve
of radius R stays within t of a tangent for sqrt(8 R t), under 0.3 mm for R ≤ 1 m).

**What it does on the corpus** (FLAT-SITES, 2026-11-25, main ded2d224, every gate part seq and
nary1, a counter on the pairs past 0.3 mm with the midpoint on an edge): 1,100+ such pairs, every
one a circle (radius 1.9–15 mm) running along an existing circular edge, gaps 0.41–14 mm. The
bound's real job is to tell a touch from a coincident run; no pair near R = 1 m was met. Under
0.3 mm main collapses coincident runs too (mixed seq 10, patterns seq 1, gaps 0.06–0.27 mm), and
its results there are right: the same topology and volume as a build keeping both ends, seq agreeing
with nary1.

**The adversarial parts.**
- Two blind holes of radius 5, floors at one depth, centres 9 to 9.9999 mm apart (coincident runs
  of 4.36 mm down to 0.045 mm): valid with the analytic volume to 1e-12 at every spacing
  (`tests/trim_shallow.rs`, which pins it).
- A cylinder of radius 2000 mm whose circle on a plate's top face dips past the plate's front edge:
  at a dip of 1e-5 the two cuts are 0.40 mm apart and the midpoint is on the edge, the pass the
  bound skips (it reaches the decision log, below). The result is valid (6 faces, `check` clean).

**Why it stays U, not fixed.** A rule that makes the bound explicit for circles (cuts kept where the
curve is the edge's own circle; otherwise joined only where sqrt(8 R_rel t) ≥ gap, R_rel =
1/|1/R − 1/R'|) gives bits identical to main but costs +0.011% in total, +0.10% on patterns nary1:
about 3,000 circle pairs past 0.3 mm a part, each tested where main tests none (proof/trim-coincident,
81f91d2a, not shipped). With no wrong answer observed, it does not qualify as a correctness exception.

**What watches it.** Under the `decisions` feature, a pass the bound skips that the midpoint would
have joined (a circle against a line with sqrt(8 R t) ≥ gap, or two circles touching inside with
sqrt(8 t R R'/|R − R'|) ≥ gap; not the curve's own circle) is counted as the site
`trim: a long shallow pass the 0.3 mm bound skips (G11 U)`. The gate parts give 0; the 2 m cylinder
above gives 1. KERNEL-STRESS's sweeps report it. If one ever comes with a wrong answer, the rule
above, or a typed refusal at that site, is brought back as a correctness exception.

**Still owed.** Curves other than lines and circles (ellipses, traced, spline, conic): their
curvature is not bounded cheaply, and the counter does not see them.
