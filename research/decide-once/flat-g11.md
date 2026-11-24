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
