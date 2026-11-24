import ForgeCertify.Check

/-!
# Facts the soundness proof uses about the checker's pieces

* `contains_par` : the reference's membership is the parity of the faces pierced;
* `par_group` : a parity over a list is the parity, over its keys, of the parities of each key's
  part;
* `ins_canonTri` : a triangle's interior does not depend on its vertices' order;
* `ahead_canon` : nor does a plane's being ahead on its scaling;
* `bary_pos` : an affine function nonnegative at a triangle's vertices, positive at one, is
  positive inside; so `sep2` keeps interiors apart and `sepOK` gives `Sep`;
* `wpoint_inside` : a witness is inside its atom, on the atom's plane.
-/

namespace ForgeCertify

open ForgeExact.Ref

attribute [local instance] Classical.propDecidable

/-! ## Membership as a parity -/

theorem foldl_count {α : Type} (P : α → Bool) (l : List α) (acc : Nat) :
    l.foldl (fun k f => if P f then k + 1 else k) acc = acc + l.countP P := by
  induction l generalizing acc with
  | nil => simp
  | cons a l ih => rw [List.foldl_cons, ih, List.countP_cons]; cases P a <;> simp <;> omega

theorem contains_par (op : String) (fs : List Face) (x : V3) :
    (Solid.mk op fs.toArray).contains x = par (fs.map fun f => f.pierced x) := by
  unfold Solid.contains
  rw [← Array.foldl_toList, List.toList_toArray, foldl_count, Nat.zero_add, par_eq_countP]

/-! ## Grouping a parity by keys -/

theorem par_group {α β : Type} [DecidableEq β] (l : List α) (key : α → β) (K : List β)
    (hK : K.Nodup) (hcov : ∀ a ∈ l, key a ∈ K) (f : α → Bool) :
    par (l.map f) = par (K.map fun k => par ((l.filter fun a => decide (key a = k)).map f)) := by
  induction l with
  | nil => apply Eq.symm; apply par_all_false; intro b hb; obtain ⟨k, _, rfl⟩ := List.mem_map.mp hb; simp
  | cons a l ih =>
    simp only [List.map_cons, par_cons, List.filter_cons]
    rw [ih (fun b hb => hcov b (List.mem_cons_of_mem _ hb))]
    have : (K.map fun k => par ((if decide (key a = k) = true then a :: l.filter (fun b => decide (key b = k))
        else l.filter fun b => decide (key b = k)).map f)) =
        K.map fun k => (decide (key a = k) && f a) ^^ par ((l.filter fun b => decide (key b = k)).map f) := by
      apply List.map_congr_left; intro k _
      by_cases h : key a = k <;> simp [h]
    rw [this, par_map_xor]
    have hs : par (K.map fun k => decide (key a = k) && f a) = f a := by
      rw [par_single K hK (key a) (hcov a (List.mem_cons_self ..)) _ (fun k _ hk => by
        have : key a ≠ k := Ne.symm hk
        simp [this])]
      simp
    rw [hs]

/-! ## Triangles -/

theorem o2_swap12 (p q r : P2) : o2 q p r = -o2 p q r := by unfold o2; grind
theorem o2_swap23 (p q r : P2) : o2 p r q = -o2 p q r := by unfold o2; grind
theorem o2_swap13 (p q r : P2) : o2 r q p = -o2 p q r := by unfold o2; grind

