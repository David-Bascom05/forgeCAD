/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import ForgeMesh.Funs

/-!
# What the CDT is to be proved to do (L5): the statements

Statements only (`Prop`s, no `sorry`); the proofs follow in their own files. They are about the
translated `cdt.within` itself, so about the shipped release build. Proposed to fix-mesh-asm as
the boundary with the tessellation plan's M1/M2 (their spec of a face's mesh and the refinement
rule): this file says what the triangulation is; theirs, that a settled refinement meets the
chord.

The geometry is exact, over ℚ: a double's `val`. The predicates are taken at their contract
(`OrientSound`, `InCircleSound`): sign-exact for `orient2d`, one-sided for `certainly_in_circle`.
`OrientSound` is ForgeMath's `orient2d` theorem once its slow path is proven
(verify/aeneas/forge-math: the fast path is); `InCircleSound` is the stage-A filter's bound.
-/

open Aeneas Aeneas.Std Result

namespace forge_mesh.Spec

open forge_mesh

abbrev P := forge_mesh.forge_math.vec.Point2

/-- Twice the signed area of a b c, exactly. -/
def orient (a b c : P) : ℚ :=
  (a.x.val - c.x.val) * (b.y.val - c.y.val) - (a.y.val - c.y.val) * (b.x.val - c.x.val)

/-- The exact in-circle determinant (positive: d inside the circle through a b c, ccw). -/
def incircle (a b c d : P) : ℚ :=
  let (adx, ady) := (a.x.val - d.x.val, a.y.val - d.y.val)
  let (bdx, bdy) := (b.x.val - d.x.val, b.y.val - d.y.val)
  let (cdx, cdy) := (c.x.val - d.x.val, c.y.val - d.y.val)
  (adx * adx + ady * ady) * (bdx * cdy - cdx * bdy)
    + (bdx * bdx + bdy * bdy) * (cdx * ady - adx * cdy)
    + (cdx * cdx + cdy * cdy) * (adx * bdy - bdx * ady)

/-- The predicates' precondition, `RANGE` in cdt.rs (checked there in debug builds). -/
def InRange (p : P) : Prop :=
  p.x.isFinite ∧ p.y.isFinite ∧ |p.x.val| ≤ 2 ^ 200 ∧ |p.y.val| ≤ 2 ^ 200

/-- `orient2d`'s contract: it returns, with the exact orientation's sign. -/
def OrientSound : Prop :=
  ∀ a b c : P, InRange a → InRange b → InRange c →
    ∃ v, forge_mesh.forge_math.predicates.orient2d a b c = ok v ∧
      (0 < v.val ↔ 0 < orient a b c) ∧ (v.val < 0 ↔ orient a b c < 0)

/-- `certainly_in_circle`'s contract: it returns, and says yes only when d is inside. -/
def InCircleSound : Prop :=
  ∀ a b c d : P, InRange a → InRange b → InRange c → InRange d →
    ∃ v, forge_mesh.forge_math.predicates.certainly_in_circle a b c d = ok v ∧
      (v = true → 0 < incircle a b c d)

/-- A refinement whose questions all have answers. -/
def Total {R : Type} (inst : cdt.Refine R) (r : R) : Prop :=
  (∀ a b p q, ∃ v, inst.edge r a b p q = ok v) ∧ (∀ v p, ∃ x, inst.triangle r v p = ok x) ∧
    (∃ x, inst.triangles r = ok x) ∧ (∃ x, inst.need r = ok x)

/-- The point with index i, if there is one. -/
def pt (pts : List P) (i : Std.U32) : Option P := pts[i.val]?

/-- Triangle t has a-b as an edge, either way round. -/
def HasEdge (t : List Std.U32) (a b : Std.U32) : Prop :=
  ∃ i j, t[i]? = some a ∧ t[j]? = some b ∧ (j = (i + 1) % 3 ∨ i = (j + 1) % 3)

/-- The loop edges: each loop's points in turn, the last back to the first. -/
def loopEdges (loops : List (List Std.U32)) : List (Std.U32 × Std.U32) :=
  loops.flatMap fun lp => (List.range lp.length).filterMap fun k =>
    match lp[k]?, lp[(k + 1) % lp.length]? with
    | some a, some b => some (a, b)
    | _, _ => none

/-- q strictly inside the triangle a b c (counter-clockwise). -/
def Inside (q a b c : P) : Prop := 0 < orient a b q ∧ 0 < orient b c q ∧ 0 < orient c a q

/-- A horizontal ray from q to the right crosses segment a-b (half-open in y, so a vertex on the
ray counts once). -/
def Crosses (q a b : P) : Prop :=
  (a.y.val ≤ q.y.val ∧ q.y.val < b.y.val ∧ 0 < orient a b q) ∨
    (b.y.val ≤ q.y.val ∧ q.y.val < a.y.val ∧ orient a b q < 0)

instance (q a b : P) : Decidable (Crosses q a b) := by unfold Crosses; infer_instance

/-- q lies on segment a-b. -/
def OnSegment (q a b : P) : Prop :=
  orient a b q = 0 ∧ min a.x.val b.x.val ≤ q.x.val ∧ q.x.val ≤ max a.x.val b.x.val ∧
    min a.y.val b.y.val ≤ q.y.val ∧ q.y.val ≤ max a.y.val b.y.val

/-! ## The statements, for `within` with `ROUNDS` rounds -/

variable {R : Type} (inst : cdt.Refine R)

