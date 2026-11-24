/-!
# The signed winding number of a rational polygon: definitions

A polygon is a list of vertices `vs`, closed: its sides are `(v₀,v₁), …, (vₙ₋₁,v₀)` (`sides`).
Its winding number about `q` (`wind`) is the sum over the sides of the signed **half-open**
crossing (`cross`) of the ray from `q` along `+x`: a side `a → b` counts `+1` if `b` is above
`q`'s height and `a` is not (`q.y < b.y`, `¬ q.y < a.y`) and `q` is strictly left of it
(`0 < orient a b q`), `-1` going down with `q` strictly right of it, else `0`. A vertex at `q`'s
height is classed as below: that is the whole of the rule for degenerate points, and no
general-position hypothesis is used anywhere.

Ported from verify/forge-exact/ForgeExact/Jordan.lean (branch claude/repo-review-mbdrlj-dol-jordan,
8a2734c8): the sign lemmas (`ne_cases`, `mul_pos_iff_left`, …), the role of the half-open rule as
the reference's `decide (a.z > q.z)` tie-break, and the shape of the argument (`seg_identity` and
`telescope`: per edge, the change of the crossings is a difference of a vertex function, which
cancels round a closed loop), here signed and in the plane.
-/

namespace Winding2d

structure Pt where
  x : Rat
  y : Rat
  deriving DecidableEq, Repr, Inhabited

@[ext] theorem Pt.ext' {p q : Pt} (hx : p.x = q.x) (hy : p.y = q.y) : p = q := by
  cases p; cases q; simp_all

/-- Twice the signed area of `a b c`: positive when `c` is left of `a → b`. -/
def orient (a b c : Pt) : Rat := (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)

/-- The half-open class of `v` against `q`: strictly above `q`'s height. -/
def above (q v : Pt) : Bool := decide (q.y < v.y)

/-- **The signed half-open crossing** of the ray from `q` along `+x` with the side `a → b`. -/
def cross (q a b : Pt) : Int :=
  if q.y < b.y ∧ ¬q.y < a.y then (if 0 < orient a b q then 1 else 0)
  else if q.y < a.y ∧ ¬q.y < b.y then (if orient a b q < 0 then -1 else 0)
  else 0

/-- Consecutive pairs of a list. -/
def chain : List Pt → List (Pt × Pt)
  | a :: b :: t => (a, b) :: chain (b :: t)
  | _ => []

/-- A polygon's sides, the closing one last. -/
def sides : List Pt → List (Pt × Pt)
  | [] => []
  | v :: rest => chain (v :: rest ++ [v])

/-- **The winding number** of the polygon `vs` about `q`. -/
def wind (vs : List Pt) (q : Pt) : Int := ((sides vs).map fun e => cross q e.1 e.2).sum

/-! ## Points on segments -/

/-- The point at `t` along `a → b`. -/
def lerp (a b : Pt) (t : Rat) : Pt := ⟨a.x + t * (b.x - a.x), a.y + t * (b.y - a.y)⟩

/-- `z` lies on the closed segment `ab`. -/
def OnSeg (z a b : Pt) : Prop := ∃ t, 0 ≤ t ∧ t ≤ 1 ∧ z = lerp a b t