theorem ins_perm (a b c p : V3) :
    (Tri.mk b a c).ins p = (Tri.mk a b c).ins p ∧ (Tri.mk a c b).ins p = (Tri.mk a b c).ins p ∧
    (Tri.mk c b a).ins p = (Tri.mk a b c).ins p ∧ (Tri.mk b c a).ins p = (Tri.mk a b c).ins p ∧
    (Tri.mk c a b).ins p = (Tri.mk a b c).ins p := by
  simp only [Tri.ins, inside2]
  obtain ⟨A, B, C, X⟩ : ∃ A B C X, A = yz a ∧ B = yz b ∧ C = yz c ∧ X = yz p := ⟨_, _, _, _, rfl, rfl, rfl, rfl⟩
  have k : ∀ (u v w x y z : Rat), u = x → v = y → w = z →
      (decide (0 < u) && decide (0 < v) && decide (0 < w)) =
        (decide (0 < x) && decide (0 < y) && decide (0 < z)) := by intros; subst_vars; rfl
  have k' : ∀ (x y z : Rat), (decide (0 < x) && decide (0 < y) && decide (0 < z)) =
      (decide (0 < y) && decide (0 < x) && decide (0 < z)) := by
    intro x y z; cases decide (0 < x) <;> cases decide (0 < y) <;> cases decide (0 < z) <;> rfl
  have k'' : ∀ (x y z : Rat), (decide (0 < x) && decide (0 < y) && decide (0 < z)) =
      (decide (0 < x) && decide (0 < z) && decide (0 < y)) := by
    intro x y z; cases decide (0 < x) <;> cases decide (0 < y) <;> cases decide (0 < z) <;> rfl
  refine ⟨?_, ?_, ?_, ?_, ?_⟩
  · rw [k', k _ _ _ (o2 (yz a) (yz b) (yz c) * o2 (yz p) (yz b) (yz c))
      (o2 (yz a) (yz b) (yz c) * o2 (yz a) (yz p) (yz c)) (o2 (yz a) (yz b) (yz c) * o2 (yz a) (yz b) (yz p))
      (by unfold o2; grind) (by unfold o2; grind) (by unfold o2; grind)]
  · rw [k'', k _ _ _ (o2 (yz a) (yz b) (yz c) * o2 (yz p) (yz b) (yz c))
      (o2 (yz a) (yz b) (yz c) * o2 (yz a) (yz p) (yz c)) (o2 (yz a) (yz b) (yz c) * o2 (yz a) (yz b) (yz p))
      (by unfold o2; grind) (by unfold o2; grind) (by unfold o2; grind)]
  · rw [show ∀ x y z : Rat, (decide (0 < x) && decide (0 < y) && decide (0 < z)) =
        (decide (0 < z) && decide (0 < y) && decide (0 < x)) from fun x y z => by
          cases decide (0 < x) <;> cases decide (0 < y) <;> cases decide (0 < z) <;> rfl]
    exact k _ _ _ _ _ _ (by unfold o2; grind) (by unfold o2; grind) (by unfold o2; grind)
  · rw [show ∀ x y z : Rat, (decide (0 < x) && decide (0 < y) && decide (0 < z)) =
        (decide (0 < z) && decide (0 < x) && decide (0 < y)) from fun x y z => by
          cases decide (0 < x) <;> cases decide (0 < y) <;> cases decide (0 < z) <;> rfl]
    exact k _ _ _ _ _ _ (by unfold o2; grind) (by unfold o2; grind) (by unfold o2; grind)
  · rw [show ∀ x y z : Rat, (decide (0 < x) && decide (0 < y) && decide (0 < z)) =
        (decide (0 < y) && decide (0 < z) && decide (0 < x)) from fun x y z => by
          cases decide (0 < x) <;> cases decide (0 < y) <;> cases decide (0 < z) <;> rfl]
    exact k _ _ _ _ _ _ (by unfold o2; grind) (by unfold o2; grind) (by unfold o2; grind)

theorem ins_canonTri (t : Tri) (p : V3) : (canonTri t).ins p = t.ins p := by
  obtain ⟨a, b, c⟩ := t
  obtain ⟨p1, p2, p3, p4, p5⟩ := ins_perm a b c p
  unfold canonTri sort2
  by_cases h1 : v3le a b <;> by_cases h2 : v3le b c <;> by_cases h3 : v3le a c <;>
    by_cases h4 : v3le b a <;> by_cases h5 : v3le c a <;> by_cases h6 : v3le c b <;>
    simp only [h1, h2, h3, h4, h5, h6, ite_true, ite_false, Bool.false_eq_true] <;>
    first | rfl | exact p1 | exact p2 | exact p3 | exact p4 | exact p5