/-- (T) It returns, never fails or runs forever: a triangulation, or a typed refusal. Unless
`as_far`, a triangulation returned is settled (an unsettled refinement is `Unsettled` or
`TooFine`). -/
def Ends : Prop :=
  OrientSound → InCircleSound →
  ∀ (points interior : Slice P) (loops : Slice (alloc.vec.Vec Std.U32)) (refine : Option R)
    (tangled as_far : Bool),
    (∀ r, refine = some r → Total inst r) →
    (∀ p ∈ points.val, InRange p) → (∀ p ∈ interior.val, p.x.isFinite ∧ p.y.isFinite) →
    ∃ res, cdt.within inst points loops interior refine tangled cdt.ROUNDS as_far = ok res ∧
      (as_far = false → ∀ t s, res = .Ok (t, s) → s = true)

/-- A triangle's three corners, if its indices name points. -/
def corners (pts : List P) (tr : Array Std.U32 3#usize) : Option (P × P × P) :=
  match tr.val with
  | [i, j, k] => match pt pts i, pt pts j, pt pts k with
    | some a, some b, some c => some (a, b, c)
    | _, _, _ => none
  | _ => none

/-- How many loop edges a ray from q crosses. -/
def crossings (points : List P) (loops : List (List Std.U32)) (q : P) : Nat :=
  (loopEdges loops).countP fun e => match pt points e.1, pt points e.2 with
    | some a, some b => decide (Crosses q a b)
    | _, _ => false

/-- What a triangulation returned for a boundary that is not tangled is. -/
structure Good (points : List P) (loops : List (List Std.U32)) (t : cdt.Triangulation) : Prop where
  /-- The input points come first, in order. -/
  prefix_points : points <+: t.points.val
  /-- Every triangle's corners are points, counter-clockwise. -/
  ccw : ∀ tr ∈ t.triangles.val, ∃ a b c, corners t.points.val tr = some (a, b, c) ∧ 0 < orient a b c
  /-- (C) Every loop edge is an edge of the triangulation. -/
  constrained : ∀ e ∈ loopEdges loops, ∃ tr ∈ t.triangles.val, HasEdge tr.val e.1 e.2
  /-- (P) No two triangles overlap. -/
  disjoint : ∀ (i j : Nat) tr tr' a b c a' b' c' (q : P), i ≠ j →
    t.triangles.val[i]? = some tr → t.triangles.val[j]? = some tr' →
    corners t.points.val tr = some (a, b, c) → corners t.points.val tr' = some (a', b', c') →
    Inside q a b c → Inside q a' b' c' → False
  /-- (P) They cover the even-odd region: a point on no loop edge is in a triangle (or on its
  edge) exactly when a ray from it crosses an odd number of loop edges. -/
  region : ∀ q : P,
    (∀ e ∈ loopEdges loops, ∀ a b, pt points e.1 = some a → pt points e.2 = some b →
      ¬ OnSegment q a b) →
    ((∃ tr ∈ t.triangles.val, ∃ a b c, corners t.points.val tr = some (a, b, c) ∧
        0 ≤ orient a b q ∧ 0 ≤ orient b c q ∧ 0 ≤ orient c a q) ↔
      Odd (crossings points loops q))

/-- (C) and (P): for a boundary that is not tangled, a triangulation returned is `Good`. -/
def Correct : Prop :=
  OrientSound → InCircleSound →
  ∀ (points interior : Slice P) (loops : Slice (alloc.vec.Vec Std.U32)) (refine : Option R)
    (as_far : Bool) t s,
    (∀ r, refine = some r → Total inst r) →
    (∀ p ∈ points.val, InRange p) →
    cdt.within inst points loops interior refine false cdt.ROUNDS as_far = ok (.Ok (t, s)) →
    Good points.val (loops.val.map (·.val)) t

/-- Triangle tr's edges, each as its two corners' indices in order round it. -/
def edgesOf (tr : Array Std.U32 3#usize) : List (Std.U32 × Std.U32) :=
  match tr.val with
  | [i, j, k] => [(i, j), (j, k), (k, i)]
  | _ => []

/-- The seam with the tessellation plan's M2 (research/tessellation-proof: "settled ⇒ every test
passed"): when a refinement settles, every triangle and every edge but the loops' passes it.
Each edge that is no loop edge gets `edge = false`; each triangle, when the refinement asks of
triangles, `triangle = false`. (Loop edges are the boundary's chords, judged by the
tessellator before the CDT is called.) -/
def Settled : Prop :=
  OrientSound → InCircleSound →
  ∀ (points interior : Slice P) (loops : Slice (alloc.vec.Vec Std.U32)) (r : R)
    (as_far : Bool) t,
    Total inst r →
    (∀ p ∈ points.val, InRange p) →
    cdt.within inst points loops interior (some r) false cdt.ROUNDS as_far = ok (.Ok (t, true)) →
    ∀ tr ∈ t.triangles.val, ∀ a b c, corners t.points.val tr = some (a, b, c) →
      (inst.triangles r = ok true → inst.triangle r tr (Array.make 3#usize [a, b, c]) = ok false) ∧
      ∀ e ∈ edgesOf tr,
        (e ∉ loopEdges (loops.val.map (·.val)) ∧ (e.2, e.1) ∉ loopEdges (loops.val.map (·.val))) →
        ∀ pa pb, pt t.points.val e.1 = some pa → pt t.points.val e.2 = some pb →
          inst.edge r e.1 e.2 pa pb = ok false

end forge_mesh.Spec