/-- The closed segments `pp'` and `ab` meet. -/
def Meets (p p' a b : Pt) : Prop := ∃ z, OnSeg z p p' ∧ OnSeg z a b

/-- `q` lies on no side of `vs`. -/
def OffPoly (vs : List Pt) (q : Pt) : Prop := ∀ e ∈ sides vs, ¬OnSeg q e.1 e.2

/-! ## Signs (after Jordan.lean) -/

theorem ne_cases {a : Rat} (h : a ≠ 0) : a < 0 ∨ 0 < a := by
  by_cases h' : a < 0
  · exact Or.inl h'
  · exact Or.inr (by grind)

theorem pos_mul_iff {k x : Rat} (hk : 0 < k) : 0 < x * k ↔ 0 < x := by
  constructor
  · intro h; apply Classical.byContradiction; intro hx
    have := Rat.mul_nonneg (by grind : (0:Rat) ≤ -x) (Rat.le_of_lt hk)
    grind
  · intro h; exact Rat.mul_pos h hk

theorem nonneg_mul_iff {k x : Rat} (hk : 0 < k) : 0 ≤ x * k ↔ 0 ≤ x := by
  constructor
  · intro h; apply Classical.byContradiction; intro hx
    have := Rat.mul_pos (by grind : (0:Rat) < -x) hk
    grind
  · intro h; exact Rat.mul_nonneg h (Rat.le_of_lt hk)

theorem pos_mul_iff_neg {k x : Rat} (hk : k < 0) : 0 < x * k ↔ x < 0 := by
  have := pos_mul_iff (k := -k) (x := -x) (by grind)
  have e : -x * -k = x * k := by grind
  rw [e] at this; grind

theorem nonneg_mul_iff_neg {k x : Rat} (hk : k < 0) : 0 ≤ x * k ↔ x ≤ 0 := by
  have := nonneg_mul_iff (k := -k) (x := -x) (by grind)
  have e : -x * -k = x * k := by grind
  rw [e] at this; grind

theorem neg_mul_iff {k x : Rat} (hk : 0 < k) : x * k < 0 ↔ x < 0 := by
  have := pos_mul_iff (k := k) (x := -x) hk
  have e : -x * k = -(x * k) := by grind
  rw [e] at this; grind

theorem mul_eq_zero_left {k x : Rat} (hk : k ≠ 0) (h : x * k = 0) : x = 0 := by
  rcases Rat.mul_eq_zero.mp h with h | h
  · exact h
  · exact absurd h hk

/-- A quotient that lies in `[0,1]`, from the signs of its numerator and of the rest. -/
theorem unit_of (n d t : Rat) (hd : d ≠ 0) (ht : t * d = n)
    (h : (0 ≤ n ∧ 0 ≤ d - n) ∨ (n ≤ 0 ∧ d - n ≤ 0)) : 0 ≤ t ∧ t ≤ 1 := by
  have e : (1 - t) * d = d - n := by grind
  rcases ne_cases hd with hd | hd
  · have h1 := nonneg_mul_iff_neg hd (x := t)
    have h2 := nonneg_mul_iff_neg hd (x := 1 - t)
    rw [ht] at h1; rw [e] at h2; grind
  · have h1 := nonneg_mul_iff hd (x := t)
    have h2 := nonneg_mul_iff hd (x := 1 - t)
    rw [ht] at h1; rw [e] at h2; grind

/-- An affine function changing sign class on `[0,1]` has a root there. -/
theorem exists_root (o1 o2 : Rat) (h : ¬(0 < o1 ↔ 0 < o2)) :
    ∃ t, 0 ≤ t ∧ t ≤ 1 ∧ (1 - t) * o1 + t * o2 = 0 := by
  have hd : o1 - o2 ≠ 0 := by grind
  have ht : o1 / (o1 - o2) * (o1 - o2) = o1 := Rat.div_mul_cancel hd
  refine ⟨o1 / (o1 - o2), ?_⟩
  have := unit_of o1 (o1 - o2) _ hd ht (by grind)
  exact ⟨this.1, this.2, by grind⟩

/-- No root on `[0,1]`: the two ends are in the same sign class. -/
theorem same_sign (o1 o2 : Rat) (h : ∀ t, 0 ≤ t → t ≤ 1 → (1 - t) * o1 + t * o2 ≠ 0) :
    (0 < o1 ↔ 0 < o2) ∧ (o1 < 0 ↔ o2 < 0) := by
  constructor
  · apply Classical.byContradiction; intro hc
    obtain ⟨t, h0, h1, he⟩ := exists_root o1 o2 hc
    exact h t h0 h1 he
  · apply Classical.byContradiction; intro hc
    obtain ⟨t, h0, h1, he⟩ := exists_root (-o1) (-o2) (by grind)
    exact h t h0 h1 (by grind)

theorem convex_ge {lo u v t : Rat} (hu : lo ≤ u) (hv : lo ≤ v) (h0 : 0 ≤ t) (h1 : t ≤ 1) :
    lo ≤ u + t * (v - u) := by
  have := Rat.mul_nonneg (by grind : (0:Rat) ≤ 1 - t) (by grind : (0:Rat) ≤ u - lo)
  have := Rat.mul_nonneg h0 (by grind : (0:Rat) ≤ v - lo)
  grind

theorem convex_le {hi u v t : Rat} (hu : u ≤ hi) (hv : v ≤ hi) (h0 : 0 ≤ t) (h1 : t ≤ 1) :
    u + t * (v - u) ≤ hi := by
  have := convex_ge (lo := -hi) (u := -u) (v := -v) (by grind) (by grind) h0 h1
  grind

/-! ## Segments -/

theorem orient_lerp (a b p p' : Pt) (t : Rat) :
    orient a b (lerp p p' t) = (1 - t) * orient a b p + t * orient a b p' := by
  simp only [orient, lerp]; grind

theorem orient_swap (a b c : Pt) : orient b a c = -orient a b c := by
  simp only [orient]; grind

theorem orient_cycle (a b c : Pt) : orient b c a = orient a b c := by
  simp only [orient]; grind

theorem onSeg_symm {z a b : Pt} (h : OnSeg z a b) : OnSeg z b a := by
  obtain ⟨t, h0, h1, rfl⟩ := h
  exact ⟨1 - t, by grind, by grind, by ext <;> simp only [lerp] <;> grind⟩

theorem meets_symm {p p' a b : Pt} (h : Meets p p' a b) : Meets a b p p' := by
  obtain ⟨z, h1, h2⟩ := h; exact ⟨z, h2, h1⟩

theorem meets_flip {p p' a b : Pt} (h : Meets p p' a b) : Meets p' p a b := by
  obtain ⟨z, h1, h2⟩ := h; exact ⟨z, onSeg_symm h1, h2⟩

theorem meets_flip' {p p' a b : Pt} (h : Meets p p' a b) : Meets p p' b a := by
  obtain ⟨z, h1, h2⟩ := h; exact ⟨z, h1, onSeg_symm h2⟩

theorem onSeg_left {a b : Pt} : OnSeg a a b := ⟨0, by grind, by grind, by ext <;> simp [lerp] <;> grind⟩
theorem onSeg_right {a b : Pt} : OnSeg b a b := ⟨1, by grind, by grind, by ext <;> simp [lerp] <;> grind⟩

/-- A point of a sub-segment `[a, lerp a b s]` lies on `ab`. -/
theorem onSeg_sub_left {a b z : Pt} {s : Rat} (hs0 : 0 ≤ s) (hs1 : s ≤ 1)
    (h : OnSeg z a (lerp a b s)) : OnSeg z a b := by
  obtain ⟨t, h0, h1, rfl⟩ := h
  refine ⟨t * s, Rat.mul_nonneg h0 hs0, ?_, by ext <;> simp only [lerp] <;> grind⟩
  have := Rat.mul_nonneg (by grind : (0:Rat) ≤ 1 - t) hs0
  have := Rat.mul_nonneg h0 (by grind : (0:Rat) ≤ 1 - s)
  grind

/-- A point of a sub-segment `[lerp a b s, b]` lies on `ab`. -/
theorem onSeg_sub_right {a b z : Pt} {s : Rat} (hs0 : 0 ≤ s) (hs1 : s ≤ 1)
    (h : OnSeg z (lerp a b s) b) : OnSeg z a b := by
  obtain ⟨t, h0, h1, rfl⟩ := h
  refine ⟨s + t * (1 - s), ?_, ?_, by ext <;> simp only [lerp] <;> grind⟩
  · have := Rat.mul_nonneg h0 (by grind : (0:Rat) ≤ 1 - s); grind
  · have := Rat.mul_nonneg (by grind : (0:Rat) ≤ 1 - t) (by grind : (0:Rat) ≤ 1 - s); grind

def Btw (u a b : Rat) : Prop := (a ≤ u ∧ u ≤ b) ∨ (b ≤ u ∧ u ≤ a)

/-- A point on the line `ab`, at a height between `a`'s and `b`'s (`a.y ≠ b.y`), is on `ab`. -/
theorem onSeg_of_orient {z a b : Pt} (ho : orient a b z = 0) (hab : a.y ≠ b.y)
    (hz : Btw z.y a.y b.y) : OnSeg z a b := by
  have hd : b.y - a.y ≠ 0 := by grind
  have ht : (z.y - a.y) / (b.y - a.y) * (b.y - a.y) = z.y - a.y := Rat.div_mul_cancel hd
  have hu := unit_of _ _ _ hd ht (by unfold Btw at hz; grind)
  refine ⟨_, hu.1, hu.2, ?_⟩
  have hx : (a.x + (z.y - a.y) / (b.y - a.y) * (b.x - a.x) - z.x) * (b.y - a.y) = 0 := by
    simp only [orient] at ho; grind
  have := mul_eq_zero_left hd hx
  ext <;> simp only [lerp] <;> grind

/-- **No sign change without a meeting.** If `pp'` misses `ab` and both of `p`, `p'` are at
heights within `ab`'s (`a.y ≠ b.y`), they are on the same side of the line `ab`. -/
theorem sign_same {p p' a b : Pt} (hm : ¬Meets p p' a b) (hab : a.y ≠ b.y)
    (h1 : Btw p.y a.y b.y) (h2 : Btw p'.y a.y b.y) :
    (0 < orient a b p ↔ 0 < orient a b p') ∧ (orient a b p < 0 ↔ orient a b p' < 0) := by
  apply same_sign
  intro t h0 h1' he
  apply hm
  refine ⟨lerp p p' t, ⟨t, h0, h1', rfl⟩, onSeg_of_orient (by rw [orient_lerp]; exact he) hab ?_⟩
  simp only [lerp]
  unfold Btw at h1 h2 ⊢
  rcases h1 with h1 | h1 <;> rcases h2 with h2 | h2
  · exact Or.inl ⟨convex_ge h1.1 h2.1 h0 h1', convex_le h1.2 h2.2 h0 h1'⟩
  · have : a.y = b.y := by grind
    exact absurd this hab
  · have : a.y = b.y := by grind
    exact absurd this hab
  · exact Or.inr ⟨convex_ge h1.1 h2.1 h0 h1', convex_le h1.2 h2.2 h0 h1'⟩

/-- Cutting `ab` at a height `h` with `a.y ≤ h < b.y`: the point `c` there, the two pieces, and
`orient a b q` for `q` at that height in terms of `c`. -/
theorem cut (a b : Pt) (h : Rat) (hlo : a.y ≤ h) (hhi : h < b.y) :
    ∃ c : Pt, c.y = h ∧ (∀ z, OnSeg z a c → OnSeg z a b) ∧ (∀ z, OnSeg z c b → OnSeg z a b) ∧
      ∀ q : Pt, q.y = h → orient a b q = (c.x - q.x) * (b.y - a.y) := by
  have hd : b.y - a.y ≠ 0 := by grind
  have ht : (h - a.y) / (b.y - a.y) * (b.y - a.y) = h - a.y := Rat.div_mul_cancel hd
  have hu := unit_of _ _ _ hd ht (by grind)
  refine ⟨lerp a b ((h - a.y) / (b.y - a.y)), by simp only [lerp]; grind,
    fun z hz => onSeg_sub_left hu.1 hu.2 hz, fun z hz => onSeg_sub_right hu.1 hu.2 hz, ?_⟩
  intro q hq
  simp only [orient, lerp]
  have := congrArg (· * (b.x - a.x)) ht
  rw [hq]; grind

end Winding2d
