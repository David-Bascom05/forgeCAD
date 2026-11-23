import ForgeExact.Ref.Geom
import ForgeExact.Flat
import ForgeExact.RefFacts

/-!
# Jordan–Brouwer for the reference's ray parity

RefFacts' one assumption, proven: a closed flat-faced B-rep's ray-parity membership
(`Ref.Solid.contains`) is the same at any two points strictly on the same side of every face
plane, so constant on every full cell of an arrangement holding its planes (`contains_const`).

The argument (no topology, no enumeration of configurations):

1. **Perturbation** (`ref_eq_exact`): the reference's simulation of simplicity (the ray's origin
   moved by `(0, ε, ε²)`) is the exact count at `q + (0, δ, δ²)` for every small enough rational
   `δ`, and there every crossing is in general position.
2. **One move** (`move`): for two points `w`, `w'` in general position, both strictly on one side
   of every face plane, the parities agree. Per face, the faces' projected loops are crossed by
   the two rays and by the segment `ww'`; per edge (`seg_identity`) the three crossings have the
   parity of the edge's ends lying in the half-strip between the rays, which cancels round each
   closed loop; and per face the segment crossings that count are exactly those of edges of faces
   ahead of the segment (`face_step`). Summed over the faces, every edge is counted an even number
   of times (the B-rep is closed), so the change is even.
3. **The sweep** (`Aff.sweep3`): the one local fact, along one edge: crossing the boundary of the
   region where three affine functions are positive, at points where the other two are, has the
   parity of the region holding at the two ends. Proven by cases on the three functions' kinds
   (each positive, negative, rising or falling across [0,1]), which hold for every geometry.

Hypotheses on the solid, both checked by the kernel's `check`: every face's loops lie in its plane
(`FacesFlat`), and every edge is used an even number of times, in either direction (`Closed`).
-/

namespace ForgeExact.Jordan

open ForgeExact.Ref ForgeExact.Flat

theorem ne_cases {a : Rat} (h : a ≠ 0) : a < 0 ∨ 0 < a := by
  by_cases h' : a < 0
  · exact Or.inl h'
  · exact Or.inr (by grind)

theorem mul_pos_iff_left {k x : Rat} (hk : 0 < k) : 0 < k * x ↔ 0 < x := by
  constructor
  · intro h; apply Classical.byContradiction; intro hx
    have : k * x ≤ 0 := by
      have h1 : 0 ≤ -x := by grind
      have := Rat.mul_nonneg (Rat.le_of_lt hk) h1
      rw [Rat.mul_neg] at this; grind
    grind
  · intro h; exact Rat.mul_pos hk h

