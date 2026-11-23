/-!
# Signs of rational products, and parity

The few facts about the signs of products the certificate's sign arguments use (`grind` decides
the linear ones itself), and `par`, the parity of a list of booleans.
-/

namespace ForgeCertify

theorem lt_trichotomy' (a b : Rat) : a < b ∨ a = b ∨ b < a := by grind

theorem mul_pos_of_neg_of_neg {a b : Rat} (ha : a < 0) (hb : b < 0) : 0 < a * b := by
  have := Rat.mul_pos (a := -a) (b := -b) (by grind) (by grind)
  have e : -a * -b = a * b := by grind
  grind

theorem mul_neg_of_pos_of_neg {a b : Rat} (ha : 0 < a) (hb : b < 0) : a * b < 0 := by
  have := Rat.mul_pos (a := a) (b := -b) ha (by grind)
  have e : a * -b = -(a * b) := by grind
  grind

theorem mul_neg_of_neg_of_pos {a b : Rat} (ha : a < 0) (hb : 0 < b) : a * b < 0 := by
  have := mul_neg_of_pos_of_neg hb ha
  rw [Rat.mul_comm]; exact this

theorem mul_ne_zero' {a b : Rat} (ha : a ≠ 0) (hb : b ≠ 0) : a * b ≠ 0 := by
  intro h
  rcases Rat.mul_eq_zero.mp h with h | h <;> contradiction

/-- `0 < a b` iff `a` and `b` have one strict sign. -/
theorem mul_pos_iff' (a b : Rat) : 0 < a * b ↔ (0 < a ∧ 0 < b) ∨ (a < 0 ∧ b < 0) := by
  constructor
  · intro h
    rcases lt_trichotomy' a 0 with ha | ha | ha <;> rcases lt_trichotomy' b 0 with hb | hb | hb
    · exact Or.inr ⟨ha, hb⟩
    · subst hb; simp at h
    · have := mul_neg_of_neg_of_pos ha hb; grind
    · subst ha; simp at h
    · subst ha; simp at h
    · subst ha; simp at h
    · have := mul_neg_of_pos_of_neg ha hb; grind
    · subst hb; simp at h
    · exact Or.inl ⟨ha, hb⟩
  · rintro (⟨ha, hb⟩ | ⟨ha, hb⟩)
    · exact Rat.mul_pos ha hb
    · exact mul_pos_of_neg_of_neg ha hb

theorem mul_neg_iff' (a b : Rat) : a * b < 0 ↔ (0 < a ∧ b < 0) ∨ (a < 0 ∧ 0 < b) := by
  have h := mul_pos_iff' a (-b)
  have e : a * -b = -(a * b) := by grind
  rw [e] at h
  constructor
  · intro hab
    rcases h.mp (by grind) with ⟨h1, h2⟩ | ⟨h1, h2⟩
    · exact Or.inl ⟨h1, by grind⟩
    · exact Or.inr ⟨h1, by grind⟩
  · intro hab
    have := h.mpr (by rcases hab with ⟨h1, h2⟩ | ⟨h1, h2⟩
                      · exact Or.inl ⟨h1, by grind⟩
                      · exact Or.inr ⟨h1, by grind⟩)
    grind