/-- The vertices of the sorted triangle are the triangle's. -/
theorem canonTri_verts (t : Tri) :
    let s := canonTri t
    (s.a = t.a ∨ s.a = t.b ∨ s.a = t.c) ∧ (s.b = t.a ∨ s.b = t.b ∨ s.b = t.c) ∧
      (s.c = t.a ∨ s.c = t.b ∨ s.c = t.c) := by
  obtain ⟨a, b, c⟩ := t
  unfold canonTri sort2
  by_cases h1 : v3le a b <;> by_cases h2 : v3le b c <;> by_cases h3 : v3le a c <;>
    by_cases h4 : v3le b a <;> by_cases h5 : v3le c a <;> by_cases h6 : v3le c b <;>
    simp [h1, h2, h3, h4, h5, h6]

/-- And its area is the triangle's, up to sign. -/
theorem canonTri_D (t : Tri) : (canonTri t).D = t.D ∨ (canonTri t).D = -t.D := by
  obtain ⟨a, b, c⟩ := t
  unfold canonTri sort2 Tri.D
  by_cases h1 : v3le a b <;> by_cases h2 : v3le b c <;> by_cases h3 : v3le a c <;>
    by_cases h4 : v3le b a <;> by_cases h5 : v3le c a <;> by_cases h6 : v3le c b <;>
    simp [h1, h2, h3, h4, h5, h6] <;> unfold o2 <;> first | (left; grind) | (right; grind)

/-! ## Planes -/

theorem canon_scale (Q : Plane) (hn : Q.n ≠ ⟨0, 0, 0⟩) :
    ∃ s : Rat, s ≠ 0 ∧ Q.canon.n = (1 / s) * Q.n ∧ Q.canon.c = Q.c / s := by
  unfold Plane.canon
  refine ⟨if Q.n.x != 0 then Q.n.x else if Q.n.y != 0 then Q.n.y else Q.n.z, ?_, rfl, rfl⟩
  by_cases hx : Q.n.x = 0
  · by_cases hy : Q.n.y = 0
    · have hz : Q.n.z ≠ 0 := by intro hz; apply hn; cases h : Q.n; simp_all
      simp [hx, hy, hz]
    · simp [hx, hy]
  · simp [hx]

theorem ahead_canon (Q : Plane) (hn : Q.n ≠ ⟨0, 0, 0⟩) (x : V3) : ahead Q.canon x = ahead Q x := by
  obtain ⟨s, hs, hn', hc⟩ := canon_scale Q hn
  unfold ahead
  apply decide_eq_decide.mpr
  rw [hn', hc]
  simp only [V3.dot, v3_smul_x, v3_smul_y, v3_smul_z]
  have e : (Q.c / s - (1 / s * Q.n.x * x.x + 1 / s * Q.n.y * x.y + 1 / s * Q.n.z * x.z)) * (1 / s * Q.n.x) =
      ((Q.c - (Q.n.x * x.x + Q.n.y * x.y + Q.n.z * x.z)) * Q.n.x) * (1 / s * (1 / s)) := by
    simp only [Rat.div_def]; grind
  rw [e]
  have hpos : 0 < 1 / s * (1 / s) := sq_pos' (by
    intro h0; have := Rat.div_mul_cancel (a := 1) hs; rw [h0, Rat.zero_mul] at this; exact absurd this.symm (by decide))
  constructor
  · intro h; exact (Rat.mul_pos_iff_of_pos_right hpos).mp h
  · intro h; exact Rat.mul_pos h hpos

theorem canon_nx (Q : Plane) (h : Q.n.x ≠ 0) : Q.canon.n.x = 1 := by
  unfold Plane.canon; simp [h]; exact Rat.div_mul_cancel (a := 1) h

/-! ## Barycentric positivity -/

/-- An affine function of the plane's points. -/
def affine2 (α β γ : Rat) (p : P2) : Rat := α * p.u + β * p.v + γ

