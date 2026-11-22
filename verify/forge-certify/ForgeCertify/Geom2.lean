import ForgeExact.Ref.Geom
import ForgeCertify.IP

/-!
# The plane: rays, segments, triangles

A point of a plane is `P2` (`u`, `v`); rays run along `+u`. Two projections of space use it: `yz`
(the reference's own: its rays run along `+y` in the yz projection, `Ref.crossesYZ`), and the
plane of a face along `x` for faces that contain the `x` direction.

* `cr2` is `Ref.crossesYZ` in these coordinates (`crossesYZ_eq`), and `gc` is the geometric
  statement it decides: the ray from `p` meets the open segment. `cr2_iff` : they agree when
  nothing is tied.
* `tri_ray` : a triangle's interior is the parity of its edges the ray meets, by `ip` along the
  ray; also when `p` lies on one edge's line outside that edge (`tri_ray_on_line`).
-/

namespace ForgeCertify

open ForgeExact.Ref

attribute [local instance] Classical.propDecidable

structure P2 where
  u : Rat
  v : Rat
  deriving DecidableEq, Inhabited

/-- Twice the signed area of `p q r`. -/
def o2 (p q r : P2) : Rat := (q.u - p.u) * (r.v - p.v) - (q.v - p.v) * (r.u - p.u)

/-- `Ref.crossesYZ`, its `y` read as `u` and `z` as `v`. -/
def cr2 (q a b : P2) : Bool :=
  let aa := decide (a.v > q.v)
  let ab := decide (b.v > q.v)
  if aa == ab then false
  else
    let dz := b.v - a.v
    let d0 := (b.u - a.u) * (q.v - a.v) - (q.u - a.u) * dz
    let s := if d0 != 0 then sgn d0 else sgn (-dz)
    s * sgn dz > 0

def yz (p : V3) : P2 := ⟨p.y, p.z⟩

theorem crossesYZ_eq (q a b : V3) : crossesYZ q a b = cr2 (yz q) (yz a) (yz b) := rfl

/-- The ray from `p` along `+u` meets the open segment `ab`. -/
def gc (p a b : P2) : Prop :=
  ∃ μ : Rat, 0 < μ ∧ μ < 1 ∧ a.v + μ * (b.v - a.v) = p.v ∧ p.u < a.u + μ * (b.u - a.u)

theorem sgn_pos {q : Rat} (h : 0 < q) : sgn q = 1 := by unfold sgn; simp [h]
theorem sgn_neg {q : Rat} (h : q < 0) : sgn q = -1 := by
  unfold sgn; simp [h, show ¬ q > 0 by grind]

theorem o2_cyc (p q r : P2) : o2 p q r = o2 q r p := by unfold o2; grind
theorem o2_swap (p q r : P2) : o2 p q r = -o2 q p r := by unfold o2; grind

theorem sgn_mul_pos_iff {a b : Rat} (ha : a ≠ 0) (hb : b ≠ 0) :
    sgn a * sgn b > 0 ↔ 0 < a * b := by
  rcases lt_trichotomy' a 0 with h1 | h1 | h1 <;> rcases lt_trichotomy' b 0 with h2 | h2 | h2
  · rw [sgn_neg h1, sgn_neg h2]; simp [mul_pos_of_neg_of_neg h1 h2]
  · exact absurd h2 hb
  · rw [sgn_neg h1, sgn_pos h2]; have := mul_neg_of_neg_of_pos h1 h2; simp; grind
  · exact absurd h1 ha
  · exact absurd h1 ha
  · exact absurd h1 ha
  · rw [sgn_pos h1, sgn_neg h2]; have := mul_neg_of_pos_of_neg h1 h2; simp; grind
  · exact absurd h2 hb
  · rw [sgn_pos h1, sgn_pos h2]; simp [Rat.mul_pos h1 h2]

/-- `p.v` strictly between `a.v` and `b.v` is a strict convex combination of them. -/
theorem between_of_combo {a b p : Rat} {μ : Rat} (h0 : 0 < μ) (h1 : μ < 1) (h : a + μ * (b - a) = p) :
    (a < p ∧ p < b) ∨ (b < p ∧ p < a) ∨ (a = p ∧ b = p) := by
  rcases lt_trichotomy' a b with hab | hab | hab
  · have := Rat.mul_lt_mul_of_pos_left (show 0 < b - a by grind) h0
    have := Rat.mul_lt_mul_of_pos_left (show 0 < b - a by grind) (show 0 < 1 - μ by grind)
    grind
  · subst hab; grind
  · have := Rat.mul_lt_mul_of_pos_left (show 0 < a - b by grind) h0
    have := Rat.mul_lt_mul_of_pos_left (show 0 < a - b by grind) (show 0 < 1 - μ by grind)
    grind

/-- `cr2` decides `gc` when the ray's line misses both ends and `p` is off the segment's line. -/
theorem cr2_iff (p a b : P2) (ha : p.v ≠ a.v) (hb : p.v ≠ b.v) (hl : o2 p a b ≠ 0) :
    cr2 p a b = true ↔ gc p a b := by
  have hd0 : (b.u - a.u) * (p.v - a.v) - (p.u - a.u) * (b.v - a.v) = o2 p a b := by
    unfold o2; grind
  unfold cr2 gc
  simp only
  by_cases hs : decide (a.v > p.v) = decide (b.v > p.v)
  · simp only [hs, beq_self_eq_true, ite_true, Bool.false_eq_true, false_iff]
    rintro ⟨μ, h0, h1, hv, _⟩
    have hab : (a.v > p.v ↔ b.v > p.v) := by simpa using hs
    rcases between_of_combo h0 h1 hv with h | h | h <;> grind
  · have hs' : (a.v > p.v) ≠ (b.v > p.v) := by simpa using hs
    simp only [show (decide (a.v > p.v) == decide (b.v > p.v)) = false by simpa using hs,
      Bool.false_eq_true, ite_false]
    rw [hd0]
    simp only [show (o2 p a b != 0) = true by simpa using hl, ite_true]
    have hdz : b.v - a.v ≠ 0 := by intro h; apply hs'; grind
    simp only [decide_eq_true_eq]
    rw [sgn_mul_pos_iff hl hdz]
    constructor
    · intro hc
      obtain ⟨μ, hμ⟩ : ∃ μ, μ = (p.v - a.v) / (b.v - a.v) := ⟨_, rfl⟩
      have hμd : μ * (b.v - a.v) = p.v - a.v := by rw [hμ]; exact Rat.div_mul_cancel hdz
      have hfac : o2 p a b = (b.v - a.v) * (a.u + μ * (b.u - a.u) - p.u) := by
        rw [← hd0]
        have : (b.v - a.v) * (μ * (b.u - a.u)) = (b.u - a.u) * (p.v - a.v) := by rw [← hμd]; grind
        grind
      rw [hfac, mul_sq_pos_iff hdz] at hc
      refine ⟨μ, ?_, ?_, by grind, by grind⟩
      · rcases lt_trichotomy' 0 (b.v - a.v) with h | h | h
        · have := (mul_pos_iff' μ (b.v - a.v)).mp (by rw [hμd]; grind); grind
        · exact absurd h.symm hdz
        · have := (mul_neg_iff' μ (b.v - a.v)).mp (by rw [hμd]; grind); grind
      · have h1 : (1 - μ) * (b.v - a.v) = b.v - p.v := by grind
        rcases lt_trichotomy' 0 (b.v - a.v) with h | h | h
        · have := (mul_pos_iff' (1 - μ) (b.v - a.v)).mp (by rw [h1]; grind); grind
        · exact absurd h.symm hdz
        · have := (mul_neg_iff' (1 - μ) (b.v - a.v)).mp (by rw [h1]; grind); grind
    · rintro ⟨μ, h0, h1, hv, hu⟩
      have hfac : o2 p a b = (b.v - a.v) * (a.u + μ * (b.u - a.u) - p.u) := by
        rw [← hd0]
        have hμd : μ * (b.v - a.v) = p.v - a.v := by grind
        have : (b.v - a.v) * (μ * (b.u - a.u)) = (b.u - a.u) * (p.v - a.v) := by rw [← hμd]; grind
        grind
      rw [hfac, mul_sq_pos_iff hdz]; grind

/-! ## Triangles -/

/-- Strictly inside the triangle `a b c` (either orientation). -/
def inside2 (a b c p : P2) : Bool :=
  decide (0 < o2 a b c * o2 p b c) && decide (0 < o2 a b c * o2 a p c) &&
    decide (0 < o2 a b c * o2 a b p)

theorem bary_sum (a b c q : P2) : o2 q b c + o2 a q c + o2 a b q = o2 a b c := by
  unfold o2; grind

theorem bary_u (a b c q : P2) :
    o2 a b c * q.u = o2 q b c * a.u + o2 a q c * b.u + o2 a b q * c.u := by unfold o2; grind

theorem bary_v (a b c q : P2) :
    o2 a b c * q.v = o2 q b c * a.v + o2 a q c * b.v + o2 a b q * c.v := by unfold o2; grind

/-- The point `μ` of the way from `a` to `b`. -/
def lerp2 (a b : P2) (μ : Rat) : P2 := ⟨a.u + μ * (b.u - a.u), a.v + μ * (b.v - a.v)⟩

theorem o2_lerp_line (a b : P2) (μ : Rat) (x : P2) : o2 (lerp2 a b μ) a b = 0 := by
  unfold o2 lerp2; grind

theorem o2_lerp_bc (a b c : P2) (μ : Rat) : o2 a (lerp2 b c μ) c = (1 - μ) * o2 a b c := by
  unfold o2 lerp2; grind

theorem o2_lerp_bc' (a b c : P2) (μ : Rat) : o2 a b (lerp2 b c μ) = μ * o2 a b c := by
  unfold o2 lerp2; grind

theorem gc_symm (p a b : P2) : gc p a b ↔ gc p b a := by
  unfold gc
  constructor <;> rintro ⟨μ, h0, h1, hv, hu⟩ <;> exact ⟨1 - μ, by grind, by grind, by grind, by grind⟩

/-- `p` moved `λ` along the ray. -/
def shift (p : P2) (l : Rat) : P2 := ⟨p.u + l, p.v⟩

/-- The triangle's three sides along the ray from `p`, each as an affine function of the ray's
parameter, signed so that all are positive exactly inside. -/
def rayFns (a b c p : P2) : List Aff :=
  [⟨o2 a b c * o2 p b c, o2 a b c * (b.v - c.v)⟩, ⟨o2 a b c * o2 a p c, o2 a b c * (c.v - a.v)⟩,
   ⟨o2 a b c * o2 a b p, o2 a b c * (a.v - b.v)⟩]

theorem rayFns_ev (a b c p : P2) (l : Rat) :
    (rayFns a b c p).map (·.ev l) =
      [o2 a b c * o2 (shift p l) b c, o2 a b c * o2 a (shift p l) c, o2 a b c * o2 a b (shift p l)] := by
  simp only [rayFns, List.map_cons, List.map_nil, Aff.ev, o2, shift]
  refine List.cons_eq_cons.mpr ⟨by grind, List.cons_eq_cons.mpr ⟨by grind, List.cons_eq_cons.mpr ⟨by grind, rfl⟩⟩⟩

theorem rayFns_cyc (a b c p : P2) :
    rayFns b c a p = [(rayFns a b c p)[1]!, (rayFns a b c p)[2]!, (rayFns a b c p)[0]!] := by
  simp only [rayFns, List.getElem!_cons_zero, List.getElem!_cons_succ]
  have h1 : o2 b c a = o2 a b c := (o2_cyc a b c).symm
  rw [h1]
  have e1 : o2 p c a = o2 a p c := by unfold o2; grind
  have e2 : o2 b p a = o2 a b p := by unfold o2; grind
  have e3 : o2 b c p = o2 p b c := by unfold o2; grind
  rw [e1, e2, e3]

theorem pos_rayFns (a b c p : P2) (l : Rat) : pos (rayFns a b c p) l = inside2 a b c (shift p l) := by
  have := rayFns_ev a b c p l
  simp only [pos, inside2, rayFns, List.all_cons, List.all_nil, Bool.and_true] at this ⊢
  simp only [List.map_cons, List.map_nil, List.cons.injEq] at this
  rw [this.1, this.2.1, this.2.2.1]
  simp [Bool.and_assoc]

theorem shift_zero (p : P2) : shift p 0 = p := by cases p; simp only [shift, P2.mk.injEq]; grind

/-- The bound past which the ray has left the triangle and met every side's line. -/
def rayEnd (a b c p : P2) : Rat :=
  1 + (qabs (a.u - p.u) + qabs (b.u - p.u) + qabs (c.u - p.u)) +
    (rayFns a b c p).foldr (fun g acc => qabs g.root + acc) 0

theorem le_qabs' (a : Rat) : a ≤ qabs a := by unfold qabs; split <;> grind
theorem qabs_nonneg' (a : Rat) : 0 ≤ qabs a := by unfold qabs; split <;> grind
theorem neg_le_qabs' (a : Rat) : -a ≤ qabs a := by unfold qabs; split <;> grind

theorem rayEnd_gt (a b c p : P2) :
    let M := qabs (a.u - p.u) + qabs (b.u - p.u) + qabs (c.u - p.u)
    M < rayEnd a b c p ∧ ∀ g ∈ rayFns a b c p, g.root < rayEnd a b c p := by
  simp only [rayEnd, rayFns, List.foldr_cons, List.foldr_nil, List.mem_cons, List.mem_nil_iff, or_false]
  have := qabs_nonneg' (a.u - p.u); have := qabs_nonneg' (b.u - p.u); have := qabs_nonneg' (c.u - p.u)
  refine ⟨?_, ?_⟩
  · have := qabs_nonneg' (Aff.root ⟨o2 a b c * o2 p b c, o2 a b c * (b.v - c.v)⟩)
    have := qabs_nonneg' (Aff.root ⟨o2 a b c * o2 a p c, o2 a b c * (c.v - a.v)⟩)
    have := qabs_nonneg' (Aff.root ⟨o2 a b c * o2 a b p, o2 a b c * (a.v - b.v)⟩)
    grind
  · intro g hg
    have := qabs_nonneg' (Aff.root ⟨o2 a b c * o2 p b c, o2 a b c * (b.v - c.v)⟩)
    have := qabs_nonneg' (Aff.root ⟨o2 a b c * o2 a p c, o2 a b c * (c.v - a.v)⟩)
    have := qabs_nonneg' (Aff.root ⟨o2 a b c * o2 a b p, o2 a b c * (a.v - b.v)⟩)
    rcases hg with rfl | rfl | rfl
    · have := le_qabs' (Aff.root ⟨o2 a b c * o2 p b c, o2 a b c * (b.v - c.v)⟩); grind
    · have := le_qabs' (Aff.root ⟨o2 a b c * o2 a p c, o2 a b c * (c.v - a.v)⟩); grind
    · have := le_qabs' (Aff.root ⟨o2 a b c * o2 a b p, o2 a b c * (a.v - b.v)⟩); grind

theorem le_add_qabs (x p : Rat) : x ≤ p + qabs (x - p) := by have := le_qabs' (x - p); grind

/-- Inside the triangle, a point is no further along `u` than `p + M`. -/
theorem inside_u_le (a b c q : P2) (m : Rat) (hD : o2 a b c ≠ 0) (h : inside2 a b c q = true)
    (ha : a.u ≤ m) (hb : b.u ≤ m) (hc : c.u ≤ m) : q.u ≤ m := by
  simp only [inside2, Bool.and_eq_true, decide_eq_true_eq] at h
  obtain ⟨⟨h1, h2⟩, h3⟩ := h
  have e := bary_u a b c q
  have s := bary_sum a b c q
  have i1 := Rat.mul_le_mul_of_nonneg_left ha (Rat.le_of_lt h1)
  have i2 := Rat.mul_le_mul_of_nonneg_left hb (Rat.le_of_lt h2)
  have i3 := Rat.mul_le_mul_of_nonneg_left hc (Rat.le_of_lt h3)
  have hD2 := sq_pos' hD
  -- D (D q.u) ≤ D D m
  have : o2 a b c * o2 a b c * q.u ≤ o2 a b c * o2 a b c * m := by
    have e2 : o2 a b c * o2 a b c * q.u =
        o2 a b c * o2 q b c * a.u + o2 a b c * o2 a q c * b.u + o2 a b c * o2 a b q * c.u := by
      rw [Rat.mul_assoc, e]; grind
    have e3 : o2 a b c * o2 a b c * m =
        o2 a b c * o2 q b c * m + o2 a b c * o2 a q c * m + o2 a b c * o2 a b q * m := by
      rw [← s]; grind
    grind
  apply Classical.byContradiction; intro hn
  have := Rat.mul_lt_mul_of_pos_left (show m < q.u by grind) hD2
  grind

theorem nonpos3_first (g₁ g₂ g₃ : Aff) (r : Rat) (h : g₁.ev r = 0) :
    (nonpos [g₁, g₂, g₃] r == 1) = (decide (0 < g₂.ev r) && decide (0 < g₃.ev r)) := by
  simp only [nonpos, List.countP_cons, List.countP_nil, h]
  by_cases h2 : 0 < g₂.ev r <;> by_cases h3 : 0 < g₃.ev r <;>
    simp [h2, h3, show ¬ g₂.ev r ≤ 0 ↔ 0 < g₂.ev r from by grind,
      show ¬ g₃.ev r ≤ 0 ↔ 0 < g₃.ev r from by grind] <;> grind

theorem nonpos_perm {l l' : List Aff} (h : l.Perm l') (r : Rat) : nonpos l r = nonpos l' r := by
  unfold nonpos; exact h.countP_eq _

theorem isEv_perm {l l' : List Aff} (h : l.Perm l') (s₀ s₁ : Rat) (g : Aff) :
    isEv s₀ s₁ l g = isEv s₀ s₁ l' g := by
  unfold isEv; rw [nonpos_perm h]

/-- `x / D` positive when `D x` is. -/
theorem div_pos_of_mul {x D : Rat} (hD : D ≠ 0) (h : 0 < D * x) : 0 < x / D := by
  have e : x / D * (D * D) = D * x := by
    rw [show x / D * (D * D) = (x / D * D) * D by grind, Rat.div_mul_cancel hD]; grind
  have : 0 < x / D * (D * D) := by rw [e]; exact h
  exact (Rat.mul_pos_iff_of_pos_right (sq_pos' hD)).mp this

/-- On the line `b c` (`o2 q b c = 0`), `q` is `b + μ (c - b)` with `μ = o2 a b q / D`. -/
theorem on_bc (a b c q : P2) (hD : o2 a b c ≠ 0) (hq : o2 q b c = 0) :
    q.u = b.u + o2 a b q / o2 a b c * (c.u - b.u) ∧ q.v = b.v + o2 a b q / o2 a b c * (c.v - b.v) := by
  have hs := bary_sum a b c q
  have hv := bary_v a b c q
  have hu := bary_u a b c q
  rw [hq] at hs hv hu
  have hk : o2 a b q / o2 a b c * o2 a b c = o2 a b q := Rat.div_mul_cancel hD
  constructor
  · have : o2 a b c * q.u = o2 a b c * (b.u + o2 a b q / o2 a b c * (c.u - b.u)) := by
      rw [hu]
      have : o2 a b c * (o2 a b q / o2 a b c * (c.u - b.u)) = o2 a b q * (c.u - b.u) := by
        rw [← Rat.mul_assoc, Rat.mul_comm (o2 a b c), hk]
      rw [Rat.mul_add, this]
      have : o2 a q c = o2 a b c - o2 a b q := by grind
      rw [this]; grind
    have := this; grind
  · have : o2 a b c * q.v = o2 a b c * (b.v + o2 a b q / o2 a b c * (c.v - b.v)) := by
      rw [hv]
      have : o2 a b c * (o2 a b q / o2 a b c * (c.v - b.v)) = o2 a b q * (c.v - b.v) := by
        rw [← Rat.mul_assoc, Rat.mul_comm (o2 a b c), hk]
      rw [Rat.mul_add, this]
      have : o2 a q c = o2 a b c - o2 a b q := by grind
      rw [this]; grind
    grind

theorem o2_lerp_bc_self (b c : P2) (μ : Rat) : o2 (lerp2 b c μ) b c = 0 := by
  unfold o2 lerp2; grind

/-- The first side's root is an event iff the ray meets the open edge `b c`. -/
theorem ev_edge (a b c p : P2) (hD : o2 a b c ≠ 0) (hc : p.v ≠ c.v) (hb : p.v ≠ b.v) :
    isEv 0 (rayEnd a b c p) (rayFns a b c p) (rayFns a b c p)[0]! = decide (gc p b c) := by
  obtain ⟨hM, hroots⟩ := rayEnd_gt a b c p
  obtain ⟨g, hg⟩ : ∃ g : Aff, g = ⟨o2 a b c * o2 p b c, o2 a b c * (b.v - c.v)⟩ := ⟨_, rfl⟩
  have hfns : rayFns a b c p = [g, ⟨o2 a b c * o2 a p c, o2 a b c * (c.v - a.v)⟩,
      ⟨o2 a b c * o2 a b p, o2 a b c * (a.v - b.v)⟩] := by rw [hg]; rfl
  have hgr : g.root < rayEnd a b c p := hroots g (by rw [hfns]; simp)
  rw [hfns, List.getElem!_cons_zero, ← hfns]
  unfold isEv
  by_cases hm : g.m = 0
  · -- the edge runs along the ray's direction: never met
    simp only [hm, ne_eq, not_true_eq_false, decide_false, Bool.false_and]
    apply Eq.symm; simp only [decide_eq_false_iff_not]
    rintro ⟨μ, h0, h1, hv, _⟩
    have : b.v = c.v := by
      have : o2 a b c * (b.v - c.v) = 0 := by rw [hg] at hm; exact hm
      rcases Rat.mul_eq_zero.mp this with h | h
      · exact absurd h hD
      · grind
    rcases between_of_combo h0 h1 hv with h | h | h <;> grind
  · have hev := rayFns_ev a b c p g.root
    rw [hfns] at hev
    simp only [List.map_cons, List.map_nil, List.cons.injEq] at hev
    obtain ⟨e1, e2, e3, -⟩ := hev
    rw [hfns, nonpos3_first _ _ _ _ (Aff.ev_root g hm), e2, e3]
    have e0 : o2 a b c * o2 (shift p g.root) b c = 0 := by rw [← e1]; exact Aff.ev_root g hm
    have hq0 : o2 (shift p g.root) b c = 0 := by
      rcases Rat.mul_eq_zero.mp e0 with h | h
      · exact absurd h hD
      · exact h
    apply Bool.eq_iff_iff.mpr
    simp only [Bool.and_eq_true, decide_eq_true_eq, ne_eq, hm, not_false_eq_true, true_and]
    constructor
    · rintro ⟨⟨hr0, -⟩, hb0, hc0⟩
      obtain ⟨hqu, hqv⟩ := on_bc a b c _ hD hq0
      refine ⟨o2 a b (shift p g.root) / o2 a b c, div_pos_of_mul hD hc0, ?_, ?_, ?_⟩
      · have h1 : 1 - o2 a b (shift p g.root) / o2 a b c = o2 a (shift p g.root) c / o2 a b c := by
          have hs := bary_sum a b c (shift p g.root)
          rw [hq0] at hs
          have k1 : o2 a b (shift p g.root) / o2 a b c * o2 a b c = o2 a b (shift p g.root) := Rat.div_mul_cancel hD
          have k2 : o2 a (shift p g.root) c / o2 a b c * o2 a b c = o2 a (shift p g.root) c := Rat.div_mul_cancel hD
          have : (1 - o2 a b (shift p g.root) / o2 a b c) * o2 a b c = o2 a (shift p g.root) c / o2 a b c * o2 a b c := by
            rw [k2]; grind
          grind
        have := div_pos_of_mul hD hb0; grind
      · have : (shift p g.root).v = p.v := rfl
        rw [this] at hqv; exact hqv.symm
      · have : (shift p g.root).u = p.u + g.root := rfl
        rw [this] at hqu; grind
    · rintro ⟨μ, h0, h1, hv, hu⟩
      obtain ⟨q, hqdef⟩ : ∃ q, q = lerp2 b c μ := ⟨_, rfl⟩
      have hsq : shift p (q.u - p.u) = q := by
        rw [hqdef]; simp only [shift, lerp2, P2.mk.injEq]; grind
      have hroot : g.root = q.u - p.u := by
        have hz : g.ev (q.u - p.u) = 0 := by
          have := rayFns_ev a b c p (q.u - p.u)
          rw [hfns] at this
          simp only [List.map_cons, List.map_nil, List.cons.injEq] at this
          rw [this.1, hsq, hqdef, o2_lerp_bc_self]; grind
        rw [Aff.ev_eq g hm] at hz
        rcases Rat.mul_eq_zero.mp hz with h | h
        · exact absurd h hm
        · grind
      rw [hroot, hsq]
      have hqu : q.u = b.u + μ * (c.u - b.u) := by rw [hqdef]; rfl
      refine ⟨⟨by grind, ?_⟩, ?_, ?_⟩
      · -- the crossing is no further than the triangle
        have := le_add_qabs b.u p.u; have := le_add_qabs c.u p.u
        have := qabs_nonneg' (a.u - p.u); have := qabs_nonneg' (b.u - p.u)
        have := qabs_nonneg' (c.u - p.u)
        have := convex_le (x := b.u) (y := c.u)
          (m := p.u + (qabs (a.u - p.u) + qabs (b.u - p.u) + qabs (c.u - p.u)))
          (μ := μ) (by grind) (by grind) (by grind) (by grind)
        grind
      · rw [hqdef, o2_lerp_bc]
        have := Rat.mul_pos (sq_pos' hD) (show 0 < 1 - μ by grind); grind
      · rw [hqdef, o2_lerp_bc']
        have := Rat.mul_pos (sq_pos' hD) h0; grind

theorem rayEnd_cyc (a b c p : P2) : rayEnd b c a p = rayEnd a b c p := by
  unfold rayEnd
  rw [rayFns_cyc]
  simp only [rayFns, List.getElem!_cons_zero, List.getElem!_cons_succ, List.foldr_cons, List.foldr_nil]
  grind

/-- Two sides vanish together only at a vertex. -/
theorem two_sides_vertex (a b c q : P2) (hD : o2 a b c ≠ 0) :
    (o2 q b c = 0 → o2 a q c = 0 → q.v = c.v) ∧ (o2 a q c = 0 → o2 a b q = 0 → q.v = a.v) ∧
      (o2 q b c = 0 → o2 a b q = 0 → q.v = b.v) := by
  have hs := bary_sum a b c q
  have hv := bary_v a b c q
  refine ⟨fun h1 h2 => ?_, fun h1 h2 => ?_, fun h1 h2 => ?_⟩
  · rw [h1, h2] at hs hv
    have : o2 a b c * q.v = o2 a b c * c.v := by rw [hv, ← hs]; grind
    grind
  · rw [h1, h2] at hs hv
    have : o2 a b c * q.v = o2 a b c * a.v := by rw [hv, ← hs]; grind
    grind
  · rw [h1, h2] at hs hv
    have : o2 a b c * q.v = o2 a b c * b.v := by rw [hv, ← hs]; grind
    grind

theorem ev_ne_of_gt (g : Aff) (s : Rat) (h0 : g.ev 0 ≠ 0) (hs : g.root < s) : g.ev s ≠ 0 := by
  by_cases hm : g.m = 0
  · unfold Aff.ev at *; rw [hm] at *; grind
  · rw [Aff.ev_eq g hm]; exact mul_ne_zero' hm (by grind)

/-- **A triangle's interior by ray parity**: when the ray's line misses the vertices and `p` is off
the sides' lines, `p` is inside iff the ray meets an odd number of the open edges. -/
theorem tri_ray (a b c p : P2) (hD : o2 a b c ≠ 0) (ha : p.v ≠ a.v) (hb : p.v ≠ b.v)
    (hc : p.v ≠ c.v) (h1 : o2 p b c ≠ 0) (h2 : o2 a p c ≠ 0) (h3 : o2 a b p ≠ 0) :
    inside2 a b c p = par [decide (gc p b c), decide (gc p c a), decide (gc p a b)] := by
  obtain ⟨hM, hroots⟩ := rayEnd_gt a b c p
  have hM0 : 0 ≤ qabs (a.u - p.u) + qabs (b.u - p.u) + qabs (c.u - p.u) := by
    have := qabs_nonneg' (a.u - p.u); have := qabs_nonneg' (b.u - p.u)
    have := qabs_nonneg' (c.u - p.u); grind
  have hΛ : 0 < rayEnd a b c p := by grind
  have hev0 := rayFns_ev a b c p 0
  rw [shift_zero] at hev0
  have hends : ∀ g ∈ rayFns a b c p, g.ev 0 ≠ 0 ∧ g.ev (rayEnd a b c p) ≠ 0 := by
    have hz : ∀ g ∈ rayFns a b c p, g.ev 0 ≠ 0 := by
      intro g hg
      simp only [rayFns, List.mem_cons, List.mem_nil_iff, or_false] at hg
      rcases hg with rfl | rfl | rfl <;> simp only [Aff.ev]
      · have := mul_ne_zero' hD h1; grind
      · have := mul_ne_zero' hD h2; grind
      · have := mul_ne_zero' hD h3; grind
    exact fun g hg => ⟨hz g hg, ev_ne_of_gt g _ (hz g hg) (hroots g hg)⟩
  have hdist : Distinct (rayFns a b c p) 0 (rayEnd a b c p) := by
    intro s _ _
    obtain ⟨t1, t2, t3⟩ := two_sides_vertex a b c (shift p s) hD
    have := rayFns_ev a b c p s
    simp only [rayFns, List.map_cons, List.map_nil, List.cons.injEq] at this
    obtain ⟨e1, e2, e3, -⟩ := this
    simp only [rayFns, List.countP_cons, List.countP_nil, e1, e2, e3]
    have hsv : (shift p s).v = p.v := rfl
    have z1 : o2 a b c * o2 (shift p s) b c = 0 ↔ o2 (shift p s) b c = 0 := by
      constructor
      · intro h; rcases Rat.mul_eq_zero.mp h with h | h; exact absurd h hD; exact h
      · intro h; rw [h]; grind
    have z2 : o2 a b c * o2 a (shift p s) c = 0 ↔ o2 a (shift p s) c = 0 := by
      constructor
      · intro h; rcases Rat.mul_eq_zero.mp h with h | h; exact absurd h hD; exact h
      · intro h; rw [h]; grind
    have z3 : o2 a b c * o2 a b (shift p s) = 0 ↔ o2 a b (shift p s) = 0 := by
      constructor
      · intro h; rcases Rat.mul_eq_zero.mp h with h | h; exact absurd h hD; exact h
      · intro h; rw [h]; grind
    by_cases k1 : o2 (shift p s) b c = 0 <;> by_cases k2 : o2 a (shift p s) c = 0 <;>
      by_cases k3 : o2 a b (shift p s) = 0 <;> simp [z1, z2, z3, k1, k2, k3]
    · exact hc (hsv ▸ t1 k1 k2)
    · exact hc (hsv ▸ t1 k1 k2)
    · exact hb (hsv ▸ t3 k1 k3)
    · exact ha (hsv ▸ t2 k2 k3)
  have hip := ip (rayFns a b c p) 0 (rayEnd a b c p) hΛ hends hdist
  rw [pos_rayFns, pos_rayFns, shift_zero] at hip
  have hfar : inside2 a b c (shift p (rayEnd a b c p)) = false := by
    apply Classical.byContradiction; intro hn
    have hin : inside2 a b c (shift p (rayEnd a b c p)) = true := by simpa using hn
    have := inside_u_le a b c _ (p.u + (qabs (a.u - p.u) + qabs (b.u - p.u) + qabs (c.u - p.u))) hD hin
      (by have := le_add_qabs a.u p.u; have := qabs_nonneg' (b.u - p.u); have := qabs_nonneg' (c.u - p.u); grind)
      (by have := le_add_qabs b.u p.u; have := qabs_nonneg' (a.u - p.u); have := qabs_nonneg' (c.u - p.u); grind)
      (by have := le_add_qabs c.u p.u; have := qabs_nonneg' (a.u - p.u); have := qabs_nonneg' (b.u - p.u); grind)
    simp only [shift] at this; grind
  rw [hfar, Bool.xor_false] at hip
  rw [hip]
  -- the three events
  obtain ⟨g0, g1, g2, hl⟩ : ∃ g0 g1 g2, rayFns a b c p = [g0, g1, g2] := ⟨_, _, _, rfl⟩
  have ev0 := ev_edge a b c p hD hc hb
  have ev1 := ev_edge b c a p (by rw [← o2_cyc]; exact hD) ha hc
  have ev2 := ev_edge c a b p (by rw [o2_cyc]; exact hD) hb ha
  rw [rayFns_cyc, rayEnd_cyc] at ev1
  rw [rayFns_cyc, rayEnd_cyc, rayFns_cyc, rayEnd_cyc] at ev2
  rw [hl] at ev0 ev1 ev2
  simp only [List.getElem!_cons_zero, List.getElem!_cons_succ] at ev0 ev1 ev2
  have p1 : [g1, g2, g0].Perm [g0, g1, g2] :=
    ((List.Perm.swap g1 g0 [g2]).trans (List.Perm.cons g1 (List.Perm.swap g2 g0 []))).symm
  have p2 : [g2, g0, g1].Perm [g0, g1, g2] :=
    ((List.Perm.cons g0 (List.Perm.swap g2 g1 [])).trans (List.Perm.swap g2 g0 [g1])).symm
  rw [isEv_perm p1] at ev1
  rw [isEv_perm p2] at ev2
  rw [hl]
  simp only [List.map_cons, List.map_nil]
  rw [ev0, ev1, ev2]

end ForgeCertify
