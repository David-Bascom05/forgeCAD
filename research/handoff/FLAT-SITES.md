# FLAT-SITES handoff (2026-11-25)

FLAT-SITES worked on the flat tolerance sites of the boolean (research/decide-once G11): proofs,
measurements and adversarial tests. At the end it moved to trim speed. Direction came from the
coordinator; branches go to main through TRAIN-CONDUCTOR. This session cannot push, so every
commit was delivered as format-patch text, and the conductor checked each tree hash.

## Held by the conductor (not in main 10d87778; for train 9)

In order on the series. Each was verified by the conductor against my tree hashes. None of
them was in main as of 10d87778: shape.rs still has `TAU - 1e-12`, and flat-g11.md is absent.

- flush-per-plane: eb58ce8f
- open-shell-refuse: 25884d3e
- cone-apex-unify: 2c926b0b
- check-fine-chords: 68501ad8
- G11 docs: cbbf47db..2543a61d
- trim-shallow-u: 68f143af
- T budget: 1310c255
- delta-accum: 7e530dc1
- shape-margin: 5dd101a7, then 2af37071 (the record corrected: the angular refusal governs, not
  size)
- model-size tests: 875846c6
- shape.rs exact whole turn: 722cad0b (bits identical, gate +0.0000%)

Sent to the conductor at wind-down (to follow 1310c255 in the series):

- 7720efa0 (on 1310c255): the two sampler U sites measured, and
  interface/forge-lang/tests/trim_sampler_slivers.rs.

## Speed (the last scope)

- **speed/subdivide-reuse, 96e28018, 2e75aefa, 9bbfc7dc** (on main c135b274).
  - What it does: curve_surface_subdivide's pieces carry g at their ends, so the grouping pass
    and Illinois reuse values the search already computed.
  - Gate: bits identical, strict total −1.08% (valve −5.1%, jack −2.4%, volute −2.0%); worst
    strict line +0.0134% (hub cli).
  - 2e75aefa pins that a traced curve's first evaluation equals its later ones (procedural hints).
  - 9bbfc7dc is a clippy type alias.
  - PRE-VERIFY: see the WOUND DOWN message for the final line.
- **The pair-loop memo: not shipped.** The n-ary pair loop got an AcrossMemo per tool pair, and
  trim reused a curve's answer across an edge when an equal curve and range had already been put
  to that edge.
  - The repeats come from several faces of one tool on one surface: (fa, fb1) and (fa, fb2) give
    the same curve. They are 31% of jack's trim-path subdivide evals.
  - Bits are identical either way. But the memo also ran in seq, and its lookups cost everywhere:
    - v1 (key: side and coedge, linear search by `==`): strict total −1.71% beyond the reuse
      commit (jack edit −7.0%, mixed seq −19.5%). Over the band: coplanar seq +13.2%, valve app
      +0.62%, volute +0.06%. An edge's list grows with every curve tried on it, and each lookup
      compares against all of them.
    - v2 (the key adds a SipHash print of the curve): worse everywhere (coplanar seq +28%,
      patterns seq +20%, heatsink +9%). The hash on every edge call costs more than it saves.
  - Next try:
    - the memo in the n-ary pair loop only (`trim(.., None)` stays for seq);
    - a cheap print: the variant plus the bits of the first chain point, no SipHash;
    - or, better, dedupe at the pair level: for a fixed fa, group the fb faces whose surface and
      region give an equal curve list, and trim fa's side once.
  - The v2 diff, on c135b274, is appended below.

## Not done (in priority order)

1. **Near-tangent band rewrite of subdivide_pieces (the big win).**
   - Calls with more than 100 leaves hold 60–94% of subdivide evals on valve, volute and jack.
   - A group's answer needs only (a) whether g's sign differs at the group's two ends (the parity
     of sign changes telescopes), (b) the first sign change, and (c) the argmin of |g|. It does
     not need thousands of LINEAR/lip leaves.
   - Approach: a band-aware search (coarse certified bracketing, then Illinois at the first
     change and a bounded argmin).
   - This moves bits and G11 semantics (which point represents a touch; noise crossings inside a
     band). That makes it a high-rigor rewrite:
     - a differential over the gate, fuzz and Haas;
     - G11 re-proved;
     - adversarial near-tangent tests;
     - the old path behind a switch;
     - a second review.
2. **Cheap Hermite pre-prune in subdivide** (bits identical if sound).
   - Prune with |implicit(hermite(m))| > reach + e before any Newton, where e is a proven bound
     on |hermite − eval| per chain segment (CURVED-SITES).
   - The ceiling is the pruned fraction: 20%, 40% and 41% of loop evals on valve, volute and jack.
3. **G11 leftovers.**
   - The curved-vertex budget (task 2: on_edge accepts up to 0.091·LINEAR on curved edges, so an
     entity can take about eleven before δ).
   - Batch 3 of G11.
   - U proofs: the n-ary probe's halfway fallback, and the sliver case.