theorem mul_pos_iff_neg {k x : Rat} (hk : k < 0) : 0 < k * x ↔ x < 0 := by
  have hk' : 0 < -k := by grind
  have e : k * x = (-k) * (-x) := by rw [Rat.neg_mul, Rat.mul_neg]; grind
  rw [e, mul_pos_iff_left hk']; grind

theorem mul_neg_iff (a b : Rat) (ha : a ≠ 0) (hb : b ≠ 0) :
    a * b < 0 ↔ (a < 0 ∧ 0 < b) ∨ (0 < a ∧ b < 0) := by
  rcases ne_cases ha with ha | ha <;> rcases ne_cases hb with hb | hb
  · have := (mul_pos_iff_neg ha).mpr hb; grind
  · have := (mul_pos_iff_neg ha (x := -b)).mpr (by grind); rw [Rat.mul_neg] at this; grind
  · have := (mul_pos_iff_left ha (x := -b)).mpr (by grind); rw [Rat.mul_neg] at this; grind
  · have := (mul_pos_iff_left ha).mpr hb; grind

structure Aff where
  α : Rat
  γ : Rat

namespace Aff
def at_ (f : Aff) (t : Rat) : Rat := f.α + (f.γ - f.α) * t
def root (f : Aff) : Rat := f.α / (f.α - f.γ)

theorem at_eq (f : Aff) (h : f.α ≠ f.γ) (t : Rat) : f.at_ t = (f.γ - f.α) * (t - f.root) := by
  unfold at_ root
  have hd : f.α - f.γ ≠ 0 := by grind
  have := Rat.div_mul_cancel (a := f.α) hd
  grind

theorem at_pos_inc (f : Aff) (h1 : f.α < 0) (h2 : 0 < f.γ) (t : Rat) : 0 < f.at_ t ↔ f.root < t := by
  rw [f.at_eq (by grind), mul_pos_iff_left (by grind)]; grind

theorem at_pos_dec (f : Aff) (h1 : 0 < f.α) (h2 : f.γ < 0) (t : Rat) : 0 < f.at_ t ↔ t < f.root := by
  rw [f.at_eq (by grind), mul_pos_iff_neg (by grind)]; grind

theorem root_inc (f : Aff) (h1 : f.α < 0) (h2 : 0 < f.γ) : 0 < f.root ∧ f.root < 1 := by
  have e0 := f.at_eq (by grind) 0
  have e1 := f.at_eq (by grind) 1
  simp only [at_] at e0 e1
  have hd : 0 < f.γ - f.α := by grind
  exact ⟨(mul_pos_iff_left hd).mp (by grind), by
    have := (mul_pos_iff_left hd (x := 1 - f.root)).mp (by grind); grind⟩

theorem root_dec (f : Aff) (h1 : 0 < f.α) (h2 : f.γ < 0) : 0 < f.root ∧ f.root < 1 := by
  have e0 := f.at_eq (by grind) 0
  have e1 := f.at_eq (by grind) 1
  simp only [at_] at e0 e1
  have hd : f.γ - f.α < 0 := by grind
  exact ⟨by have := (mul_pos_iff_neg hd (x := 0 - f.root)).mp (by grind); grind, by
    have := (mul_pos_iff_neg hd (x := f.root - 1)).mp (by grind); grind⟩

theorem at_pos_pp (f : Aff) (h1 : 0 < f.α) (h2 : 0 < f.γ) {t : Rat} (ht0 : 0 ≤ t) (ht1 : t ≤ 1) :
    0 < f.at_ t := by
  have e : f.at_ t = (1 - t) * f.α + t * f.γ := by unfold at_; grind
  rw [e]
  have := Rat.mul_nonneg (by grind : (0:Rat) ≤ 1 - t) (Rat.le_of_lt h1)
  have := Rat.mul_nonneg ht0 (Rat.le_of_lt h2)
  by_cases h : t = 0
  · subst h; grind
  · have := Rat.mul_pos (by grind : (0:Rat) < t) h2; grind

theorem at_neg_nn (f : Aff) (h1 : f.α < 0) (h2 : f.γ < 0) {t : Rat} (ht0 : 0 ≤ t) (ht1 : t ≤ 1) :
    ¬0 < f.at_ t := by
  have := at_pos_pp ⟨-f.α, -f.γ⟩ (by grind) (by grind) ht0 ht1
  simp only [at_] at this ⊢; grind

/-- On [0,1], positivity by kind. -/
theorem at_pos_iff (f : Aff) (ha : f.α ≠ 0) (hc : f.γ ≠ 0) {t : Rat} (ht0 : 0 ≤ t) (ht1 : t ≤ 1) :
    0 < f.at_ t ↔ (0 < f.α ∧ 0 < f.γ) ∨ (f.α < 0 ∧ 0 < f.γ ∧ f.root < t) ∨
      (0 < f.α ∧ f.γ < 0 ∧ t < f.root) := by
  rcases ne_cases ha with ha | ha <;> rcases ne_cases hc with hc | hc
  · have := f.at_neg_nn ha hc ht0 ht1; grind
  · rw [f.at_pos_inc ha hc]; grind
  · rw [f.at_pos_dec ha hc]; grind
  · have := f.at_pos_pp ha hc ht0 ht1; grind

def X (f g h : Aff) : Prop := f.α * f.γ < 0 ∧ 0 < g.at_ f.root ∧ 0 < h.at_ f.root

instance (f g h : Aff) : Decidable (X f g h) := by unfold X; infer_instance

theorem X_iff (f g h : Aff) (hf : f.α ≠ 0 ∧ f.γ ≠ 0) (hg : g.α ≠ 0 ∧ g.γ ≠ 0) (hh : h.α ≠ 0 ∧ h.γ ≠ 0) :
    X f g h ↔ ((f.α < 0 ∧ 0 < f.γ) ∨ (0 < f.α ∧ f.γ < 0)) ∧
      ((0 < g.α ∧ 0 < g.γ) ∨ (g.α < 0 ∧ 0 < g.γ ∧ g.root < f.root) ∨ (0 < g.α ∧ g.γ < 0 ∧ f.root < g.root)) ∧
      ((0 < h.α ∧ 0 < h.γ) ∨ (h.α < 0 ∧ 0 < h.γ ∧ h.root < f.root) ∨ (0 < h.α ∧ h.γ < 0 ∧ f.root < h.root)) := by
  unfold X
  rw [mul_neg_iff _ _ hf.1 hf.2]
  constructor
  · rintro ⟨hc, h1, h2⟩
    have hr : 0 < f.root ∧ f.root < 1 := by
      rcases hc with hc | hc
      · exact f.root_inc hc.1 hc.2
      · exact f.root_dec hc.1 hc.2
    exact ⟨hc, (g.at_pos_iff hg.1 hg.2 (by grind) (by grind)).mp h1,
      (h.at_pos_iff hh.1 hh.2 (by grind) (by grind)).mp h2⟩
  · rintro ⟨hc, h1, h2⟩
    have hr : 0 < f.root ∧ f.root < 1 := by
      rcases hc with hc | hc
      · exact f.root_inc hc.1 hc.2
      · exact f.root_dec hc.1 hc.2
    exact ⟨hc, (g.at_pos_iff hg.1 hg.2 (by grind) (by grind)).mpr h1,
      (h.at_pos_iff hh.1 hh.2 (by grind) (by grind)).mpr h2⟩

theorem roots (f : Aff) (hc : (f.α < 0 ∧ 0 < f.γ) ∨ (0 < f.α ∧ f.γ < 0)) : 0 < f.root ∧ f.root < 1 := by
  rcases hc with hc | hc
  · exact f.root_inc hc.1 hc.2
  · exact f.root_dec hc.1 hc.2

/-- **The sweep, three constraints.** Along [0,1], the boundary crossings of the region where all
three are positive, each at a point where the other two are positive, have the parity of the
region holding at the two ends. -/
theorem sweep3 (f g h : Aff) (hf : f.α ≠ 0 ∧ f.γ ≠ 0) (hg : g.α ≠ 0 ∧ g.γ ≠ 0) (hh : h.α ≠ 0 ∧ h.γ ≠ 0)
    (dfg : f.α * f.γ < 0 → g.α * g.γ < 0 → f.root ≠ g.root)
    (dfh : f.α * f.γ < 0 → h.α * h.γ < 0 → f.root ≠ h.root)
    (dgh : g.α * g.γ < 0 → h.α * h.γ < 0 → g.root ≠ h.root) :
    (X f g h ↔ (X g f h ↔ X h f g)) ↔
      ((0 < f.α ∧ 0 < g.α ∧ 0 < h.α) ↔ ¬(0 < f.γ ∧ 0 < g.γ ∧ 0 < h.γ)) := by
  rw [X_iff f g h hf hg hh, X_iff g f h hg hf hh, X_iff h f g hh hf hg]
  simp only [mul_neg_iff _ _ hf.1 hf.2, mul_neg_iff _ _ hg.1 hg.2, mul_neg_iff _ _ hh.1 hh.2] at dfg dfh dgh
  have rf := fun hc => f.roots hc
  have rg := fun hc => g.roots hc
  have rh := fun hc => h.roots hc
  rcases ne_cases hf.1 with a1 | a1 <;> rcases ne_cases hf.2 with c1 | c1 <;>
  rcases ne_cases hg.1 with a2 | a2 <;> rcases ne_cases hg.2 with c2 | c2 <;>
  rcases ne_cases hh.1 with a3 | a3 <;> rcases ne_cases hh.2 with c3 | c3 <;>
  grind

end Aff

/-! ## Parity of lists -/

def xorL (l : List Bool) : Bool := l.foldr (· ^^ ·) false

@[simp] theorem xorL_nil : xorL [] = false := rfl
@[simp] theorem xorL_cons (b : Bool) (l : List Bool) : xorL (b :: l) = (b ^^ xorL l) := rfl

theorem xorL_append (l m : List Bool) : xorL (l ++ m) = (xorL l ^^ xorL m) := by
  induction l with
  | nil => simp
  | cons b l ih => simp [ih]

theorem xorL_map_xor {α : Type} (l : List α) (f g : α → Bool) :
    xorL (l.map fun x => f x ^^ g x) = (xorL (l.map f) ^^ xorL (l.map g)) := by
  induction l with
  | nil => simp
  | cons x l ih =>
    simp only [List.map_cons, xorL_cons, ih]
    cases f x <;> cases g x <;> cases xorL (l.map f) <;> cases xorL (l.map g) <;> rfl

theorem xorL_flatMap {α β : Type} (l : List α) (h : α → List β) (g : β → Bool) :
    xorL ((l.flatMap h).map g) = xorL (l.map fun x => xorL ((h x).map g)) := by
  induction l with
  | nil => simp
  | cons x l ih => simp [List.flatMap_cons, xorL_append, ih]

theorem xorL_false {α : Type} (l : List α) : xorL (l.map fun _ => false) = false := by
  induction l with
  | nil => rfl
  | cons x l ih => simp [ih]

theorem xorL_congr {α : Type} {l : List α} {f g : α → Bool} (h : ∀ x ∈ l, f x = g x) :
    xorL (l.map f) = xorL (l.map g) := by
  rw [List.map_congr_left h]

/-! ## The edges of a loop, and the telescope round it -/

/-- The edges of a loop, as the reference walks them. -/
def edgesA (lp : Array V3) : List (V3 × V3) :=
  (List.range lp.size).map fun i => (lp[i]!, lp[(i + 1) % lp.size]!)

theorem xorL_rot (h : Nat → Bool) (n : Nat) :
    xorL ((List.range n).map fun i => h ((i + 1) % n)) = xorL ((List.range n).map h) := by
  cases n with
  | zero => rfl
  | succ k =>
    have e1 : (List.range (k + 1)).map (fun i => h ((i + 1) % (k + 1))) =
        (List.range k).map (fun i => h (i + 1)) ++ [h 0] := by
      rw [List.range_succ, List.map_append]
      congr 1
      · apply List.map_congr_left
        intro i hi
        rw [Nat.mod_eq_of_lt (by simp at hi; omega)]
      · simp
    have e2 : (List.range (k + 1)).map h = h 0 :: (List.range k).map (fun i => h (i + 1)) := by
      rw [List.range_succ_eq_map]; simp [Function.comp_def]
    rw [e1, e2, xorL_append]
    simp [Bool.xor_comm]

/-- **Round a closed loop, every end is counted twice.** -/
theorem telescope (g : V3 → Bool) (lp : Array V3) :
    xorL ((edgesA lp).map fun e => g e.1 ^^ g e.2) = false := by
  unfold edgesA
  rw [List.map_map, Function.comp_def]
  simp only
  rw [xorL_map_xor (List.range lp.size) (fun i => g lp[i]!) (fun i => g lp[(i + 1) % lp.size]!),
    xorL_rot (fun i => g lp[i]!)]
  simp

/-! ## Crossing a segment, exactly -/

/-- The reference's side test, unperturbed. -/
def d0 (q a b : V3) : Rat := (b.y - a.y) * (q.z - a.z) - (q.y - a.y) * (b.z - a.z)

/-- The ray from `q` along +y (in the y–z plane) crosses the segment `ab`: its ends straddle
`q.z` and the crossing is past `q`. In general position this is the reference's `crossesYZ`. -/
def crossE (q a b : V3) : Bool :=
  (decide (a.z > q.z) != decide (b.z > q.z)) && decide (0 < d0 q a b * (b.z - a.z))

/-- The side of the line `ww'` (in the y–z plane) a point is on: positive toward the rays. -/
def Fs (w w' x : V3) : Rat := (w'.z - w.z) * (x.y - w.y) - (w'.y - w.y) * (x.z - w.z)

def f1 (w a b : V3) : Aff := ⟨a.z - w.z, b.z - w.z⟩
def f2 (w' a b : V3) : Aff := ⟨w'.z - a.z, w'.z - b.z⟩
def f3 (w w' a b : V3) : Aff := ⟨Fs w w' a, Fs w w' b⟩

/-- The segment `ab` crosses the segment `ww'` (in the y–z plane). -/
def crossS (w w' a b : V3) : Bool := decide (Aff.X (f3 w w' a b) (f1 w a b) (f2 w' a b))

/-- A point in the half-strip between the two rays. -/
def inH (w w' x : V3) : Bool := decide (0 < x.z - w.z ∧ 0 < w'.z - x.z ∧ 0 < Fs w w' x)

theorem root_eq (f : Aff) (h : f.α ≠ f.γ) : f.at_ f.root = 0 := by
  rw [f.at_eq h]; grind

theorem chg_ne {f : Aff} (h : f.α * f.γ < 0) : f.α ≠ f.γ := by
  intro e; rw [e] at h
  rcases (mul_neg_iff _ _ (by intro h0; rw [h0] at h; grind) (by intro h0; rw [h0] at h; grind)).mp h
    with h | h <;> grind

theorem straddle_iff (x a b : Rat) (ha : a ≠ x) (hb : b ≠ x) :
    ((decide (a > x) != decide (b > x)) = true) ↔ (a - x) * (b - x) < 0 := by
  rw [mul_neg_iff _ _ (by grind) (by grind)]
  by_cases h1 : a > x <;> by_cases h2 : b > x <;> simp [h1, h2] <;> grind

theorem sq_pos {d : Rat} (h : d ≠ 0) : 0 < d * d := by
  rcases ne_cases h with h | h
  · have := (mul_pos_iff_neg h (x := d)).mpr h; exact this
  · exact Rat.mul_pos h h

theorem root_mul (f : Aff) (h : f.α ≠ f.γ) : f.root * (f.α - f.γ) = f.α := by
  unfold Aff.root; exact Rat.div_mul_cancel (by grind)

/-- The ray from `w` crosses `ab` iff, along `ab`, the first constraint changes sign at a point
where the other two hold. -/
theorem crossE_iff (w w' a b : V3) (hw : w.z < w'.z) (ha : a.z ≠ w.z) (hb : b.z ≠ w.z) :
    crossE w a b = true ↔ Aff.X (f1 w a b) (f2 w' a b) (f3 w w' a b) := by
  unfold crossE Aff.X
  simp only [Bool.and_eq_true, decide_eq_true_eq]
  rw [straddle_iff _ _ _ ha hb]
  have key : (f1 w a b).α * (f1 w a b).γ < 0 →
      (0 < (f2 w' a b).at_ (f1 w a b).root ∧
       (0 < (f3 w w' a b).at_ (f1 w a b).root ↔ 0 < d0 w a b * (b.z - a.z))) := by
    intro hc
    have hr := root_mul _ (chg_ne hc)
    simp only [f1] at hr
    have hdz : b.z - a.z ≠ 0 := by
      intro h0; have := chg_ne hc; simp only [f1] at this; grind
    have e2 : (f2 w' a b).at_ (f1 w a b).root = w'.z - w.z := by
      simp only [Aff.at_, f2, f1] at *; grind
    have e3 : (f3 w w' a b).at_ (f1 w a b).root * ((b.z - a.z) * (b.z - a.z)) =
        (w'.z - w.z) * (d0 w a b * (b.z - a.z)) := by
      simp only [Aff.at_, f3, f1, Fs, d0] at *; grind
    refine ⟨by rw [e2]; grind, ?_⟩
    have hk : 0 < w'.z - w.z := by grind
    rw [← mul_pos_iff_left (sq_pos hdz) (x := (f3 w w' a b).at_ (f1 w a b).root), Rat.mul_comm, e3,
      mul_pos_iff_left hk]
  constructor
  · rintro ⟨hc, hd⟩
    have := key hc
    exact ⟨hc, this.1, this.2.mpr hd⟩
  · rintro ⟨hc, _, h3⟩
    exact ⟨hc, (key hc).2.mp h3⟩

theorem f3_at_r1 (w w' a b : V3) (hc : (f1 w a b).α * (f1 w a b).γ < 0) :
    (f3 w w' a b).at_ (f1 w a b).root * ((b.z - a.z) * (b.z - a.z)) =
      (w'.z - w.z) * (d0 w a b * (b.z - a.z)) := by
  have hr := root_mul _ (chg_ne hc)
  simp only [Aff.at_, f3, f1, Fs, d0] at *; grind

theorem f3_at_r2 (w w' a b : V3) (hc : (f2 w' a b).α * (f2 w' a b).γ < 0) :
    (f3 w w' a b).at_ (f2 w' a b).root * ((b.z - a.z) * (b.z - a.z)) =
      (w'.z - w.z) * (d0 w' a b * (b.z - a.z)) := by
  have hr := root_mul _ (chg_ne hc)
  simp only [Aff.at_, f3, f2, Fs, d0] at *; grind

theorem dz_ne1 {w a b : V3} (hc : (f1 w a b).α * (f1 w a b).γ < 0) : b.z - a.z ≠ 0 := by
  intro h0; have := chg_ne hc; simp only [f1] at this; grind

theorem dz_ne2 {w' a b : V3} (hc : (f2 w' a b).α * (f2 w' a b).γ < 0) : b.z - a.z ≠ 0 := by
  intro h0; have := chg_ne hc; simp only [f2] at this; grind

theorem crossE_iff' (w w' a b : V3) (hw : w.z < w'.z) (ha : a.z ≠ w'.z) (hb : b.z ≠ w'.z) :
    crossE w' a b = true ↔ Aff.X (f2 w' a b) (f1 w a b) (f3 w w' a b) := by
  unfold crossE Aff.X
  simp only [Bool.and_eq_true, decide_eq_true_eq]
  rw [straddle_iff _ _ _ ha hb]
  have hp : (a.z - w'.z) * (b.z - w'.z) = (f2 w' a b).α * (f2 w' a b).γ := by simp only [f2]; grind
  rw [hp]
  have key : (f2 w' a b).α * (f2 w' a b).γ < 0 →
      (0 < (f1 w a b).at_ (f2 w' a b).root ∧
       (0 < (f3 w w' a b).at_ (f2 w' a b).root ↔ 0 < d0 w' a b * (b.z - a.z))) := by
    intro hc
    have hr := root_mul _ (chg_ne hc)
    simp only [f2] at hr
    have e1 : (f1 w a b).at_ (f2 w' a b).root = w'.z - w.z := by
      simp only [Aff.at_, f2, f1] at *; grind
    refine ⟨by rw [e1]; grind, ?_⟩
    have hk : 0 < w'.z - w.z := by grind
    rw [← mul_pos_iff_left (sq_pos (dz_ne2 hc)) (x := (f3 w w' a b).at_ (f2 w' a b).root),
      Rat.mul_comm, f3_at_r2 w w' a b hc, mul_pos_iff_left hk]
  constructor
  · rintro ⟨hc, hd⟩
    exact ⟨hc, (key hc).1, (key hc).2.mpr hd⟩
  · rintro ⟨hc, _, h3⟩
    exact ⟨hc, (key hc).2.mp h3⟩

theorem xor3_of_iff {P Q R S T : Prop} [Decidable P] [Decidable Q] [Decidable R] [Decidable S]
    [Decidable T] (h : (P ↔ (Q ↔ R)) ↔ (S ↔ ¬T)) :
    (decide P ^^ decide Q ^^ decide R) = (decide S ^^ decide T) := by
  by_cases hP : P <;> by_cases hQ : Q <;> by_cases hR : R <;> by_cases hS : S <;> by_cases hT : T <;>
    simp_all

/-- General position of a move `w → w'` against one edge `ab`. -/
structure GP (w w' a b : V3) : Prop where
  a1 : a.z ≠ w.z
  b1 : b.z ≠ w.z
  a2 : a.z ≠ w'.z
  b2 : b.z ≠ w'.z
  fa : Fs w w' a ≠ 0
  fb : Fs w w' b ≠ 0
  d1 : (a.z - w.z) * (b.z - w.z) < 0 → d0 w a b ≠ 0
  d2 : (a.z - w'.z) * (b.z - w'.z) < 0 → d0 w' a b ≠ 0

/-- **The identity along one edge.** For a move with `w.z < w'.z`, the edge's crossings of the two
rays and of the segment between them have the parity of its two ends lying in the half-strip. -/
theorem seg_identity (w w' a b : V3) (hw : w.z < w'.z) (g : GP w w' a b) :
    (crossE w a b ^^ crossE w' a b ^^ crossS w w' a b) = (inH w w' a ^^ inH w w' b) := by
  have h1 : crossE w a b = decide (Aff.X (f1 w a b) (f2 w' a b) (f3 w w' a b)) := by
    rw [Bool.eq_iff_iff, crossE_iff w w' a b hw g.a1 g.b1]; simp
  have h2 : crossE w' a b = decide (Aff.X (f2 w' a b) (f1 w a b) (f3 w w' a b)) := by
    rw [Bool.eq_iff_iff, crossE_iff' w w' a b hw g.a2 g.b2]; simp
  rw [h1, h2]
  unfold crossS inH
  have hf1 : (f1 w a b).α ≠ 0 ∧ (f1 w a b).γ ≠ 0 := ⟨by simp [f1]; grind [g.a1], by simp [f1]; grind [g.b1]⟩
  have hf2 : (f2 w' a b).α ≠ 0 ∧ (f2 w' a b).γ ≠ 0 := ⟨by simp [f2]; grind [g.a2], by simp [f2]; grind [g.b2]⟩
  have hf3 : (f3 w w' a b).α ≠ 0 ∧ (f3 w w' a b).γ ≠ 0 := ⟨g.fa, g.fb⟩
  have sw := Aff.sweep3 (f1 w a b) (f2 w' a b) (f3 w w' a b) hf1 hf2 hf3
    (fun c1 c2 e => by
      have z1 := root_eq _ (chg_ne c1)
      have z2 := root_eq _ (chg_ne c2)
      rw [← e] at z2
      simp only [Aff.at_, f1, f2] at z1 z2; grind)
    (fun c1 c3 e => by
      have z3 := root_eq _ (chg_ne c3)
      rw [← e] at z3
      have := f3_at_r1 w w' a b c1
      rw [z3] at this
      have hk : 0 < w'.z - w.z := by grind
      have hd := g.d1 (by simpa [f1] using c1)
      have hdz := dz_ne1 c1
      have : d0 w a b * (b.z - a.z) = 0 := by
        rcases Rat.mul_eq_zero.mp (by grind : (w'.z - w.z) * (d0 w a b * (b.z - a.z)) = 0) with h | h
        · grind
        · exact h
      rcases Rat.mul_eq_zero.mp this with h | h <;> contradiction)
    (fun c2 c3 e => by
      have z3 := root_eq _ (chg_ne c3)
      rw [← e] at z3
      have := f3_at_r2 w w' a b c2
      rw [z3] at this
      have hd := g.d2 (by have : (a.z - w'.z) * (b.z - w'.z) = (f2 w' a b).α * (f2 w' a b).γ := by
                              simp only [f2]; grind
                          rw [this]; exact c2)
      have hdz := dz_ne2 c2
      have : d0 w' a b * (b.z - a.z) = 0 := by
        rcases Rat.mul_eq_zero.mp (by grind : (w'.z - w.z) * (d0 w' a b * (b.z - a.z)) = 0) with h | h
        · grind
        · exact h
      rcases Rat.mul_eq_zero.mp this with h | h <;> contradiction)
  have := xor3_of_iff sw
  rw [this]
  simp only [f1, f2, f3]
  congr 1 <;> (apply Bool.eq_iff_iff.mpr; simp; grind)

/-! ## A move along a level: the two rays on one line -/

/-- The point at `t` along `ab`. -/
def ptv (a b : V3) (t : Rat) : V3 := ⟨a.x + t * (b.x - a.x), a.y + t * (b.y - a.y), a.z + t * (b.z - a.z)⟩

/-- `ab` crosses the level segment from `w` to `w'` (`w.z = w'.z`, `w.y < w'.y`). -/
def crossH (w w' a b : V3) : Bool :=
  decide ((f1 w a b).α * (f1 w a b).γ < 0 ∧ w.y < (ptv a b (f1 w a b).root).y ∧
    (ptv a b (f1 w a b).root).y < w'.y)

theorem ey_mul (w a b : V3) (hc : (f1 w a b).α * (f1 w a b).γ < 0) :
    ((ptv a b (f1 w a b).root).y - w.y) * (b.z - a.z) = d0 w a b := by
  have hr := root_mul _ (chg_ne hc)
  simp only [ptv, f1, d0] at *; grind

theorem crossE_pos (w a b : V3) (ha : a.z ≠ w.z) (hb : b.z ≠ w.z) :
    crossE w a b = true ↔ (f1 w a b).α * (f1 w a b).γ < 0 ∧ w.y < (ptv a b (f1 w a b).root).y := by
  unfold crossE
  simp only [Bool.and_eq_true, decide_eq_true_eq]
  rw [straddle_iff _ _ _ ha hb]
  constructor
  · rintro ⟨hc, hd⟩
    refine ⟨hc, ?_⟩
    have := ey_mul w a b hc
    have h' : 0 < ((ptv a b (f1 w a b).root).y - w.y) * ((b.z - a.z) * (b.z - a.z)) := by
      rw [show ((ptv a b (f1 w a b).root).y - w.y) * ((b.z - a.z) * (b.z - a.z)) =
        d0 w a b * (b.z - a.z) by rw [← this]; grind]; exact hd
    rw [Rat.mul_comm, mul_pos_iff_left (sq_pos (dz_ne1 hc))] at h'
    grind
  · rintro ⟨hc, hy⟩
    refine ⟨hc, ?_⟩
    rw [← ey_mul w a b hc, show ((ptv a b (f1 w a b).root).y - w.y) * (b.z - a.z) * (b.z - a.z) =
      ((b.z - a.z) * (b.z - a.z)) * ((ptv a b (f1 w a b).root).y - w.y) by grind]
    exact Rat.mul_pos (sq_pos (dz_ne1 hc)) (by grind)

/-- **The identity along one edge, for a level move.** -/
theorem seg_level (w w' a b : V3) (hz : w.z = w'.z) (hy : w.y < w'.y) (ha : a.z ≠ w.z)
    (hb : b.z ≠ w.z) (hd2 : (a.z - w.z) * (b.z - w.z) < 0 → d0 w' a b ≠ 0) :
    (crossE w a b ^^ crossE w' a b) = crossH w w' a b := by
  have ha' : a.z ≠ w'.z := hz ▸ ha
  have hb' : b.z ≠ w'.z := hz ▸ hb
  have e1 := crossE_pos w a b ha hb
  have e2 := crossE_pos w' a b ha' hb'
  have same : f1 w' a b = f1 w a b := by simp [f1, hz]
  rw [same] at e2
  unfold crossH
  by_cases hc : (f1 w a b).α * (f1 w a b).γ < 0
  · have hne : (ptv a b (f1 w a b).root).y ≠ w'.y := by
      intro h0
      have := ey_mul w' a b (by rw [same]; exact hc)
      rw [same, h0] at this
      apply hd2 (by simpa [f1] using hc); grind
    cases h1 : crossE w a b <;> cases h2 : crossE w' a b <;> simp_all <;> grind
  · cases h1 : crossE w a b <;> cases h2 : crossE w' a b <;> simp_all

/-! ## A face: the crossings that count are those of faces ahead -/

/-- Strictly on the face plane's far side along +x: the ray from `q` meets the plane past `q`. -/
def aheadB (f : Face) (q : V3) : Bool := decide (0 < (f.plane.c - f.plane.n.dot q) * f.plane.n.x)

def edgesF (f : Face) : List (V3 × V3) := f.loops.toList.flatMap edgesA

def chiE (f : Face) (q : V3) : Bool := xorL ((edgesF f).map fun e => crossE q e.1 e.2)

/-- The face is pierced by the exact ray from `q`. -/
def piercedE (f : Face) (q : V3) : Bool := aheadB f q && chiE f q

theorem eval_ptv (P : Plane) (a b : V3) (t : Rat) :
    P.eval (ptv a b t) = P.eval a + t * (P.eval b - P.eval a) := by
  simp only [Plane.eval, V3.dot, ptv]; grind

theorem sign_keep {k x y : Rat} (hk : 0 < k) (h : k * x = y) : 0 < x ↔ 0 < y := by
  rw [← h, mul_pos_iff_left hk]

/-- A point `E` on the face plane, over the point `B` of the segment `ww'` (same y and z), lies past
`B` along +x exactly when the face is ahead; and a face parallel to +x is never over the segment. -/
theorem ahead_of_over (P : Plane) (w w' E : V3) (l : Rat) (hl0 : 0 < l) (hl1 : l < 1)
    (hside : 0 < P.eval w * P.eval w') (hE : P.eval E = 0)
    (hy : (ptv w w' l).y = E.y) (hz : (ptv w w' l).z = E.z) :
    P.n.x ≠ 0 ∧ ((ptv w w' l).x < E.x ↔ 0 < (P.c - P.n.dot w) * P.n.x) := by
  have hB := eval_ptv P w w' l
  generalize ptv w w' l = B at hB hy hz ⊢
  have hdiff : P.n.x * (E.x - B.x) = -P.eval B := by
    simp only [Plane.eval, V3.dot] at hE hB ⊢
    rw [← hy, ← hz] at hE; grind
  -- P(B) has the side of P(w)
  have hpos : 0 < P.eval B * P.eval w := by
    rw [hB]
    have e : (P.eval w + l * (P.eval w' - P.eval w)) * P.eval w =
        (1 - l) * (P.eval w * P.eval w) + l * (P.eval w * P.eval w') := by grind
    rw [e]
    have hww : 0 < P.eval w * P.eval w := sq_pos (by intro h0; rw [h0] at hside; grind)
    have := Rat.mul_pos (by grind : (0 : Rat) < 1 - l) hww
    have := Rat.mul_pos hl0 hside
    grind
  have hnx : P.n.x ≠ 0 := by
    intro h0; rw [h0] at hdiff
    have : P.eval B = 0 := by grind
    rw [this] at hpos; grind
  refine ⟨hnx, ?_⟩
  have hw0 : P.eval w ≠ 0 := by intro h0; rw [h0] at hpos; grind
  -- 0 < (c - n·w) nx ⟺ 0 < -P(B) nx ⟺ 0 < E.x - B.x
  have s1 : 0 < (P.c - P.n.dot w) * P.n.x ↔ 0 < -P.eval B * P.n.x := by
    have e : (P.eval B * P.eval w) * ((P.c - P.n.dot w) * P.n.x) =
        (P.eval w * P.eval w) * (-P.eval B * P.n.x) := by simp only [Plane.eval]; grind
    rw [← mul_pos_iff_left hpos (x := (P.c - P.n.dot w) * P.n.x), e, mul_pos_iff_left (sq_pos hw0)]
  have s2 : 0 < E.x - B.x ↔ 0 < -P.eval B * P.n.x := by
    apply sign_keep (k := P.n.x * P.n.x) (sq_pos hnx)
    rw [← hdiff]; grind
  rw [s1, ← s2]; grind

theorem mul_cancel {k x y : Rat} (hk : k ≠ 0) (h : k * x = k * y) : x = y := by
  have : k * (x - y) = 0 := by grind
  rcases Rat.mul_eq_zero.mp this with h0 | h0
  · exact absurd h0 hk
  · grind

theorem pos_of_mul {k x : Rat} (hk : 0 < k) (h : 0 < x * k) : 0 < x := by
  rw [Rat.mul_comm, mul_pos_iff_left hk] at h; exact h

theorem f1_at (w a b : V3) (t : Rat) : (f1 w a b).at_ t = (ptv a b t).z - w.z := by
  simp only [Aff.at_, f1, ptv]; grind
theorem f2_at (w' a b : V3) (t : Rat) : (f2 w' a b).at_ t = w'.z - (ptv a b t).z := by
  simp only [Aff.at_, f2, ptv]; grind
theorem f3_at (w w' a b : V3) (t : Rat) : (f3 w w' a b).at_ t = Fs w w' (ptv a b t) := by
  simp only [Aff.at_, f3, Fs, ptv]; grind

/-- Where `ab` crosses the segment, as a fraction of the segment. -/
def lamS (w w' a b : V3) : Rat := ((ptv a b (f3 w w' a b).root).z - w.z) / (w'.z - w.z)

/-- An edge crossing the half-strip swept by the ray along `ww'`: across the segment, at a point
past it along +x. A function of the edge alone (`e3_symm`). -/
def e3 (w w' : V3) (e : V3 × V3) : Bool :=
  crossS w w' e.1 e.2 && decide ((ptv w w' (lamS w w' e.1 e.2)).x < (ptv e.1 e.2 (f3 w w' e.1 e.2).root).x)

theorem over_S (w w' a b : V3) (hw : w.z < w'.z) (h : crossS w w' a b = true) :
    0 < lamS w w' a b ∧ lamS w w' a b < 1 ∧
    (ptv w w' (lamS w w' a b)).y = (ptv a b (f3 w w' a b).root).y ∧
    (ptv w w' (lamS w w' a b)).z = (ptv a b (f3 w w' a b).root).z := by
  unfold crossS Aff.X at h
  simp only [decide_eq_true_eq] at h
  obtain ⟨hc, h1, h2⟩ := h
  rw [f1_at] at h1; rw [f2_at] at h2
  have hk : 0 < w'.z - w.z := by grind
  have hl := Rat.div_mul_cancel (a := (ptv a b (f3 w w' a b).root).z - w.z) (b := w'.z - w.z) (by grind)
  have hF : Fs w w' (ptv a b (f3 w w' a b).root) = 0 := by rw [← f3_at]; exact root_eq _ (chg_ne hc)
  unfold lamS
  generalize (ptv a b (f3 w w' a b).root) = E at *
  generalize hL : (E.z - w.z) / (w'.z - w.z) = L at hl
  refine ⟨pos_of_mul hk (by grind), ?_, ?_, ?_⟩
  · have := pos_of_mul hk (x := 1 - L) (by grind); grind
  · simp only [ptv]
    simp only [Fs] at hF
    have : (w'.z - w.z) * (E.y - w.y) = (w'.z - w.z) * (L * (w'.y - w.y)) := by grind
    have := mul_cancel (by grind) this; grind
  · simp only [ptv]; grind

theorem plane_zero (P : Plane) (a b : V3) (t : Rat) (ha : P.eval a = 0) (hb : P.eval b = 0) :
    P.eval (ptv a b t) = 0 := by rw [eval_ptv, ha, hb]; grind

/-- The ahead test agrees at the two ends of a segment on one side of the plane. -/
theorem ahead_same (P : Plane) (w w' : V3) (hside : 0 < P.eval w * P.eval w') :
    (0 < (P.c - P.n.dot w) * P.n.x ↔ 0 < (P.c - P.n.dot w') * P.n.x) := by
  have hw0 : P.eval w ≠ 0 := by intro h0; rw [h0] at hside; grind
  have e : (P.eval w * P.eval w') * ((P.c - P.n.dot w) * P.n.x) =
      (P.eval w * P.eval w) * ((P.c - P.n.dot w') * P.n.x) := by simp only [Plane.eval]; grind
  rw [← mul_pos_iff_left hside (x := (P.c - P.n.dot w) * P.n.x), e, mul_pos_iff_left (sq_pos hw0)]

theorem xor_rearrange (x y z p : Bool) (h : (x ^^ y ^^ z) = p) : (x ^^ y) = (z ^^ p) := by
  subst h; cases x <;> cases y <;> cases z <;> rfl

/-- **One face, one move.** Its pierce status changes by the parity of its edges crossing the
half-strip past the segment. -/
theorem face_step (f : Face) (w w' : V3) (hw : w.z < w'.z)
    (hflat : ∀ e ∈ edgesF f, f.plane.eval e.1 = 0 ∧ f.plane.eval e.2 = 0)
    (hside : 0 < f.plane.eval w * f.plane.eval w') (hgp : ∀ e ∈ edgesF f, GP w w' e.1 e.2) :
    (piercedE f w ^^ piercedE f w') = xorL ((edgesF f).map (e3 w w')) := by
  have hchi : (chiE f w ^^ chiE f w') = xorL ((edgesF f).map fun e => crossS w w' e.1 e.2) := by
    unfold chiE
    have per : ∀ e ∈ edgesF f, (crossE w e.1 e.2 ^^ crossE w' e.1 e.2) =
        (crossS w w' e.1 e.2 ^^ (inH w w' e.1 ^^ inH w w' e.2)) := fun e he =>
      xor_rearrange _ _ _ _ (seg_identity w w' e.1 e.2 hw (hgp e he))
    rw [← xorL_map_xor (edgesF f) (fun e => crossE w e.1 e.2) (fun e => crossE w' e.1 e.2),
      xorL_congr per, xorL_map_xor (edgesF f) (fun e => crossS w w' e.1 e.2)
        (fun e => inH w w' e.1 ^^ inH w w' e.2)]
    have : xorL ((edgesF f).map fun e => inH w w' e.1 ^^ inH w w' e.2) = false := by
      unfold edgesF
      rw [xorL_flatMap]
      rw [xorL_congr (g := fun _ => false) (fun lp _ => telescope (inH w w') lp), xorL_false]
    rw [this]; simp
  have hahead : aheadB f w = aheadB f w' := by
    unfold aheadB; exact decide_eq_decide.mpr (ahead_same _ _ _ hside)
  have he3 : ∀ e ∈ edgesF f, e3 w w' e = (crossS w w' e.1 e.2 && aheadB f w) := by
    intro e he
    unfold e3
    cases hc : crossS w w' e.1 e.2
    · rfl
    · obtain ⟨l0, l1, hy, hz⟩ := over_S w w' e.1 e.2 hw hc
      have hE := plane_zero f.plane e.1 e.2 (f3 w w' e.1 e.2).root (hflat e he).1 (hflat e he).2
      have := (ahead_of_over f.plane w w' _ _ l0 l1 hside hE hy hz).2
      simp only [Bool.true_and, aheadB]
      exact decide_eq_decide.mpr this
  rw [xorL_congr he3]
  unfold piercedE
  cases ha : aheadB f w
  · rw [← hahead, ha]; simp only [Bool.false_and, Bool.and_false]; rw [xorL_false]; rfl
  · rw [← hahead, ha]; simp only [Bool.true_and, Bool.and_true]; exact hchi

/-- Where `ab` crosses a level segment, as a fraction of it. -/
def lamH (w w' a b : V3) : Rat := ((ptv a b (f1 w a b).root).y - w.y) / (w'.y - w.y)

def eH (w w' : V3) (e : V3 × V3) : Bool :=
  crossH w w' e.1 e.2 && decide ((ptv w w' (lamH w w' e.1 e.2)).x < (ptv e.1 e.2 (f1 w e.1 e.2).root).x)

theorem over_H (w w' a b : V3) (hz : w.z = w'.z) (hy : w.y < w'.y) (h : crossH w w' a b = true) :
    0 < lamH w w' a b ∧ lamH w w' a b < 1 ∧
    (ptv w w' (lamH w w' a b)).y = (ptv a b (f1 w a b).root).y ∧
    (ptv w w' (lamH w w' a b)).z = (ptv a b (f1 w a b).root).z := by
  unfold crossH at h
  simp only [decide_eq_true_eq] at h
  obtain ⟨hc, h1, h2⟩ := h
  have hk : 0 < w'.y - w.y := by grind
  have hl := Rat.div_mul_cancel (a := (ptv a b (f1 w a b).root).y - w.y) (b := w'.y - w.y) (by grind)
  have hE : (ptv a b (f1 w a b).root).z = w.z := by
    have := root_eq _ (chg_ne hc); rw [f1_at] at this; grind
  unfold lamH
  generalize (ptv a b (f1 w a b).root) = E at *
  generalize (E.y - w.y) / (w'.y - w.y) = L at hl
  refine ⟨pos_of_mul hk (by grind), ?_, ?_, ?_⟩
  · have := pos_of_mul hk (x := 1 - L) (by grind); grind
  · simp only [ptv]; grind
  · simp only [ptv]; grind

/-- GP of a level move against an edge. -/
structure GPH (w w' a b : V3) : Prop where
  a1 : a.z ≠ w.z
  b1 : b.z ≠ w.z
  d2 : (a.z - w.z) * (b.z - w.z) < 0 → d0 w' a b ≠ 0

theorem face_level (f : Face) (w w' : V3) (hz : w.z = w'.z) (hy : w.y < w'.y)
    (hflat : ∀ e ∈ edgesF f, f.plane.eval e.1 = 0 ∧ f.plane.eval e.2 = 0)
    (hside : 0 < f.plane.eval w * f.plane.eval w') (hgp : ∀ e ∈ edgesF f, GPH w w' e.1 e.2) :
    (piercedE f w ^^ piercedE f w') = xorL ((edgesF f).map (eH w w')) := by
  have hchi : (chiE f w ^^ chiE f w') = xorL ((edgesF f).map fun e => crossH w w' e.1 e.2) := by
    unfold chiE
    have per : ∀ e ∈ edgesF f, (crossE w e.1 e.2 ^^ crossE w' e.1 e.2) = crossH w w' e.1 e.2 :=
      fun e he => seg_level w w' e.1 e.2 hz hy (hgp e he).a1 (hgp e he).b1 (hgp e he).d2
    rw [← xorL_map_xor (edgesF f) (fun e => crossE w e.1 e.2) (fun e => crossE w' e.1 e.2)]
    exact xorL_congr per
  have hahead : aheadB f w = aheadB f w' := by
    unfold aheadB; exact decide_eq_decide.mpr (ahead_same _ _ _ hside)
  have he3 : ∀ e ∈ edgesF f, eH w w' e = (crossH w w' e.1 e.2 && aheadB f w) := by
    intro e he
    unfold eH
    cases hc : crossH w w' e.1 e.2
    · rfl
    · obtain ⟨l0, l1, hy', hz'⟩ := over_H w w' e.1 e.2 hz hy hc
      have hE := plane_zero f.plane e.1 e.2 (f1 w e.1 e.2).root (hflat e he).1 (hflat e he).2
      have := (ahead_of_over f.plane w w' _ _ l0 l1 hside hE hy' hz').2
      simp only [Bool.true_and, aheadB]
      exact decide_eq_decide.mpr this
  rw [xorL_congr he3]
  unfold piercedE
  cases ha : aheadB f w
  · rw [← hahead, ha]; simp only [Bool.false_and, Bool.and_false]; rw [xorL_false]; rfl
  · rw [← hahead, ha]; simp only [Bool.true_and, Bool.and_true]; exact hchi

/-! ## An edge's crossing does not depend on its direction -/

def Aff.rev (f : Aff) : Aff := ⟨f.γ, f.α⟩

theorem rev_at (f : Aff) (t : Rat) : f.rev.at_ (1 - t) = f.at_ t := by
  simp only [Aff.rev, Aff.at_]; grind

theorem rev_root (f : Aff) (h : f.α ≠ f.γ) : f.rev.root = 1 - f.root := by
  have h1 := root_mul f h
  have h2 : f.rev.root * (f.γ - f.α) = f.γ := root_mul f.rev (by simp [Aff.rev]; grind)
  apply mul_cancel (k := f.γ - f.α) (by grind)
  grind

theorem X_rev (f g h : Aff) : Aff.X f.rev g.rev h.rev ↔ Aff.X f g h := by
  unfold Aff.X
  have hc : f.rev.α * f.rev.γ = f.α * f.γ := by simp [Aff.rev, Rat.mul_comm]
  rw [hc]
  constructor
  · rintro ⟨c, h1, h2⟩
    rw [rev_root f (chg_ne c), rev_at] at h1 h2
    exact ⟨c, h1, h2⟩
  · rintro ⟨c, h1, h2⟩
    refine ⟨c, ?_, ?_⟩ <;> rw [rev_root f (chg_ne c), rev_at] <;> assumption

theorem ptv_rev (a b : V3) (t : Rat) : ptv b a (1 - t) = ptv a b t := by
  simp only [ptv, V3.mk.injEq]; grind

theorem crossS_symm (w w' a b : V3) : crossS w w' b a = crossS w w' a b := by
  unfold crossS
  have e1 : f1 w b a = (f1 w a b).rev := rfl
  have e2 : f2 w' b a = (f2 w' a b).rev := rfl
  have e3 : f3 w w' b a = (f3 w w' a b).rev := rfl
  rw [e1, e2, e3]
  exact decide_eq_decide.mpr (X_rev _ _ _)

theorem e3_symm (w w' : V3) (e : V3 × V3) : e3 w w' (e.2, e.1) = e3 w w' e := by
  unfold e3
  simp only
  rw [crossS_symm]
  cases hc : crossS w w' e.1 e.2
  · rfl
  · unfold crossS Aff.X at hc
    simp only [decide_eq_true_eq] at hc
    have hr : (f3 w w' e.2 e.1).root = 1 - (f3 w w' e.1 e.2).root := rev_root _ (chg_ne hc.1)
    have hE : ptv e.2 e.1 (f3 w w' e.2 e.1).root = ptv e.1 e.2 (f3 w w' e.1 e.2).root := by
      rw [hr, ptv_rev]
    simp only [lamS, hE]
    try rfl

theorem crossH_symm (w w' a b : V3) : crossH w w' b a = crossH w w' a b := by
  unfold crossH
  have e1 : f1 w b a = (f1 w a b).rev := rfl
  rw [e1]
  apply decide_eq_decide.mpr
  have hc : (f1 w a b).rev.α * (f1 w a b).rev.γ = (f1 w a b).α * (f1 w a b).γ := by
    simp [Aff.rev, Rat.mul_comm]
  rw [hc]
  constructor
  · rintro ⟨c, h1, h2⟩
    rw [rev_root _ (chg_ne c), ptv_rev] at h1 h2
    exact ⟨c, h1, h2⟩
  · rintro ⟨c, h1, h2⟩
    rw [rev_root _ (chg_ne c), ptv_rev]
    exact ⟨c, h1, h2⟩

theorem eH_symm (w w' : V3) (e : V3 × V3) : eH w w' (e.2, e.1) = eH w w' e := by
  unfold eH
  simp only
  rw [crossH_symm]
  cases hc : crossH w w' e.1 e.2
  · rfl
  · unfold crossH at hc
    simp only [decide_eq_true_eq] at hc
    have hr : (f1 w e.2 e.1).root = 1 - (f1 w e.1 e.2).root := rev_root (f1 w e.1 e.2) (chg_ne hc.1)
    have hE : ptv e.2 e.1 (f1 w e.2 e.1).root = ptv e.1 e.2 (f1 w e.1 e.2).root := by
      rw [hr, ptv_rev]
    simp only [lamH, hE]
    try rfl

/-! ## A closed surface counts every edge an even number of times -/

/-- The same edge, either way round. -/
def sameEdge (e d : V3 × V3) : Bool := decide (d = e) || decide (d = (e.2, e.1))

theorem xorL_filter {α : Type} (p : α → Bool) (g : α → Bool) (l : List α) :
    xorL (l.map g) = (xorL ((l.filter p).map g) ^^ xorL ((l.filter fun x => !p x).map g)) := by
  induction l with
  | nil => rfl
  | cons x l ih =>
    simp only [List.map_cons, xorL_cons, List.filter_cons, ih]
    cases p x <;> simp <;> cases g x <;> cases xorL ((l.filter p).map g) <;>
      cases xorL ((l.filter fun x => !p x).map g) <;> rfl

theorem xorL_const {α : Type} (g : α → Bool) (c : Bool) :
    ∀ (l : List α), (∀ x ∈ l, g x = c) → l.length % 2 = 0 → xorL (l.map g) = false
  | [], _, _ => rfl
  | [_], _, h => by simp at h
  | x :: y :: l, hc, hl => by
    simp only [List.map_cons, xorL_cons]
    rw [hc x (by simp), hc y (by simp),
      xorL_const g c l (fun z hz => hc z (by simp [hz])) (by simp at hl; omega)]
    cases c <;> rfl

theorem sameEdge_iff (e d : V3 × V3) : sameEdge e d = true ↔ d = e ∨ d = (e.2, e.1) := by
  simp [sameEdge]

/-- **Even multiplicities cancel.** If every edge occurs an even number of times (either way
round), the parity of any direction-blind property over all of them is even. -/
theorem xor_even (g : V3 × V3 → Bool) (hg : ∀ e, g (e.2, e.1) = g e) :
    ∀ (n : Nat) (l : List (V3 × V3)), l.length ≤ n →
      (∀ e, (l.countP (sameEdge e)) % 2 = 0) → xorL (l.map g) = false
  | 0, l, hn, _ => by
    have : l = [] := List.length_eq_zero_iff.mp (by omega)
    subst this; rfl
  | n + 1, [], _, _ => rfl
  | n + 1, e :: l, hn, hc => by
    rw [xorL_filter (sameEdge e)]
    have h1 : xorL (((e :: l).filter (sameEdge e)).map g) = false := by
      apply xorL_const g (g e)
      · intro d hd
        rcases (sameEdge_iff e d).mp (List.mem_filter.mp hd).2 with rfl | rfl
        · rfl
        · exact hg e
      · rw [← List.countP_eq_length_filter]; exact hc e
    rw [h1]
    simp only [Bool.false_xor]
    apply xor_even g hg n
    · have : ((e :: l).filter fun x => !sameEdge e x).length ≤ l.length := by
        simp only [List.filter_cons]
        have hs : sameEdge e e = true := by simp [sameEdge]
        simp only [hs, Bool.not_true, Bool.false_eq_true, ite_false]
        exact List.length_filter_le _ _
      simp at hn; omega
    · intro e'
      rw [List.countP_filter]
      by_cases hm : e' = e ∨ e' = (e.2, e.1)
      · -- e' is e's edge: none is left
        rw [List.countP_eq_zero.mpr]
        intro d _ hd
        simp only [Bool.and_eq_true, Bool.not_eq_true', sameEdge, Bool.or_eq_true, decide_eq_true_eq,
          Bool.or_eq_false_iff, decide_eq_false_iff_not] at hd
        obtain ⟨hd1, hd2, hd3⟩ := hd
        rcases hm with rfl | rfl <;> rcases hd1 with rfl | rfl <;> simp_all
      · -- otherwise, every match of e' is no match of e
        rw [← hc e']
        congr 1
        apply List.countP_congr
        intro d _
        simp only [Bool.and_eq_true, Bool.not_eq_true', sameEdge, Bool.or_eq_true, decide_eq_true_eq,
          Bool.or_eq_false_iff, decide_eq_false_iff_not]
        constructor
        · exact fun h => h.1
        · intro h
          rcases e with ⟨p1, p2⟩; rcases e' with ⟨q1, q2⟩; rcases d with ⟨r1, r2⟩
          refine ⟨h, ?_, ?_⟩ <;> intro h' <;> apply hm <;> rcases h with h | h <;>
            simp only [Prod.mk.injEq] at h h' ⊢ <;> grind

/-! ## The solid -/

def facesL (S : Solid) : List Face := S.faces.toList
def allEdges (S : Solid) : List (V3 × V3) := (facesL S).flatMap edgesF

/-- The exact membership: the parity of faces pierced by the exact ray. -/
def containsE (S : Solid) (q : V3) : Bool := xorL ((facesL S).map fun f => piercedE f q)

/-- Every face's loops lie in its plane. -/
def FacesFlat (S : Solid) : Prop :=
  ∀ f ∈ facesL S, ∀ e ∈ edgesF f, f.plane.eval e.1 = 0 ∧ f.plane.eval e.2 = 0

/-- **Closed**: every edge is used an even number of times, counted either way round (a
B-rep's edges are used once in each direction). -/
def Closed (S : Solid) : Prop := ∀ e, ((allEdges S).countP (sameEdge e)) % 2 = 0

theorem xorL_eq_of_xor {a b : Bool} (h : (a ^^ b) = false) : a = b := by
  cases a <;> cases b <;> simp_all

/-- **One move keeps the parity**, for a move that rises (`w.z < w'.z`). -/
theorem move_rise (S : Solid) (hflat : FacesFlat S) (hcl : Closed S) (w w' : V3) (hw : w.z < w'.z)
    (hside : ∀ f ∈ facesL S, 0 < f.plane.eval w * f.plane.eval w')
    (hgp : ∀ f ∈ facesL S, ∀ e ∈ edgesF f, GP w w' e.1 e.2) :
    containsE S w = containsE S w' := by
  apply xorL_eq_of_xor
  unfold containsE
  rw [← xorL_map_xor, xorL_congr fun f hf => face_step f w w' hw (hflat f hf) (hside f hf) (hgp f hf),
    ← xorL_flatMap]
  exact xor_even (e3 w w') (e3_symm w w') _ _ (Nat.le_refl _) hcl

/-- And for a level move (`w.z = w'.z`, `w.y < w'.y`). -/
theorem move_level (S : Solid) (hflat : FacesFlat S) (hcl : Closed S) (w w' : V3)
    (hz : w.z = w'.z) (hy : w.y < w'.y)
    (hside : ∀ f ∈ facesL S, 0 < f.plane.eval w * f.plane.eval w')
    (hgp : ∀ f ∈ facesL S, ∀ e ∈ edgesF f, GPH w w' e.1 e.2) :
    containsE S w = containsE S w' := by
  apply xorL_eq_of_xor
  unfold containsE
  rw [← xorL_map_xor, xorL_congr fun f hf => face_level f w w' hz hy (hflat f hf) (hside f hf) (hgp f hf),
    ← xorL_flatMap]
  exact xor_even (eH w w') (eH_symm w w') _ _ (Nat.le_refl _) hcl

/-- And along +x: the ray's line does not move. -/
theorem move_x (S : Solid) (w w' : V3) (hy : w.y = w'.y) (hz : w.z = w'.z)
    (hside : ∀ f ∈ facesL S, 0 < f.plane.eval w * f.plane.eval w') :
    containsE S w = containsE S w' := by
  unfold containsE
  apply xorL_congr
  intro f hf
  have hchi : chiE f w = chiE f w' := by
    unfold chiE
    apply xorL_congr
    intro e _
    simp only [crossE, d0, hy, hz]
    try rfl
  have hahead : aheadB f w = aheadB f w' := by
    unfold aheadB; exact decide_eq_decide.mpr (ahead_same _ _ _ (hside f hf))
  simp only [piercedE, hchi, hahead]

/-! ## Small perturbations: the sign of a quadratic near 0 -/

def poly (c0 c1 c2 δ : Rat) : Rat := c0 + c1 * δ + c2 * (δ * δ)

def LexPos (c0 c1 c2 : Rat) : Prop := 0 < c0 ∨ (c0 = 0 ∧ (0 < c1 ∨ (c1 = 0 ∧ 0 < c2)))

def bnd (c0 c1 c2 : Rat) : Rat :=
  if c0 ≠ 0 then qmin 1 (qabs c0 / (2 * (qabs c1 + qabs c2 + 1)))
  else if c1 ≠ 0 then qmin 1 (qabs c1 / (2 * (qabs c2 + 1))) else 1

theorem qabs_pos {a : Rat} (h : a ≠ 0) : 0 < qabs a := by
  unfold qabs; split <;> grind

theorem bnd_pos (c0 c1 c2 : Rat) : 0 < bnd c0 c1 c2 := by
  unfold bnd
  split
  · rename_i h
    have := qabs_pos h; have := qabs_nonneg c1; have := qabs_nonneg c2
    unfold qmin; split
    · decide
    · exact div_pos' (qabs_pos h) (by grind)
  · split
    · rename_i _ h
      have := qabs_nonneg c2
      unfold qmin; split
      · decide
      · exact div_pos' (qabs_pos h) (by grind)
    · decide

/-- `|b δ| ≤ |b| δ` for `δ ≥ 0`, as two inequalities. -/
theorem abs_mul_le (b δ : Rat) (hδ : 0 ≤ δ) : b * δ ≤ qabs b * δ ∧ -(qabs b * δ) ≤ b * δ := by
  have h1 := Rat.mul_le_mul_of_nonneg_left (le_qabs b) hδ
  have h2 := Rat.mul_le_mul_of_nonneg_left (neg_le_qabs b) hδ
  constructor
  · rw [Rat.mul_comm b, Rat.mul_comm (qabs b)]; exact h1
  · have : δ * -b = -(b * δ) := by rw [Rat.mul_neg, Rat.mul_comm]
    rw [this] at h2; rw [Rat.mul_comm (qabs b)]; grind

theorem le_of_le_div {d x y : Rat} (hd : 0 < d) (h : x ≤ y / d) : x * d ≤ y := by
  have := Rat.mul_le_mul_of_nonneg_left h (Rat.le_of_lt hd)
  rw [Rat.mul_comm d, Rat.mul_comm d (y / d), Rat.div_mul_cancel (by grind)] at this
  exact this

/-- Below the bound, a quadratic's value has the sign of its lowest nonzero coefficient. -/
theorem small_pos (c0 c1 c2 δ : Rat) (h0 : 0 < δ) (hb : δ ≤ bnd c0 c1 c2) :
    0 < poly c0 c1 c2 δ ↔ LexPos c0 c1 c2 := by
  unfold bnd at hb
  unfold poly LexPos
  have hdd : 0 ≤ δ * δ := Rat.le_of_lt (Rat.mul_pos h0 h0)
  split at hb
  · rename_i hc0
    have hq1 : δ ≤ 1 := Rat.le_trans hb (qmin_le_left _ _)
    have hq2 : δ ≤ qabs c0 / (2 * (qabs c1 + qabs c2 + 1)) := Rat.le_trans hb (qmin_le_right _ _)
    have hK : 0 < 2 * (qabs c1 + qabs c2 + 1) := by
      have := qabs_nonneg c1; have := qabs_nonneg c2; grind
    have hm := le_of_le_div hK hq2
    have a1 := abs_mul_le c1 δ (Rat.le_of_lt h0)
    have a2 := abs_mul_le c2 (δ * δ) hdd
    have hsq : δ * δ ≤ δ := by have := Rat.mul_le_mul_of_nonneg_left hq1 (Rat.le_of_lt h0); grind
    have b2 := Rat.mul_le_mul_of_nonneg_left hsq (qabs_nonneg c2)
    have := qabs_nonneg c1; have := qabs_nonneg c2
    have ha := qabs_pos hc0
    rcases ForgeExact.RefFacts.qabs_eq_or c0 with e | e <;> rw [e] at hm ha <;> constructor <;> intro h <;> grind
  · rename_i hc0
    have hz : c0 = 0 := Classical.not_not.mp hc0
    subst hz
    split at hb
    · rename_i hc1
      have hq1 : δ ≤ 1 := Rat.le_trans hb (qmin_le_left _ _)
      have hq2 : δ ≤ qabs c1 / (2 * (qabs c2 + 1)) := Rat.le_trans hb (qmin_le_right _ _)
      have hK : 0 < 2 * (qabs c2 + 1) := by have := qabs_nonneg c2; grind
      have hm := le_of_le_div hK hq2
      have a2 := abs_mul_le c2 δ (Rat.le_of_lt h0)
      have := qabs_nonneg c2
      have ha := qabs_pos hc1
      have e : 0 + c1 * δ + c2 * (δ * δ) = δ * (c1 + c2 * δ) := by grind
      rw [e, mul_pos_iff_left h0]
      rcases ForgeExact.RefFacts.qabs_eq_or c1 with e | e <;> rw [e] at hm ha <;> constructor <;>
        intro h <;> grind
    · rename_i hc1
      have hz : c1 = 0 := Classical.not_not.mp hc1
      subst hz
      have e : 0 + 0 * δ + c2 * (δ * δ) = (δ * δ) * c2 := by grind
      rw [e, mul_pos_iff_left (Rat.mul_pos h0 h0)]
      grind
theorem qabs_neg' (a : Rat) : qabs (-a) = qabs a := by unfold qabs; split <;> split <;> grind

theorem bnd_neg (c0 c1 c2 : Rat) : bnd (-c0) (-c1) (-c2) = bnd c0 c1 c2 := by
  have e0 : (-c0 = 0) ↔ c0 = 0 := by grind
  have e1 : (-c1 = 0) ↔ c1 = 0 := by grind
  unfold bnd
  simp only [qabs_neg', ne_eq, e0, e1]

theorem small_neg (c0 c1 c2 δ : Rat) (h0 : 0 < δ) (hb : δ ≤ bnd c0 c1 c2) :
    poly c0 c1 c2 δ < 0 ↔ LexPos (-c0) (-c1) (-c2) := by
  rw [← small_pos _ _ _ δ h0 (by rw [bnd_neg]; exact hb)]
  unfold poly; grind

theorem small_ne (c0 c1 c2 δ : Rat) (h0 : 0 < δ) (hb : δ ≤ bnd c0 c1 c2)
    (hne : c0 ≠ 0 ∨ c1 ≠ 0 ∨ c2 ≠ 0) : poly c0 c1 c2 δ ≠ 0 := by
  have hp := small_pos c0 c1 c2 δ h0 hb
  have hn := small_neg c0 c1 c2 δ h0 hb
  unfold LexPos at hp hn
  intro h0'
  rw [h0'] at hp hn
  grind

/-- The point the reference's simulation of simplicity stands for, at a real `δ`. -/
def qd (q : V3) (δ : Rat) : V3 := ⟨q.x, q.y + δ, q.z + δ * δ⟩

/-- The least of the bounds of a list of quadratics (and 1). -/
def bndAll (l : List (Rat × Rat × Rat)) : Rat := l.foldr (fun t m => qmin (bnd t.1 t.2.1 t.2.2) m) 1

theorem bndAll_pos : ∀ l : List (Rat × Rat × Rat), 0 < bndAll l
  | [] => by unfold bndAll; decide
  | t :: l => by
    unfold bndAll; simp only [List.foldr_cons]
    have := bnd_pos t.1 t.2.1 t.2.2
    have := bndAll_pos l
    unfold bndAll at this
    unfold qmin; split <;> assumption

theorem bndAll_le : ∀ (l : List (Rat × Rat × Rat)), ∀ t ∈ l, bndAll l ≤ bnd t.1 t.2.1 t.2.2
  | [], _, h => nomatch h
  | u :: l, t, h => by
    unfold bndAll; simp only [List.foldr_cons]
    rcases List.mem_cons.mp h with rfl | h
    · exact qmin_le_left _ _
    · exact Rat.le_trans (qmin_le_right _ _) (bndAll_le l t h)

theorem z_perturb (q a : V3) (δ : Rat) (h0 : 0 < δ) (hb : δ ≤ bnd (a.z - q.z) 0 (-1)) :
    decide (a.z > (qd q δ).z) = decide (a.z > q.z) ∧ a.z ≠ (qd q δ).z := by
  have e : a.z - (qd q δ).z = poly (a.z - q.z) 0 (-1) δ := by simp only [qd, poly]; grind
  have hp := small_pos _ _ _ δ h0 hb
  have hne := small_ne _ _ _ δ h0 hb (Or.inr (Or.inr (by decide)))
  rw [← e] at hp hne
  unfold LexPos at hp
  constructor
  · apply decide_eq_decide.mpr; grind
  · grind

theorem sgn_pos (x : Rat) (h : 0 < x) : sgn x = 1 := by unfold sgn; simp [h]
theorem sgn_neg (x : Rat) (h : x < 0) : sgn x = -1 := by
  unfold sgn; have : ¬x > 0 := by grind
  simp [this, h]

theorem mul_pos_iff' (a b : Rat) (ha : a ≠ 0) (hb : b ≠ 0) :
    0 < a * b ↔ (0 < a ∧ 0 < b) ∨ (a < 0 ∧ b < 0) := by
  rcases ne_cases ha with ha | ha <;> rcases ne_cases hb with hb | hb
  · rw [mul_pos_iff_neg ha]; grind
  · rw [mul_pos_iff_neg ha]; grind
  · rw [mul_pos_iff_left ha]; grind
  · rw [mul_pos_iff_left ha]; grind

/-- The quadratics a point's perturbation must stay below, for one edge. -/
def edgePolys (q a b : V3) : List (Rat × Rat × Rat) :=
  [(a.z - q.z, 0, -1), (b.z - q.z, 0, -1), (d0 q a b, -(b.z - a.z), b.y - a.y)]

theorem d0_qd (q a b : V3) (δ : Rat) :
    d0 (qd q δ) a b = poly (d0 q a b) (-(b.z - a.z)) (b.y - a.y) δ := by
  simp only [d0, qd, poly]; grind

/-- **The reference's tie-break is the exact crossing at the perturbed point.** -/
theorem crosses_eq (q a b : V3) (δ : Rat) (h0 : 0 < δ) (hb : ∀ t ∈ edgePolys q a b, δ ≤ bnd t.1 t.2.1 t.2.2) :
    crossesYZ q a b = crossE (qd q δ) a b := by
  have za := z_perturb q a δ h0 (hb (a.z - q.z, 0, -1) (by simp [edgePolys]))
  have zb := z_perturb q b δ h0 (hb (b.z - q.z, 0, -1) (by simp [edgePolys]))
  have hd := hb (d0 q a b, -(b.z - a.z), b.y - a.y) (by simp [edgePolys])
  unfold crossesYZ crossE
  rw [za.1, zb.1]
  by_cases hs : decide (a.z > q.z) = decide (b.z > q.z)
  · simp [hs]
  · have hdz : b.z - a.z ≠ 0 := by
      intro h; apply hs; apply decide_eq_decide.mpr; grind
    have hne : decide (a.z > q.z) != decide (b.z > q.z) := by simp [hs]
    simp only [hs, hne, beq_iff_eq, Bool.true_and, ite_false]
    have hp := small_pos _ _ _ δ h0 hd
    have hn := small_neg _ _ _ δ h0 hd
    have hnz := small_ne _ _ _ δ h0 hd (Or.inr (Or.inl (by grind)))
    rw [← d0_qd] at hp hn hnz
    unfold LexPos at hp hn
    simp only at hp hn
    apply Bool.eq_iff_iff.mpr
    simp only [decide_eq_true_eq]
    rw [mul_pos_iff' _ _ hnz hdz]
    show (if (d0 q a b != 0) = true then sgn (d0 q a b) else sgn (-(b.z - a.z))) * sgn (b.z - a.z) > 0 ↔ _
    generalize d0 (qd q δ) a b = Dq at hp hn hnz ⊢
    generalize d0 q a b = D at hp hn ⊢
    by_cases h1 : D = 0
    · subst h1
      simp only [bne_self_eq_false, Bool.false_eq_true, ite_false]
      rcases ne_cases hdz with hz | hz
      · rw [sgn_pos (-(b.z - a.z)) (by grind), sgn_neg _ hz]; grind
      · rw [sgn_neg (-(b.z - a.z)) (by grind), sgn_pos _ hz]; grind
    · have : (D != 0) = true := by simp [h1]
      simp only [this, ite_true]
      rcases ne_cases h1 with h2 | h2 <;> rcases ne_cases hdz with hz | hz
      · rw [sgn_neg _ h2, sgn_neg _ hz]; grind
      · rw [sgn_neg _ h2, sgn_pos _ hz]; grind
      · rw [sgn_pos _ h2, sgn_neg _ hz]; grind
      · rw [sgn_pos _ h2, sgn_pos _ hz]; grind

/-! ## The reference's code, as folds -/

def loopCount (p : V3) (lp : Array V3) : Nat :=
  ((List.range lp.size).filter fun i => crossesYZ p lp[i]! lp[(i + 1) % lp.size]!).length

theorem fold_count {α : Type} (g : α → Bool) : ∀ (l : List α) (acc : Nat),
    List.foldl (fun b a => if g a then b + 1 else b) acc l = acc + (l.filter g).length
  | [], acc => by simp
  | x :: xs, acc => by
    simp only [List.foldl_cons, List.filter_cons]
    rw [fold_count g xs]
    cases g x <;> simp <;> omega

theorem loop_eq (p : V3) (lp : Array V3) (acc : Nat) :
    (Id.run do
      let mut k := acc
      for i in [0:lp.size] do
        if crossesYZ p lp[i]! lp[(i + 1) % lp.size]! then k := k + 1
      return k) = acc + loopCount p lp := by
  simp only [Id.run, Std.Legacy.Range.forIn_eq_forIn_range']
  have : ∀ (i : Nat) (k : Nat), (if crossesYZ p lp[i]! lp[(i + 1) % lp.size]! = true then
      (pure (ForInStep.yield (k + 1)) : Id (ForInStep Nat)) else pure (ForInStep.yield k)) =
      pure (ForInStep.yield (if crossesYZ p lp[i]! lp[(i + 1) % lp.size]! then k + 1 else k)) := by
    intro i k; split <;> rfl
  simp only [this, List.forIn_pure_yield_eq_foldl, bind_pure]
  rw [show [:lp.size].size = lp.size by simp [Std.Legacy.Range.size], ← List.range_eq_range']
  show List.foldl _ acc _ = _
  rw [fold_count]
  rfl

theorem par_len {α : Type} (g : α → Bool) : ∀ l : List α, ((l.filter g).length % 2 == 1) = xorL (l.map g)
  | [] => rfl
  | x :: l => by
    simp only [List.filter_cons, List.map_cons, xorL_cons, ← par_len g l]
    cases g x
    · simp
    · simp only [ite_true, List.length_cons, Bool.true_xor]
      rcases Nat.mod_two_eq_zero_or_one (List.filter g l).length with h | h <;> simp [h, Nat.add_mod]

theorem par_sum {α : Type} (c : α → Nat) : ∀ (l : List α) (a : Nat),
    ((List.foldl (fun acc x => acc + c x) a l) % 2 == 1) =
      ((a % 2 == 1) ^^ xorL (l.map fun x => c x % 2 == 1))
  | [], a => by simp
  | x :: l, a => by
    simp only [List.foldl_cons, List.map_cons, xorL_cons, par_sum c l]
    cases h1 : (a % 2 == 1) <;> cases h2 : (c x % 2 == 1) <;> cases xorL (l.map fun x => c x % 2 == 1) <;>
      simp_all <;> omega

theorem loopCount_eq (p : V3) (lp : Array V3) :
    loopCount p lp = ((edgesA lp).filter fun e => crossesYZ p e.1 e.2).length := by
  unfold loopCount edgesA
  rw [List.filter_map, List.length_map]
  rfl

/-- The reference's count over a face's loops, as the parity of its edges' crossings. -/
theorem cnt_par (p : V3) (f : Face) :
    (f.loops.foldl (fun acc lp => acc + loopCount p lp) 0 % 2 == 1) =
      xorL ((edgesF f).map fun e => crossesYZ p e.1 e.2) := by
  rw [← Array.foldl_toList, par_sum]
  rw [show ((0 : Nat) % 2 == 1) = false from rfl, Bool.false_xor]
  unfold edgesF
  rw [xorL_flatMap]
  apply xorL_congr
  intro lp _
  rw [loopCount_eq, par_len]

/-! ## Perturbed: the reference is the exact count at a point in general position -/

/-- Every quadratic a face's tests at `q` give. -/
def facePolys (f : Face) (q : V3) : List (Rat × Rat × Rat) :=
  (f.plane.c - f.plane.n.dot q, -f.plane.n.y, -f.plane.n.z) ::
    (edgesF f).flatMap fun e => edgePolys q e.1 e.2

def solidPolys (S : Solid) (q : V3) : List (Rat × Rat × Rat) := (facesL S).flatMap fun f => facePolys f q

theorem plane_qd (P : Plane) (q : V3) (δ : Rat) :
    P.c - P.n.dot (qd q δ) = poly (P.c - P.n.dot q) (-P.n.y) (-P.n.z) δ := by
  simp only [V3.dot, qd, poly]; grind

theorem plane_perturb (P : Plane) (q : V3) (δ : Rat) (h0 : 0 < δ)
    (hb : δ ≤ bnd (P.c - P.n.dot q) (-P.n.y) (-P.n.z)) (hoff : P.eval q ≠ 0) :
    0 < (P.c - P.n.dot (qd q δ)) * (P.c - P.n.dot q) := by
  have hc0 : P.c - P.n.dot q ≠ 0 := by simp only [Plane.eval] at hoff; grind
  have hp := small_pos _ _ _ δ h0 hb
  have hn := small_neg _ _ _ δ h0 hb
  rw [← plane_qd] at hp hn
  unfold LexPos at hp hn
  rw [mul_pos_iff' _ _ (by intro h; rw [h] at hp hn; grind) hc0]
  rcases ne_cases hc0 with h | h <;> grind

theorem sgn_mul_pos (a b : Rat) (ha : a ≠ 0) (hb : b ≠ 0) : sgn a * sgn b > 0 ↔ 0 < a * b := by
  rw [mul_pos_iff' _ _ ha hb]
  rcases ne_cases ha with h1 | h1 <;> rcases ne_cases hb with h2 | h2
  · rw [sgn_neg _ h1, sgn_neg _ h2]; grind
  · rw [sgn_neg _ h1, sgn_pos _ h2]; grind
  · rw [sgn_pos _ h1, sgn_neg _ h2]; grind
  · rw [sgn_pos _ h1, sgn_pos _ h2]; grind

theorem chain_pos {x a y b : Rat} (h1 : 0 < x * a) (h2 : 0 < y * b) (h3 : 0 < a * b) : 0 < x * y := by
  have := Rat.mul_pos h1 h2
  rw [show x * a * (y * b) = (x * y) * (a * b) by grind] at this
  exact pos_of_mul h3 this

/-- **The reference pierces a face iff the exact ray from the perturbed point does.** -/
theorem pierced_eq (f : Face) (q : V3) (δ : Rat) (h0 : 0 < δ)
    (hb : ∀ t ∈ facePolys f q, δ ≤ bnd t.1 t.2.1 t.2.2) (hoff : f.plane.eval q ≠ 0) :
    f.pierced q = piercedE f (qd q δ) := by
  have hpl := plane_perturb f.plane q δ h0
    (hb (f.plane.c - f.plane.n.dot q, -f.plane.n.y, -f.plane.n.z) (List.mem_cons_self ..)) hoff
  have hc0 : f.plane.c - f.plane.n.dot q ≠ 0 := by simp only [Plane.eval] at hoff; grind
  have hchi : chiE f (qd q δ) = xorL ((edgesF f).map fun e => crossesYZ q e.1 e.2) := by
    unfold chiE
    apply xorL_congr
    intro e he
    exact (crosses_eq q e.1 e.2 δ h0 fun t ht => hb t (List.mem_cons_of_mem _
      (List.mem_flatMap.mpr ⟨e, he, ht⟩))).symm
  unfold Face.pierced piercedE aheadB
  by_cases hx : f.plane.n.x = 0
  · simp [hx]
  · have hx' : (f.plane.n.x == 0) = false := by simp [hx]
    simp only [hx', Bool.false_eq_true, ite_false]
    have hq : f.plane.c - f.plane.n.dot (qd q δ) ≠ 0 := by intro h; rw [h] at hpl; grind
    have ha : (0 < (f.plane.c - f.plane.n.dot (qd q δ)) * f.plane.n.x) ↔
        (sgn (f.plane.c - f.plane.n.dot q) * sgn f.plane.n.x > 0) := by
      rw [sgn_mul_pos _ _ hc0 hx]
      have e : ((f.plane.c - f.plane.n.dot (qd q δ)) * (f.plane.c - f.plane.n.dot q)) *
          ((f.plane.c - f.plane.n.dot (qd q δ)) * f.plane.n.x) =
          ((f.plane.c - f.plane.n.dot (qd q δ)) * (f.plane.c - f.plane.n.dot (qd q δ))) *
          ((f.plane.c - f.plane.n.dot q) * f.plane.n.x) := by grind
      rw [← mul_pos_iff_left hpl (x := (f.plane.c - f.plane.n.dot (qd q δ)) * f.plane.n.x), e,
        mul_pos_iff_left (sq_pos hq)]
    have hfun : (fun (acc : Nat) (lp : Array V3) => Id.run do
        let mut k := acc
        for i in [0:lp.size] do
          if crossesYZ q lp[i]! lp[(i + 1) % lp.size]! then k := k + 1
        return k) = (fun acc lp => acc + loopCount q lp) := by
      funext acc lp; exact loop_eq q lp acc
    rw [hfun, cnt_par, ← hchi]
    by_cases hs : sgn (f.plane.c - f.plane.n.dot q) * sgn f.plane.n.x ≤ 0
    · have : ¬(0 < (f.plane.c - f.plane.n.dot (qd q δ)) * f.plane.n.x) := by rw [ha]; grind
      simp [hs, this]
    · have : 0 < (f.plane.c - f.plane.n.dot (qd q δ)) * f.plane.n.x := by rw [ha]; grind
      simp [hs, this]

theorem contains_eq (S : Solid) (q : V3) (δ : Rat) (h0 : 0 < δ)
    (hb : ∀ t ∈ solidPolys S q, δ ≤ bnd t.1 t.2.1 t.2.2) (hoff : ∀ f ∈ facesL S, f.plane.eval q ≠ 0) :
    S.contains q = containsE S (qd q δ) := by
  unfold Solid.contains containsE
  rw [← Array.foldl_toList, fold_count, Nat.zero_add, par_len]
  apply xorL_congr
  intro f hf
  exact pierced_eq f q δ h0 (fun t ht => hb t (List.mem_flatMap.mpr ⟨f, hf, ht⟩)) (hoff f hf)

/-- The quadratics a move's general position needs: no vertex on the line of the move. -/
def pairPolys (S : Solid) (p q : V3) : List (Rat × Rat × Rat) :=
  (allEdges S).flatMap fun e =>
    [(Fs p q e.1, -(q.z - p.z), q.y - p.y), (Fs p q e.2, -(q.z - p.z), q.y - p.y)]

theorem Fs_qd (p q x : V3) (δ : Rat) :
    Fs (qd p δ) (qd q δ) x = poly (Fs p q x) (-(q.z - p.z)) (q.y - p.y) δ := by
  simp only [Fs, qd, poly]; grind

theorem Fs_swap (w w' x : V3) : Fs w' w x = -Fs w w' x := by simp only [Fs]; grind

theorem mem_solidPolys {S : Solid} {q : V3} {f : Face} (hf : f ∈ facesL S) {e : V3 × V3}
    (he : e ∈ edgesF f) {t : Rat × Rat × Rat} (ht : t ∈ edgePolys q e.1 e.2) : t ∈ solidPolys S q :=
  List.mem_flatMap.mpr ⟨f, hf, List.mem_cons_of_mem _ (List.mem_flatMap.mpr ⟨e, he, ht⟩)⟩

theorem d0_ne (q a b : V3) (δ : Rat) (h0 : 0 < δ) (hb : δ ≤ bnd (d0 q a b) (-(b.z - a.z)) (b.y - a.y))
    (hs : (a.z - (qd q δ).z) * (b.z - (qd q δ).z) < 0) : d0 (qd q δ) a b ≠ 0 := by
  rw [d0_qd]
  apply small_ne _ _ _ δ h0 hb
  right; left
  intro h
  have : b.z = a.z := by grind
  rw [this] at hs
  have := mul_self_nonneg' (a.z - (qd q δ).z)
  grind

theorem trich (a b : Rat) : a < b ∨ a = b ∨ b < a := by
  by_cases h1 : a < b
  · exact Or.inl h1
  · by_cases h2 : a = b
    · exact Or.inr (Or.inl h2)
    · exact Or.inr (Or.inr (by grind))

/-- **Jordan–Brouwer for the reference.** For a closed B-rep whose faces lie in their planes, the
reference's ray-parity membership is the same at any two points strictly on the same side of every
face plane. -/
theorem contains_const (S : Solid) (hflat : FacesFlat S) (hcl : Closed S) (p q : V3)
    (hside : ∀ f ∈ facesL S, 0 < f.plane.eval p * f.plane.eval q) :
    S.contains p = S.contains q := by
  obtain ⟨δ, h0, hbp, hbq, hbpq⟩ : ∃ δ : Rat, 0 < δ ∧
      (∀ t ∈ solidPolys S p, δ ≤ bnd t.1 t.2.1 t.2.2) ∧
      (∀ t ∈ solidPolys S q, δ ≤ bnd t.1 t.2.1 t.2.2) ∧
      (∀ t ∈ pairPolys S p q, δ ≤ bnd t.1 t.2.1 t.2.2) :=
    ⟨bndAll (solidPolys S p ++ solidPolys S q ++ pairPolys S p q), bndAll_pos _,
      fun t ht => bndAll_le _ t (List.mem_append_left _ (List.mem_append_left _ ht)),
      fun t ht => bndAll_le _ t (List.mem_append_left _ (List.mem_append_right _ ht)),
      fun t ht => bndAll_le _ t (List.mem_append_right _ ht)⟩
  have hoffp : ∀ f ∈ facesL S, f.plane.eval p ≠ 0 := fun f hf h => by
    have := hside f hf; rw [h] at this; grind
  have hoffq : ∀ f ∈ facesL S, f.plane.eval q ≠ 0 := fun f hf h => by
    have := hside f hf; rw [h] at this; grind
  rw [contains_eq S p δ h0 hbp hoffp, contains_eq S q δ h0 hbq hoffq]
  -- the perturbed points are on the same sides
  have hside' : ∀ f ∈ facesL S, 0 < f.plane.eval (qd p δ) * f.plane.eval (qd q δ) := by
    intro f hf
    have hp := plane_perturb f.plane p δ h0
      (hbp (f.plane.c - f.plane.n.dot p, -f.plane.n.y, -f.plane.n.z)
        (List.mem_flatMap.mpr ⟨f, hf, List.mem_cons_self ..⟩)) (hoffp f hf)
    have hq := plane_perturb f.plane q δ h0
      (hbq (f.plane.c - f.plane.n.dot q, -f.plane.n.y, -f.plane.n.z)
        (List.mem_flatMap.mpr ⟨f, hf, List.mem_cons_self ..⟩)) (hoffq f hf)
    have e1 : ∀ x : V3, f.plane.eval x = -(f.plane.c - f.plane.n.dot x) := by
      intro x; simp only [Plane.eval]; grind
    have := hside f hf
    rw [e1, e1] at this ⊢
    rw [show -(f.plane.c - f.plane.n.dot (qd p δ)) * -(f.plane.c - f.plane.n.dot (qd q δ)) =
      (f.plane.c - f.plane.n.dot (qd p δ)) * (f.plane.c - f.plane.n.dot (qd q δ)) by grind]
    rw [show -(f.plane.c - f.plane.n.dot p) * -(f.plane.c - f.plane.n.dot q) =
      (f.plane.c - f.plane.n.dot p) * (f.plane.c - f.plane.n.dot q) by grind] at this
    exact chain_pos hp hq this
  -- the facts general position needs, at either end
  have zP : ∀ f ∈ facesL S, ∀ e ∈ edgesF f,
      e.1.z ≠ (qd p δ).z ∧ e.2.z ≠ (qd p δ).z ∧ e.1.z ≠ (qd q δ).z ∧ e.2.z ≠ (qd q δ).z := by
    intro f hf e he
    exact ⟨(z_perturb p e.1 δ h0 (hbp (e.1.z - p.z, 0, -1) (mem_solidPolys hf he (by simp [edgePolys])))).2,
      (z_perturb p e.2 δ h0 (hbp (e.2.z - p.z, 0, -1) (mem_solidPolys hf he (by simp [edgePolys])))).2,
      (z_perturb q e.1 δ h0 (hbq (e.1.z - q.z, 0, -1) (mem_solidPolys hf he (by simp [edgePolys])))).2,
      (z_perturb q e.2 δ h0 (hbq (e.2.z - q.z, 0, -1) (mem_solidPolys hf he (by simp [edgePolys])))).2⟩
  have dP : ∀ f ∈ facesL S, ∀ e ∈ edgesF f,
      ((e.1.z - (qd p δ).z) * (e.2.z - (qd p δ).z) < 0 → d0 (qd p δ) e.1 e.2 ≠ 0) ∧
      ((e.1.z - (qd q δ).z) * (e.2.z - (qd q δ).z) < 0 → d0 (qd q δ) e.1 e.2 ≠ 0) := by
    intro f hf e he
    exact ⟨d0_ne p e.1 e.2 δ h0 (hbp (d0 p e.1 e.2, -(e.2.z - e.1.z), e.2.y - e.1.y)
        (mem_solidPolys hf he (by simp [edgePolys]))),
      d0_ne q e.1 e.2 δ h0 (hbq (d0 q e.1 e.2, -(e.2.z - e.1.z), e.2.y - e.1.y)
        (mem_solidPolys hf he (by simp [edgePolys])))⟩
  have fP : q.z ≠ p.z → ∀ f ∈ facesL S, ∀ e ∈ edgesF f,
      Fs (qd p δ) (qd q δ) e.1 ≠ 0 ∧ Fs (qd p δ) (qd q δ) e.2 ≠ 0 := by
    intro hz f hf e he
    have hm : e ∈ allEdges S := List.mem_flatMap.mpr ⟨f, hf, he⟩
    constructor
    · rw [Fs_qd]
      exact small_ne _ _ _ δ h0 (hbpq (Fs p q e.1, -(q.z - p.z), q.y - p.y)
        (List.mem_flatMap.mpr ⟨e, hm, by simp⟩)) (Or.inr (Or.inl (by grind)))
    · rw [Fs_qd]
      exact small_ne _ _ _ δ h0 (hbpq (Fs p q e.2, -(q.z - p.z), q.y - p.y)
        (List.mem_flatMap.mpr ⟨e, hm, by simp⟩)) (Or.inr (Or.inl (by grind)))
  have hzz : (qd p δ).z < (qd q δ).z ↔ p.z < q.z := by simp only [qd]; grind
  have hyy : (qd p δ).y < (qd q δ).y ↔ p.y < q.y := by simp only [qd]; grind
  rcases trich p.z q.z with hz | hz | hz
  · apply move_rise S hflat hcl _ _ (hzz.mpr hz) hside'
    intro f hf e he
    obtain ⟨a1, b1, a2, b2⟩ := zP f hf e he
    exact ⟨a1, b1, a2, b2, (fP (by grind) f hf e he).1, (fP (by grind) f hf e he).2,
      (dP f hf e he).1, (dP f hf e he).2⟩
  · rcases trich p.y q.y with hy | hy | hy
    · apply move_level S hflat hcl _ _ (by simp only [qd]; grind) (hyy.mpr hy) hside'
      intro f hf e he
      obtain ⟨a1, b1, _, _⟩ := zP f hf e he
      refine ⟨a1, b1, fun hs => (dP f hf e he).2 ?_⟩
      have : (qd q δ).z = (qd p δ).z := by simp only [qd]; grind
      rw [this]; exact hs
    · exact move_x S _ _ (by simp only [qd]; grind) (by simp only [qd]; grind) hside'
    · symm
      apply move_level S hflat hcl _ _ (by simp only [qd]; grind) (by simp only [qd]; grind)
        (fun f hf => by rw [Rat.mul_comm]; exact hside' f hf)
      intro f hf e he
      obtain ⟨_, _, a2, b2⟩ := zP f hf e he
      refine ⟨a2, b2, fun hs => (dP f hf e he).1 ?_⟩
      have : (qd p δ).z = (qd q δ).z := by simp only [qd]; grind
      rw [this]; exact hs
  · symm
    apply move_rise S hflat hcl _ _ (by simp only [qd]; grind)
      (fun f hf => by rw [Rat.mul_comm]; exact hside' f hf)
    intro f hf e he
    obtain ⟨a1, b1, a2, b2⟩ := zP f hf e he
    have h1 := fP (by grind) f hf e he
    exact ⟨a2, b2, a1, b1, by rw [Fs_swap]; grind, by rw [Fs_swap]; grind,
      (dP f hf e he).2, (dP f hf e he).1⟩

end ForgeExact.Jordan
