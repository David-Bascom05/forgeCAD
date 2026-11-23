# A proven outer box for through tools: what is sound, and what is not

2026-11-23. Item (b) of the ops spec. A through tool (`hole::reach`, H1) needs a box that is never
smaller than the part. `hole::body_box` builds it from four inputs:
- the vertices' hull;
- each curved edge's box (`Curve::bbox`, or a circle's or ellipse's whole box);
- each sphere's or torus's whole box;
- each non-ruled procedural face's own box (`Body::face_box` / `uv::procedural_box`).

This note records which of those inputs are proven outer bounds today, read at bf8790a with file
references.

## Sound now

These are proven, or exact by construction:
- vertices;
- line edges (their ends);
- circles and ellipses (the whole box);
- spheres and tori (centre ± r, centre ± (major + minor));
- plane, cylinder, cone and ruled faces (a line of the surface through any point of the face meets
  its boundary on both sides).

These procedural curves are sound by interval or closed-form maths:
- Helix (screw.rs:920);
- coaxial Round (coaxial.rs:455);
- Wrap (wrap.rs:437);
- SplineCurve, from Bernstein control points (spline.rs:681);
- corner Cubic, from its control hull (corner.rs:762).

These procedural surfaces have a sound `bbox` over a uv box:
- Ruled, Screw, Skinned (interval weights), Extruded, Spun;
- Nurbs with positive weights;
- HolePatch (rational cells).

## Not proven

1. **Intersection curves (`IntCurve`, intcurve.rs:501).** The box is the chain's points grown by
   `sag`. `sag` is measured at chord midpoints only, ×1.5 (intcurve.rs:998). A deviation off the
   midpoint can go unmeasured. `speed_bound` is a heuristic too.
   - Inherited by: blend Pipe (blend.rs:1780), Spring (blend.rs:1139), blend Fence (blend.rs:1241)
     and ChordCurve (smooth.rs:452) whenever their spine is an IntCurve.
2. **`uv::procedural_box` (uv.rs:353).**
   - The face's uv region comes from sampled boundary polylines, padded by 5% of a column's width.
   - A boundary bulging further in uv is missed, and so is the interior behind it. The edge boxes
     cover the boundary, but not the interior.
3. **Bounds built on a sampled derivative.**
   - VarBlend and its Cut and Track (varblend.rs:2540, 3016, 2672) are sound only if `segs[].vb`
     and `speed_bound` are true bounds.
   - Swept (swept.rs:532) only if `rates` is.
   - corner SurfaceBezier's `jacobian_bound` is "the largest sampled, doubled" (corner.rs:2682).
4. **heal Iso (heal.rs:1377).** The uv box comes from the two end uvs, so it is sound only if the uv
   is monotone along the curve.

## The rule, and the plan

The rule: an input is used as an outer bound only where it is proven. Where it is not proven, the
through tool takes a coarser input that is proven.

1. **Faces.** For a non-ruled procedural face, use the surface's sound `bbox` over its **whole
   domain**, not over a sampled uv region.
   - That is sound wherever the surface's `bbox` is: Skinned, Spun, Nurbs, HolePatch.
   - It is looser, and a looser box changes no cut (H1). So item 2 leaves the path.
   - For Offset, Pipe, Tube, Fence and VarBlend, the whole-domain box is only as good as their
     spine's box. They wait on 3.
2. **Edges of sound kinds** keep `Curve::bbox`.
3. **Intersection curves.** A proven enclosure of each chord's arc is the trace session's work: its
   certified step (the Krawczyk lemma) bounds the true curve near each chain point.
   - Until that lands, use the faces. An intersection curve lies on both of its faces' surfaces,
     so it lies in either surface's sound whole-domain box when that box is bounded: a sphere's, a
     torus's, a Skinned, Spun, Nurbs or HolePatch surface's.
   - Planes, cylinders and cones have unbounded domains, and a face's own hull includes this very
     edge, so they give nothing here.
   - Where neither face gives a bounded proven box, the edge's box stays today's sampled one. That
     case is listed as unproven. It is not refused: nothing that builds today may start to fail.
     It is closed when the trace session's certified enclosure is used.
4. **Derivative-sampled kinds (3, 4).** They stay as they are, listed as open, until their owners
   prove the bounds: varblend (fillet sessions), corner (fillet sessions), heal (import).