4. **Skinned::Data::point plus sin_cos** (about 45% self on volute): CURVED-SITES.

## Risks and observations

- `ZeroThickness` refuses strips 1.05e-5 to 2e-5 mm wide crossed by a bore. This is a typed
  refusal of a legitimate thin wedge, not a wrong answer (KERNEL-STRESS).
- MODEL_HALF_SIZE (5e5) is enforced only on STEP read; native parts up to InputSize reach the
  kernel. shape.rs no longer depends on size, but other sites citing the model half size should
  be checked against this.
- The decisions counters used for the U sweeps were scratch and are not in any branch.
  research/decide-once/flat-g11.md records how they were made.
- Pre-verify runs need an idle VM. A 4-core VM shared with gates or sweeps gave false budget
  overruns, and gate scratch filled the disk once.

## Appendix: the pair-loop memo, v2 diff (not shipped; see above)

```diff
diff --git a/kernel/forge-bool/src/boolean.rs b/kernel/forge-bool/src/boolean.rs
index 3c131101..33b5ea47 100644
--- a/kernel/forge-bool/src/boolean.rs
+++ b/kernel/forge-bool/src/boolean.rs
@@ -861,6 +861,43 @@ struct Scratch {
     across: AcrossMemo,
 }
 
+/// What curves met across an edge, per side and coedge: (curve, its range, the answer).
+type EdgeHits =
+    std::cell::RefCell<std::collections::HashMap<(usize, u32, u64), Vec<(Curve, Interval, CurveSurface)>>>;
+
+/// A curve's print, cheap and alike for equal curves (bits of its defining numbers), so an edge's
+/// list of curves met is searched only among curves that may be equal.
+fn print(c: &Curve, range: Interval) -> u64 {
+    use std::hash::{Hash, Hasher};
+    let mut h = std::collections::hash_map::DefaultHasher::new();
+    let p3 = |p: Point3, h: &mut std::collections::hash_map::DefaultHasher| {
+        (p.x.to_bits(), p.y.to_bits(), p.z.to_bits()).hash(h)
+    };
+    (range.lo.to_bits(), range.hi.to_bits()).hash(&mut h);
+    match c {
+        Curve::Line(l) => {
+            0u8.hash(&mut h);
+            p3(l.origin, &mut h);
+            p3(Point3::ORIGIN + l.dir.get(), &mut h);
+        }
+        Curve::Circle(k) => {
+            1u8.hash(&mut h);
+            p3(k.frame.origin, &mut h);
+            k.radius.to_bits().hash(&mut h);
+        }
+        Curve::Intersection(ic) => {
+            2u8.hash(&mut h);
+            ic.pts.len().hash(&mut h);
+            if let (Some(&a), Some(&b)) = (ic.pts.first(), ic.pts.last()) {
+                p3(a, &mut h);
+                p3(b, &mut h);
+            }
+        }
+        _ => 3u8.hash(&mut h),
+    }
+    h.finish()
+}
+
 /// `across` per side and coedge, stamped by the boolean it was worked out in: (stamp, slot in
 /// `surfaces`), small, so the lists grow cheaply with the bodies' arenas.
 #[derive(Default)]
@@ -870,32 +907,38 @@ pub(crate) struct AcrossMemo {
     surfaces: Vec<Surface>,
     /// `trim`'s cuts by parameter, its room kept from one curve to the next
     ts: Vec<(f64, usize)>,
+    /// What a curve met across an edge, per side and coedge: a curve equal to one already put to
+    /// that edge (its two faces' surfaces, met again by another face on the same surface, give
+    /// the same curve) meets it alike, with the same surface across and the same box
+    hits: EdgeHits,
 }
 
 impl AcrossMemo {
     /// A new boolean: what was kept before is stale.
-    fn next(&mut self) {
+    pub(crate) fn next(&mut self) {
         self.stamp = self.stamp.wrapping_add(1);
         if self.stamp == 0 {
             self.at = Default::default();
             self.stamp = 1;
         }
         self.surfaces.clear();
+        self.hits.get_mut().clear();
     }
-    /// `across(body, f, c)` for side `k`, worked out once per boolean.
-    fn get(&mut self, k: usize, body: &Body, f: FaceId, c: forge_topo::CoedgeId) -> R<&Surface> {
+    /// `across(body, f, c)` for side `k`, worked out once per boolean, with what curves met across
+    /// that edge so far.
+    fn get(&mut self, k: usize, body: &Body, f: FaceId, c: forge_topo::CoedgeId) -> R<(&Surface, &EdgeHits)> {
         let v = &mut self.at[k];
         if v.len() <= c.index() {
             v.resize(body.coedges.len().max(c.index() + 1), (0, 0));
         }
         let (at, slot) = v[c.index()];
         if at == self.stamp {
-            return Ok(&self.surfaces[slot as usize]);
+            return Ok((&self.surfaces[slot as usize], &self.hits));
         }
         let s = across(body, f, c)?;
         v[c.index()] = (self.stamp, self.surfaces.len() as u32);
         self.surfaces.push(s);
-        Ok(&self.surfaces[self.surfaces.len() - 1])
+        Ok((&self.surfaces[self.surfaces.len() - 1], &self.hits))
     }
 }
 
@@ -2205,7 +2248,11 @@ pub(crate) fn apply<S: crate::imprint::Src>(a: &mut Body, b: &S, plan: &Plan) {
         new_vertex: vec![None; points.len()],
     };
     let mut emap: Vec<Option<EdgeId>> = vec![None; pieces.len()];
-    let shell = if a.shells.is_empty() { a.add_shell() } else { forge_topo::ShellId(0) };
+    let shell = if a.shells.is_empty() {
+        a.add_shell()
+    } else {
+        forge_topo::ShellId(0)
+    };
     let other_shell =
         a.shells.len() > 1 && specs.iter().any(|s| s.side == 0 && a.face(s.from).shell != shell);
     for &(si, flip) in kept {
@@ -3103,11 +3150,14 @@ fn trim_in(
                 }
                 // (the memo's own copy, borrowed for this edge; without one, worked out here)
                 let own;
-                let across: &Surface = match memo.as_deref_mut() {
-                    Some(m) => m.get(k, body, f, c)?,
+                let (across, seen): (&Surface, _) = match memo.as_deref_mut() {
+                    Some(m) => {
+                        let (s, h) = m.get(k, body, f, c)?;
+                        (s, Some(h))
+                    }
                     None => {
                         own = across(body, f, c)?;
-                        &own
+                        (&own, None)
                     }
                 };
                 // only hits on the edge count: within its ends' tolerance of its box
@@ -3181,14 +3231,42 @@ fn trim_in(
                     ));
                     CurveSurface::Hits(crossing_on_face(&curve, range, body, f, edge))
                 } else {
-                    with_hairs!(curve_surface_in(&curve, range, across, near), |hairs| log::hairs(
-                        [(bodies[0], faces[0]), (bodies[1], faces[1])],
-                        body,
-                        e,
-                        (&curve, range, across),
-                        &hairs,
-                        rung,
-                    ))
+                    // (an equal curve already put to this edge: its answer, the same inputs)
+                    let key = (k, c.index() as u32, print(&curve, range));
+                    let before = seen.and_then(|h| {
+                        h.borrow().get(&key).and_then(|v| {
+                            v.iter()
+                                .find(|x| {
+                                    x.1.lo.to_bits() == range.lo.to_bits()
+                                        && x.1.hi.to_bits() == range.hi.to_bits()
+                                        && x.0 == curve
+                                })
+                                .map(|x| x.2.clone())
+                        })
+                    });
+                    match before {
+                        Some(r) => r,
+                        None => {
+                            let r = with_hairs!(curve_surface_in(&curve, range, across, near), |hairs| {
+                                log::hairs(
+                                    [(bodies[0], faces[0]), (bodies[1], faces[1])],
+                                    body,
+                                    e,
+                                    (&curve, range, across),
+                                    &hairs,
+                                    rung,
+                                )
+                            });
+                            if let Some(h) = seen {
+                                h.borrow_mut().entry(key).or_default().push((
+                                    curve.clone(),
+                                    range,
+                                    r.clone(),
+                                ));
+                            }
+                            r
+                        }
+                    }
                 };
                 decided!({
                     let sine = match (&curve, &edge.curve) {
diff --git a/kernel/forge-bool/src/nary.rs b/kernel/forge-bool/src/nary.rs
index 040377f5..0862381d 100644
--- a/kernel/forge-bool/src/nary.rs
+++ b/kernel/forge-bool/src/nary.rs
@@ -929,6 +929,9 @@ fn nary_body(
         let (a, b) = (tools[i], tools[j]);
         let (ia, ib) = (&index[i], &index[j]);
         let mut out = PairOut::default();
+        // the two tools' surfaces across their edges, and what curves met there, for this pair
+        let mut memo = crate::boolean::AcrossMemo::default();
+        memo.next();
         // a face surely clear of the other tool's solid meets none of its faces
         let clear = |f: &forge_math::Box3, k: Side| shapes.is_some_and(|sh| sh[k].box_out(f, SHAPE_MARGIN));
         let (ba, bb) = (ia.boxes(), ib.boxes());
@@ -1076,7 +1079,7 @@ fn nary_body(
                         &mut out.segs,
                         &mut out.imprints,
                         &mut out.touching,
-                        None,
+                        Some(&mut memo),
                         &mut crate::boolean::Tried::default(),
                     )?;
                 }
```
