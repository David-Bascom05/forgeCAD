# Fillet overflow: prior art for groups B, O1, A (research summary, 2026-11-21)

Sources: Parasolid/ACIS/SolidWorks docs via search snippets (q-solid.com, help.solidworks.com, tflex.com
blocked here); OCCT ChFi3d sources read directly (described, not transcribed).

## Industry rules
- Edge met is G1 (smooth): blend flows onto the neighbour (Parasolid ov_smooth, OCCT IsG1, NX Roll Over Smooth).
- Edge met is sharp: CLIFF by default (ball touches the other support and the edge): OCCT always
  (surface-restriction blend, BRepBlend_SurfRstConstRad; leaves when the face-face solution exists again);
  Parasolid/T-FLEX/NX order smooth -> cliff -> notch; SolidWorks "keep edge" ~ cliff.
- NOTCH (blend trimmed by the neighbouring face carried on) is the fallback or a user option
  (Parasolid ov_notch, SolidWorks keep surface, NX Maintain Blend).
- Mixed convexity vertex: same-convexity pair mitred (ACIS, OCCT, ours); springs meeting the pivot apart:
  ACIS takes the farther point (setback), OCCT a two-curve big/small trim; near-tangent springs: ACIS
  BL_NO_X_CURVE and OCCT fail too.
- Parasolid FD ch.76.4 (exact default overflow conditions) unread: q-solid.com blocked.

## Recommendation (one construction for B, O1, A)
CLIFF: ball touching the other support and the rim/edge curve.
- spine = offset(other support, r) ∩ tube(r, about the rim curve); circular rim -> torus, straight edge -> cylinder
  (forge-geom tube.rs exists). Traced numerically like other spines; implicit, sqrt only.
- enter where the face-face spring reaches the rim (ball centre = rim point + r * face normal; tube and offset tangent);
  leave where the face-face solution exists again (OCCT's "decroch").
- B: replaces the refusal at seam crossings beyond a rim.
- O1: face-face -> cliff -> face-face as one chain; the two-loop failure is an artefact of trimming the pipe by
  the plane (a notch construction).
- A: keep the mitre where its far point exists (matches OCCT 1e-10); else roll on the convex edge E3 (cliff:
  boss offset ∩ tube about E3); refuse only if they don't meet transversally.
- Seam independence: decide overflow on geometry only: seam edges are not boundaries; find rim crossings on
  the whole rim curve, impose seam splits afterwards (current `reseamed` retry is topology-dependent).
- Check: locally the cliff equals the morphological closing (ball stays in free space), so closing*.rs and the
  per-ball distance test are independent references; OCCT only where its volume is seam-stable (not on B parts).
- Caveat: mitre-first then cliff jumps in kind as geometry varies; cliff everywhere is the closing-consistent
  choice, mitre-first is the industry convention.