/-- Two numbers with a positive product have one sign. -/
theorem same_sign {a b : Rat} (h : 0 < a * b) : (0 < a ↔ 0 < b) := by
  rcases (mul_pos_iff' a b).mp h with ⟨h1, h2⟩ | ⟨h1, h2⟩ <;> grind

theorem sq_pos' {d : Rat} (hd : d ≠ 0) : 0 < d * d := by
  rcases lt_trichotomy' d 0 with h | h | h
  · exact mul_pos_of_neg_of_neg h h
  · exact absurd h hd
  · exact Rat.mul_pos h h

/-- `0 < d² X` iff `0 < X`, for `d ≠ 0`. -/
theorem sq_mul_pos_iff {d : Rat} (hd : d ≠ 0) (X : Rat) : 0 < d * d * X ↔ 0 < X := by
  have := sq_pos' hd
  rw [mul_pos_iff']; grind

/-- `0 < (d X) d` iff `0 < X`. -/
theorem mul_sq_pos_iff {d : Rat} (hd : d ≠ 0) (X : Rat) : 0 < d * X * d ↔ 0 < X := by
  rw [show d * X * d = d * d * X by grind]; exact sq_mul_pos_iff hd X

theorem convex_lt {x y m μ : Rat} (h0 : 0 < μ) (h1 : μ < 1) (hx : x < m) (hy : y < m) :
    x + μ * (y - x) < m := by
  have a := Rat.mul_lt_mul_of_pos_left hy h0
  have b := Rat.mul_lt_mul_of_pos_left hx (show 0 < 1 - μ by grind)
  grind

theorem convex_gt {x y m μ : Rat} (h0 : 0 < μ) (h1 : μ < 1) (hx : m < x) (hy : m < y) :
    m < x + μ * (y - x) := by
  have a := Rat.mul_lt_mul_of_pos_left hy h0
  have b := Rat.mul_lt_mul_of_pos_left hx (show 0 < 1 - μ by grind)
  grind

theorem convex_le {x y m μ : Rat} (h0 : 0 ≤ μ) (h1 : μ ≤ 1) (hx : x ≤ m) (hy : y ≤ m) :
    x + μ * (y - x) ≤ m := by
  have a := Rat.mul_le_mul_of_nonneg_left hy h0
  have b := Rat.mul_le_mul_of_nonneg_left hx (show 0 ≤ 1 - μ by grind)
  grind

/-! ## Parity -/

/-- The parity of a list of booleans: odd number of `true`s. -/
def par (l : List Bool) : Bool := l.foldr (· ^^ ·) false

@[simp] theorem par_nil : par [] = false := rfl
@[simp] theorem par_cons (b : Bool) (l : List Bool) : par (b :: l) = (b ^^ par l) := rfl

theorem par_append (l₁ l₂ : List Bool) : par (l₁ ++ l₂) = (par l₁ ^^ par l₂) := by
  induction l₁ with
  | nil => simp
  | cons b l ih => simp [ih]

theorem par_flatten (ls : List (List Bool)) : par ls.flatten = par (ls.map par) := by
  induction ls with
  | nil => simp
  | cons l ls ih => simp [par_append, ih]

theorem par_all_false (l : List Bool) (h : ∀ b ∈ l, b = false) : par l = false := by
  induction l with
  | nil => rfl
  | cons b l ih =>
    simp only [par_cons]
    rw [h b (List.mem_cons_self ..), ih fun c hc => h c (List.mem_cons_of_mem _ hc)]; rfl

theorem par_map_xor {α : Type} (l : List α) (f g : α → Bool) :
    par (l.map fun a => f a ^^ g a) = (par (l.map f) ^^ par (l.map g)) := by
  induction l with
  | nil => simp
  | cons a l ih =>
    simp only [List.map_cons, par_cons, ih]
    cases f a <;> cases g a <;> cases par (l.map f) <;> cases par (l.map g) <;> rfl

theorem par_map_congr {α : Type} (l : List α) (f g : α → Bool) (h : ∀ a ∈ l, f a = g a) :
    par (l.map f) = par (l.map g) := by
  rw [List.map_congr_left h]

theorem par_filter {α : Type} (l : List α) (p f : α → Bool) :
    par ((l.filter p).map f) = par (l.map fun a => p a && f a) := by
  induction l with
  | nil => simp
  | cons a l ih =>
    simp only [List.filter_cons]
    cases hp : p a <;> simp [ih, hp]

theorem par_map_append {α : Type} (l₁ l₂ : List α) (f : α → Bool) :
    par ((l₁ ++ l₂).map f) = (par (l₁.map f) ^^ par (l₂.map f)) := by
  rw [List.map_append, par_append]

theorem par_flatMap {α β : Type} (l : List α) (g : α → List β) (f : β → Bool) :
    par ((l.flatMap g).map f) = par (l.map fun a => par ((g a).map f)) := by
  induction l with
  | nil => simp
  | cons a l ih => simp only [List.flatMap_cons, List.map_append, par_append, ih, List.map_cons, par_cons]

end ForgeCertify