theorem bary_affine (a b c p : P2) (α β γ : Rat) :
    o2 a b c * affine2 α β γ p = o2 p b c * affine2 α β γ a + o2 a p c * affine2 α β γ b +
      o2 a b p * affine2 α β γ c := by
  unfold affine2
  have hu := bary_u a b c p; have hv := bary_v a b c p; have hs := bary_sum a b c p
  have : o2 a b c * (α * p.u + β * p.v + γ) = α * (o2 a b c * p.u) + β * (o2 a b c * p.v) + γ * o2 a b c := by
    grind
  rw [this, hu, hv, ← hs]; grind

theorem bary_pos (a b c p : P2) (α β γ : Rat) (h : inside2 a b c p = true)
    (ha : 0 ≤ affine2 α β γ a) (hb : 0 ≤ affine2 α β γ b) (hc : 0 ≤ affine2 α β γ c)
    (h1 : 0 < affine2 α β γ a ∨ 0 < affine2 α β γ b ∨ 0 < affine2 α β γ c) : 0 < affine2 α β γ p := by
  simp only [inside2, Bool.and_eq_true, decide_eq_true_eq] at h
  obtain ⟨⟨w1, w2⟩, w3⟩ := h
  have hD : o2 a b c ≠ 0 := by intro e; rw [e] at w1; simp at w1
  have e := bary_affine a b c p α β γ
  have e2 : o2 a b c * (o2 a b c * affine2 α β γ p) =
      o2 a b c * o2 p b c * affine2 α β γ a + o2 a b c * o2 a p c * affine2 α β γ b +
        o2 a b c * o2 a b p * affine2 α β γ c := by rw [e]; grind
  have i1 := Rat.mul_nonneg (Rat.le_of_lt w1) ha
  have i2 := Rat.mul_nonneg (Rat.le_of_lt w2) hb
  have i3 := Rat.mul_nonneg (Rat.le_of_lt w3) hc
  have pos : 0 < o2 a b c * o2 a b c * affine2 α β γ p := by
    rw [Rat.mul_assoc, e2]
    rcases h1 with h | h | h
    · have := Rat.mul_pos w1 h; grind
    · have := Rat.mul_pos w2 h; grind
    · have := Rat.mul_pos w3 h; grind
  exact (sq_mul_pos_iff hD _).mp pos

theorem bary_neg (a b c p : P2) (α β γ : Rat) (h : inside2 a b c p = true)
    (ha : affine2 α β γ a ≤ 0) (hb : affine2 α β γ b ≤ 0) (hc : affine2 α β γ c ≤ 0)
    (h1 : affine2 α β γ a < 0 ∨ affine2 α β γ b < 0 ∨ affine2 α β γ c < 0) : affine2 α β γ p < 0 := by
  have := bary_pos a b c p (-α) (-β) (-γ) h (by unfold affine2 at *; grind) (by unfold affine2 at *; grind)
    (by unfold affine2 at *; grind) (by unfold affine2 at *; grind)
  unfold affine2 at *; grind

/-- A side function is affine. -/
theorem o2_affine (s t r : P2) :
    o2 s t r = affine2 (-(t.v - s.v)) (t.u - s.u) (-(t.u - s.u) * s.v + (t.v - s.v) * s.u) r := by
  unfold o2 affine2; grind

/-- A plane's value on another plane not along `x` is affine in the yz projection. -/
theorem affine_on_nv (P H : Plane) (hnx : P.n.x ≠ 0) :
    ∃ α β γ : Rat, ∀ p, P.eval p = 0 → H.eval p = affine2 α β γ (yz p) := by
  refine ⟨H.n.y - H.n.x * P.n.y / P.n.x, H.n.z - H.n.x * P.n.z / P.n.x, H.n.x * P.c / P.n.x - H.c,
    fun p hp => ?_⟩
  simp only [affine2, yz]
  simp only [Plane.eval, V3.dot] at hp ⊢
  have e : p.x = (P.c - P.n.y * p.y - P.n.z * p.z) / P.n.x := by
    have : p.x * P.n.x = P.c - P.n.y * p.y - P.n.z * p.z := by grind
    rw [← this, Rat.mul_div_cancel hnx]
  rw [e]; simp only [Rat.div_def]; grind

end ForgeCertify
