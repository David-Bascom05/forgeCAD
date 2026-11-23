import ForgeCertify.Geom2
import ForgeCertify.Chain

/-!
# Faces as triangles

A face of the reference (`Ref.Face`: a plane and loops) is pierced by the `+x` ray from `p` when
its plane is ahead of `p` and the ray's yz projection crosses its loops an odd number of times
(`Ref.Face.pierced`). Given triangles on the face's plane whose edges, with the face's loops, make a
zero chain (`zero1`), and a point `p` nothing is tied at, the face is pierced iff an odd number
of the triangles is hit (`pierced_tris`). So `Ref.Solid.contains` is the parity of a triangle soup.
-/

namespace ForgeCertify

open ForgeExact.Ref

attribute [local instance] Classical.propDecidable

/-! ## The reference's loops, as segments -/

theorem forIn_count (P : Nat → Bool) (l : List Nat) (acc : Nat) :
    (forIn (m := Id) l acc fun i s => if P i = true then pure (ForInStep.yield (s + 1)) else pure (ForInStep.yield s)).run
      = acc + l.countP P := by
  induction l generalizing acc with
  | nil => simp
  | cons a l ih =>
    rw [List.forIn_cons, List.countP_cons]
    cases h : P a <;> simp [ih] <;> omega

theorem loop_count (P : Nat → Bool) (n acc : Nat) :
    (Id.run do
      let mut k := acc
      for i in [0:n] do
        if P i then k := k + 1
      return k) = acc + (List.range' 0 n).countP P := by
  simp only [Std.Legacy.Range.forIn_eq_forIn_range', Std.Legacy.Range.size]
  have := forIn_count P (List.range' 0 n) acc
  simp at this ⊢
  exact this

/-- A loop's segments, as `Face.pierced` walks them. -/
def loopSegs (lp : Array V3) : List Seg :=
  (List.range' 0 lp.size).map fun i => (lp[i]!, lp[(i + 1) % lp.size]!)

def faceSegs (f : Face) : List Seg := f.loops.toList.flatMap loopSegs

theorem par_eq_countP {α : Type} (l : List α) (f : α → Bool) :
    par (l.map f) = (l.countP f % 2 == 1) := by
  induction l with
  | nil => rfl
  | cons a l ih =>
    simp only [List.map_cons, par_cons, ih, List.countP_cons]
    cases f a
    · simp
    · simp only [Bool.true_xor, ite_true]
      cases h : (List.countP f l % 2 == 1) <;> simp at h ⊢ <;> omega

/-- The plane is ahead of `p` along `+x`. -/
def ahead (P : Plane) (p : V3) : Bool := decide (0 < (P.c - P.n.dot p) * P.n.x)

theorem pierced_eq (f : Face) (p : V3) :
    f.pierced p = (ahead f.plane p && par ((faceSegs f).map fun s => crossesYZ p s.1 s.2)) := by
  unfold Face.pierced ahead
  simp only
  by_cases hn : f.plane.n.x = 0
  · simp [hn]
  · simp only [show (f.plane.n.x == 0) = false by simpa using hn, Bool.false_eq_true, ite_false]
    have hsg : (sgn (f.plane.c - f.plane.n.dot p) * sgn f.plane.n.x ≤ 0) ↔
        ¬ 0 < (f.plane.c - f.plane.n.dot p) * f.plane.n.x := by
      by_cases hz : f.plane.c - f.plane.n.dot p = 0
      · rw [hz]; simp [sgn]
      · have := sgn_mul_pos_iff hz hn; constructor <;> intro h <;> grind
    by_cases ha : 0 < (f.plane.c - f.plane.n.dot p) * f.plane.n.x
    · simp only [show ¬ (sgn (f.plane.c - f.plane.n.dot p) * sgn f.plane.n.x ≤ 0) from fun h => (hsg.mp h) ha,
        ite_false, ha, decide_true, Bool.true_and]
      rw [par_eq_countP]
      congr 1
      -- the fold over loops is the count over their segments
      unfold faceSegs
      have : ∀ (ls : List (Array V3)) (acc : Nat),
          ls.foldl (fun acc lp => Id.run do
            let mut k := acc
            for i in [0:lp.size] do
              if crossesYZ p lp[i]! lp[(i + 1) % lp.size]! then k := k + 1
            return k) acc =
          acc + (ls.flatMap loopSegs).countP (fun s => crossesYZ p s.1 s.2) := by
        intro ls
        induction ls with
        | nil => simp
        | cons lp ls ih =>
          intro acc
          rw [List.foldl_cons, ih, loop_count, List.flatMap_cons, List.countP_append]
          simp only [loopSegs, List.countP_map]
          simp only [Function.comp_def]
          omega
      rw [← Array.foldl_toList, this]; simp
    · simp only [show (sgn (f.plane.c - f.plane.n.dot p) * sgn f.plane.n.x ≤ 0) from hsg.mpr ha,
        ite_true, ha, decide_false, Bool.false_and]

/-! ## Triangles in the yz projection -/

structure Tri where
  a : V3
  b : V3
  c : V3
  deriving DecidableEq, Inhabited

def Tri.segs (t : Tri) : List Seg := [(t.a, t.b), (t.b, t.c), (t.c, t.a)]

/-- Its yz projection's doubled signed area. -/
def Tri.D (t : Tri) : Rat := o2 (yz t.a) (yz t.b) (yz t.c)

/-- `p`'s projection strictly inside the triangle's. -/
def Tri.ins (t : Tri) (p : V3) : Bool := inside2 (yz t.a) (yz t.b) (yz t.c) (yz p)

/-- Nothing about the segment is tied for the ray from `p`: its line misses the ends, and `p`'s
projection is off the segment's projected line. -/
def GenSeg (p : V3) (s : Seg) : Prop :=
  p.z ≠ s.1.z ∧ p.z ≠ s.2.z ∧ (yz s.1 ≠ yz s.2 → o2 (yz p) (yz s.1) (yz s.2) ≠ 0)

/-- The point the ray from `p` picks on the line `K`. -/
def SY (p : V3) (K : Key) (q : V3) : Prop := K.d.z ≠ 0 ∧ onKey K q ∧ q.z = p.z ∧ p.y < q.y

theorem cross_form (p a b : V3) (g : GenSeg p (a, b)) :
    crossesYZ p a b = decide (a ≠ b ∧ ∃ q, SY p (key a b) q ∧ inOpen a b q) := by
  obtain ⟨ga, gb, gl⟩ := g
  by_cases hz : a.z = b.z
  · have : crossesYZ p a b = false := by
      rw [crossesYZ_eq]; unfold cr2; simp [yz, hz]
    rw [this]; apply Eq.symm; simp only [decide_eq_false_iff_not]
    rintro ⟨-, q, ⟨-, -, hqz, -⟩, μ, -, -, rfl⟩
    simp at hqz; rw [← hz] at hqz; grind
  · have hab : a ≠ b := fun e => hz (e ▸ rfl)
    have hyz : yz a ≠ yz b := fun e => hz (by simp [yz] at e; exact e.2)
    rw [crossesYZ_eq, Bool.eq_iff_iff, cr2_iff _ _ _ ga gb (gl hyz)]
    simp only [decide_eq_true_eq]
    constructor
    · rintro ⟨μ, h0, h1, hv, hu⟩
      refine ⟨hab, a + μ * (b - a), ⟨?_, onKey_lerp hab μ, ?_, ?_⟩, μ, h0, h1, rfl⟩
      · obtain ⟨s, hs, hd⟩ := key_d hab
        rw [hd]; simp only [v3_smul_z, v3_sub_z]
        exact mul_ne_zero' hs (by grind)
      · simp [yz] at hv; simp; grind
      · simp [yz] at hu; simp; grind
    · rintro ⟨-, q, ⟨-, -, hqz, hqy⟩, μ, h0, h1, rfl⟩
      refine ⟨μ, h0, h1, ?_, ?_⟩
      · simp [yz] at hqz ⊢; grind
      · simp [yz] at hqy ⊢; grind

theorem SY_unique {p : V3} {K : Key} {q q' : V3} (h : SY p K q) (h' : SY p K q') : q = q' := by
  obtain ⟨hd, hq, hz, -⟩ := h
  obtain ⟨-, hq', hz', -⟩ := h'
  apply onKey_eq hq hq'
  have e1 := congrArg V3.z hq; have e2 := congrArg V3.z hq'
  simp at e1 e2
  have : coord K.k q * K.d.z = coord K.k q' * K.d.z := by grind
  grind

/-- **Zero chains and the ray**: the segments of a zero chain the ray from a generic point
crosses are even in number. -/
theorem cross_zero (p : V3) (C : List Seg) (hz : zero1 C = true) (g : ∀ s ∈ C, GenSeg p s) :
    par (C.map fun s => crossesYZ p s.1 s.2) = false := by
  rw [par_map_congr C _ _ (fun s hs => cross_form p s.1 s.2 (g s hs))]
  apply chainZ C hz (SY p)
  · intro s _ _ q q' h h'; exact SY_unique h h'
  · intro s _ _ q h; exact h.2.1
  · intro s hs _ q h
    obtain ⟨ga, gb, -⟩ := g s hs
    exact ⟨fun e => ga (by rw [← e]; exact h.2.2.1.symm), fun e => gb (by rw [← e]; exact h.2.2.1.symm)⟩

theorem yz_ne_of_D {t : Tri} (h : t.D ≠ 0) : yz t.a ≠ yz t.b ∧ yz t.b ≠ yz t.c ∧ yz t.c ≠ yz t.a := by
  unfold Tri.D at h
  refine ⟨fun e => h ?_, fun e => h ?_, fun e => h ?_⟩ <;> rw [e] <;> unfold o2 <;> grind

/-- A triangle's three crossings are its interior. -/
theorem tri_cross (t : Tri) (p : V3) (hD : t.D ≠ 0) (g : ∀ s ∈ t.segs, GenSeg p s) :
    par (t.segs.map fun s => crossesYZ p s.1 s.2) = t.ins p := by
  obtain ⟨n1, n2, n3⟩ := yz_ne_of_D hD
  have gab := g (t.a, t.b) (by simp [Tri.segs])
  have gbc := g (t.b, t.c) (by simp [Tri.segs])
  have gca := g (t.c, t.a) (by simp [Tri.segs])
  obtain ⟨a1, a2, a3⟩ := gab
  obtain ⟨b1, b2, b3⟩ := gbc
  obtain ⟨c1, c2, c3⟩ := gca
  have l1 := a3 n1; have l2 := b3 n2; have l3 := c3 n3
  simp only [Tri.segs, List.map_cons, List.map_nil, Tri.ins]
  rw [crossesYZ_eq, crossesYZ_eq, crossesYZ_eq]
  have e1 := cr2_iff (yz p) (yz t.a) (yz t.b) a1 a2 l1
  have e2 := cr2_iff (yz p) (yz t.b) (yz t.c) b1 b2 l2
  have e3 := cr2_iff (yz p) (yz t.c) (yz t.a) c1 c2 l3
  rw [tri_ray (yz t.a) (yz t.b) (yz t.c) (yz p) hD a1 b1 c1 l2
    (by rw [o2_cyc]; rw [o2_cyc] at l3; rw [o2_cyc] at l3; exact (by unfold o2 at *; grind))
    (by rw [← o2_cyc]; exact l1)]
  have d1 : cr2 (yz p) (yz t.a) (yz t.b) = decide (gc (yz p) (yz t.a) (yz t.b)) := by
    rw [Bool.eq_iff_iff, e1]; simp
  have d2 : cr2 (yz p) (yz t.b) (yz t.c) = decide (gc (yz p) (yz t.b) (yz t.c)) := by
    rw [Bool.eq_iff_iff, e2]; simp
  have d3 : cr2 (yz p) (yz t.c) (yz t.a) = decide (gc (yz p) (yz t.c) (yz t.a)) := by
    rw [Bool.eq_iff_iff, e3]; simp
  rw [d1, d2, d3]
  simp only [par_cons, par_nil, Bool.xor_false]
  cases decide (gc (yz p) (yz t.a) (yz t.b)) <;> cases decide (gc (yz p) (yz t.b) (yz t.c)) <;>
    cases decide (gc (yz p) (yz t.c) (yz t.a)) <;> rfl

/-- **A face is its triangles.** Triangles tiling a face (with its loops, a zero chain), each with
a proper projection, at a point nothing is tied at: the face's crossing parity is the parity of
the triangles containing the point. -/
theorem face_tris (f : Face) (T : List Tri) (p : V3)
    (hz : zero1 (faceSegs f ++ T.flatMap Tri.segs) = true) (hD : ∀ t ∈ T, t.D ≠ 0)
    (g : ∀ s ∈ faceSegs f ++ T.flatMap Tri.segs, GenSeg p s) :
    par ((faceSegs f).map fun s => crossesYZ p s.1 s.2) = par (T.map fun t => t.ins p) := by
  have h0 := cross_zero p _ hz g
  rw [List.map_append, par_append, par_flatMap] at h0
  rw [par_map_congr T _ _ (fun t ht => (tri_cross t p (hD t ht)
    (fun s hs => g s (List.mem_append_right _ (List.mem_flatMap.mpr ⟨t, ht, hs⟩)))).symm)]
  revert h0; cases par ((faceSegs f).map fun s => crossesYZ p s.1 s.2) <;>
    cases par (T.map fun t => par ((Tri.segs t).map fun s => crossesYZ p s.1 s.2)) <;> simp

end ForgeCertify
