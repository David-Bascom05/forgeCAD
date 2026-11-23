/-!
# A union–find whose root is the least member: the model

A parent array `p` with every parent at most its child (`Good`). `ufRoot p x` follows parents to
a fixed point. Path halving and the union (the greater root under the lesser) are proven to act
on roots as stated, and after the unions of a list of pairs from the identity, `ufRoot` is the
least member of each class of the equivalence the pairs generate (`build_root`).
-/

namespace ForgeSheetCore.UF

def par (p : List Nat) (x : Nat) : Nat := p.getD x x

/-- Every parent is at most its child, and in range. -/
def Good (p : List Nat) : Prop := ∀ i, i < p.length → par p i ≤ i

def ufRoot (p : List Nat) (x : Nat) : Nat :=
  if h : par p x < x then ufRoot p (par p x) else x
termination_by x

theorem ufRoot_eq (p : List Nat) (x : Nat) :
    ufRoot p x = if par p x < x then ufRoot p (par p x) else x := by
  rw [ufRoot]; split <;> rfl

theorem par_out {p : List Nat} {x : Nat} (h : p.length ≤ x) : par p x = x := by
  unfold par; rw [List.getD_eq_getElem?_getD, List.getElem?_eq_none h]; rfl

theorem par_le {p : List Nat} (hg : Good p) (x : Nat) : par p x ≤ x := by
  by_cases h : x < p.length
  · exact hg x h
  · rw [par_out (by omega)]; exact Nat.le_refl _

theorem root_le {p : List Nat} (hg : Good p) : ∀ x, ufRoot p x ≤ x := by
  intro x
  induction x using Nat.strongRecOn with
  | ind x ih =>
    rw [ufRoot_eq]; split
    · rename_i h; have := ih _ h; omega
    · exact Nat.le_refl _

theorem root_lt {p : List Nat} (hg : Good p) {x : Nat} (hx : x < p.length) : ufRoot p x < p.length :=
  Nat.lt_of_le_of_lt (root_le hg x) hx

/-- A root is its own parent. -/
theorem par_root {p : List Nat} (hg : Good p) : ∀ x, par p (ufRoot p x) = ufRoot p x := by
  intro x
  induction x using Nat.strongRecOn with
  | ind x ih =>
    rw [ufRoot_eq]; split
    · rename_i h; exact ih _ h
    · have := par_le hg x; omega

theorem root_root {p : List Nat} (hg : Good p) (x : Nat) : ufRoot p (ufRoot p x) = ufRoot p x := by
  rw [ufRoot_eq, par_root hg]; simp

theorem root_par {p : List Nat} (hg : Good p) (x : Nat) : ufRoot p (par p x) = ufRoot p x := by
  conv => rhs; rw [ufRoot_eq]
  split
  · rfl
  · rename_i h
    have e : par p x = x := by have := par_le hg x; omega
    rw [e, ufRoot_eq, if_neg h]

/-! ## Setting one parent -/

theorem par_set (p : List Nat) (i v y : Nat) :
    par (p.set i v) y = if y = i ∧ i < p.length then v else par p y := by
  unfold par
  rw [List.getD_eq_getElem?_getD, List.getElem?_set, List.getD_eq_getElem?_getD]
  by_cases h1 : i = y
  · subst h1
    by_cases h2 : i < p.length
    · simp [h2]
    · simp [h2, List.getElem?_eq_none (by omega : p.length ≤ i)]
  · simp [h1, Ne.symm h1]

/-- **Path halving keeps every root.** -/
theorem halve_good {p : List Nat} (hg : Good p) (x : Nat) :
    Good (p.set x (par p (par p x))) := by
  intro i hi
  rw [List.length_set] at hi
  rw [par_set]
  split
  · rename_i h; obtain ⟨rfl, _⟩ := h
    have := par_le hg (par p i); have := par_le hg i; omega
  · exact hg i hi

theorem halve_root {p : List Nat} (hg : Good p) (x : Nat) :
    ∀ y, ufRoot (p.set x (par p (par p x))) y = ufRoot p y := by
  intro y
  induction y using Nat.strongRecOn with
  | ind y ih =>
    have hq := halve_good hg x
    rw [ufRoot_eq (p.set x _), par_set]
    by_cases hy : y = x ∧ x < p.length
    · obtain ⟨rfl, _⟩ := hy
      simp only [and_self, ‹y < p.length›, ite_true]
      split
      · rename_i h
        rw [ih _ h, root_par hg, root_par hg]
      · rename_i h
        have h1 := par_le hg (par p y); have h2 := par_le hg y
        have e : par p y = y := by omega
        rw [ufRoot_eq, e]; simp
    · rw [if_neg hy]
      split
      · rename_i h; rw [ih _ h, root_par hg]
      · rename_i h; rw [ufRoot_eq, if_neg h]

/-- **The union**: root `r` set under the lesser root `r'`. -/
theorem link_good {p : List Nat} (hg : Good p) {r r' : Nat} (hlt : r' < r) : Good (p.set r r') := by
  intro i hi
  rw [List.length_set] at hi
  rw [par_set]
  split
  · rename_i h; obtain ⟨rfl, _⟩ := h; omega
  · exact hg i hi

theorem link_root {p : List Nat} (hg : Good p) {r r' : Nat} (hlt : r' < r) (hr : r < p.length)
    (hrr : par p r = r) (hrr' : par p r' = r') :
    ∀ y, ufRoot (p.set r r') y = if ufRoot p y = r then r' else ufRoot p y := by
  intro y
  induction y using Nat.strongRecOn with
  | ind y ih =>
    by_cases hy : y = r
    · subst hy
      rw [ufRoot_eq (p.set y r'), par_set]
      simp only [hr, and_self, ite_true, hlt]
      rw [ih _ hlt]
      have e1 : ufRoot p r' = r' := by rw [ufRoot_eq, hrr']; simp
      have e2 : ufRoot p y = y := by rw [ufRoot_eq, hrr]; simp
      rw [e1, e2]; simp [Nat.ne_of_lt hlt]
    · have hp : par (p.set r r') y = par p y := by rw [par_set, if_neg (fun h => hy h.1)]
      rw [ufRoot_eq (p.set r r'), hp]
      by_cases h : par p y < y
      · rw [if_pos h, ih _ h, root_par hg]
      · rw [if_neg h]
        have e : ufRoot p y = y := by rw [ufRoot_eq, if_neg h]
        rw [e]; simp [hy]

/-! ## The classes -/

/-- The equivalence a list of pairs generates. -/
inductive Eqv (E : List (Nat × Nat)) : Nat → Nat → Prop
  | refl (a : Nat) : Eqv E a a
  | edge {e : Nat × Nat} : e ∈ E → Eqv E e.1 e.2
  | symm {a b : Nat} : Eqv E a b → Eqv E b a
  | trans {a b c : Nat} : Eqv E a b → Eqv E b c → Eqv E a c

theorem Eqv.mono {E E' : List (Nat × Nat)} (h : ∀ e ∈ E, e ∈ E') {a b : Nat} (hab : Eqv E a b) :
    Eqv E' a b := by
  induction hab with
  | refl a => exact .refl a
  | edge he => exact .edge (h _ he)
  | symm _ ih => exact .symm ih
  | trans _ _ ih1 ih2 => exact .trans ih1 ih2

/-- `R` names each class of `E` on `0 .. n-1` by its least member. -/
def Least (E : List (Nat × Nat)) (n : Nat) (R : Nat → Nat) : Prop :=
  ∀ y, y < n → R y < n ∧ Eqv E y (R y) ∧ ∀ z, Eqv E y z → R y ≤ z

/-- With `Least`, two points are in one class iff they have one name. -/
theorem least_iff {E : List (Nat × Nat)} {n : Nat} {R : Nat → Nat} (hl : Least E n R)
    {y z : Nat} (hy : y < n) (hz : z < n) :
    Eqv E y z ↔ R y = R z := by
  constructor
  · intro h
    obtain ⟨_, hy1, hy2⟩ := hl y hy
    obtain ⟨_, hz1, hz2⟩ := hl z hz
    have a := hy2 (R z) (h.trans hz1)
    have b := hz2 (R y) (h.symm.trans hy1)
    omega
  · intro h
    obtain ⟨_, hy1, _⟩ := hl y hy
    obtain ⟨_, hz1, _⟩ := hl z hz
    rw [h] at hy1
    exact hy1.trans hz1.symm

theorem Eqv.of_sub {E E' : List (Nat × Nat)} (h : ∀ e ∈ E, Eqv E' e.1 e.2) {a b : Nat}
    (hab : Eqv E a b) : Eqv E' a b := by
  induction hab with
  | refl a => exact .refl a
  | edge he => exact h _ he
  | symm _ ih => exact .symm ih
  | trans _ _ ih1 ih2 => exact .trans ih1 ih2

/-- In range: a class of in-range pairs never leaves `0 .. n-1`. -/
theorem Eqv.range {E : List (Nat × Nat)} {n : Nat} (hE : ∀ e ∈ E, e.1 < n ∧ e.2 < n) {y z : Nat}
    (h : Eqv E y z) : y = z ∨ (y < n ∧ z < n) := by
  induction h with
  | refl a => exact Or.inl rfl
  | edge he => exact Or.inr (hE _ he)
  | symm _ ih => rcases ih with h | h; exact Or.inl h.symm; exact Or.inr ⟨h.2, h.1⟩
  | trans _ _ ih1 ih2 =>
    rcases ih1 with h1 | h1 <;> rcases ih2 with h2 | h2
    · exact Or.inl (h1.trans h2)
    · subst h1; exact Or.inr h2
    · subst h2; exact Or.inr h1
    · exact Or.inr ⟨h1.1, h2.2⟩

theorem eqv_nil {y z : Nat} (h : Eqv [] y z) : y = z := by
  induction h with
  | refl => rfl
  | edge he => simp at he
  | symm _ ih => exact ih.symm
  | trans _ _ ih1 ih2 => exact ih1.trans ih2

theorem least_nil (n : Nat) : Least [] n id := by
  intro y hy
  exact ⟨hy, .refl y, fun z hz => by simp [eqv_nil hz]⟩

/-- **One union.** Joining `a`'s class to `b`'s, the greater name under the lesser, names the
classes of the pairs with `(a, b)` added by their least members. -/
theorem least_union {E : List (Nat × Nat)} {n : Nat} {R : Nat → Nat} (hl : Least E n R)
    (hE : ∀ e ∈ E, e.1 < n ∧ e.2 < n) {a b : Nat} (ha : a < n) (hb : b < n) (hlt : R a < R b) :
    Least (E ++ [(a, b)]) n (fun y => if R y = R b then R a else R y) := by
  have hE' : ∀ e ∈ E ++ [(a, b)], e.1 < n ∧ e.2 < n := by
    intro e he
    rcases List.mem_append.mp he with h | h
    · exact hE e h
    · simp at h; subst h; exact ⟨ha, hb⟩
  have sub : ∀ {y z}, Eqv E y z → Eqv (E ++ [(a, b)]) y z :=
    Eqv.mono (fun e he => List.mem_append_left _ he)
  have eab : Eqv (E ++ [(a, b)]) a b := Eqv.edge (e := (a, b)) (by simp)
  -- one name on each new class
  have same : ∀ y z, Eqv (E ++ [(a, b)]) y z → y < n → z < n →
      (if R y = R b then R a else R y) = (if R z = R b then R a else R z) := by
    intro y z h
    induction h with
    | refl => intro _ _; rfl
    | edge he =>
      rename_i e
      intro h1 h2
      rcases List.mem_append.mp he with h | h
      · have := (least_iff hl h1 h2).mp (Eqv.edge h); rw [this]
      · simp at h; subst h
        simp [Nat.ne_of_lt hlt]
    | symm _ ih => intro h1 h2; exact (ih h2 h1).symm
    | trans hab _ ih1 ih2 =>
      intro h1 h3
      rename_i x m w
      have hm : x < n := by
        rcases Eqv.range hE' hab with e | e
        · rw [← e]; exact h1
        · exact e.2
      rw [ih1 h1 hm, ih2 hm h3]
  intro y hy
  simp only []
  obtain ⟨hRy, hy1, _⟩ := hl y hy
  refine ⟨by split; exact (hl a ha).1; exact hRy, ?_, fun z hz => ?_⟩
  · split
    · rename_i e
      have h1 := sub hy1; rw [e] at h1
      exact h1.trans ((sub (hl b hb).2.1).symm.trans (eab.symm.trans (sub (hl a ha).2.1)))
    · exact sub hy1
  · have hzn : z < n := by
      rcases Eqv.range hE' hz with e | e
      · rw [← e]; exact hy
      · exact e.2
    rw [same y z hz hy hzn]
    have hz' := (hl z hzn).2.2 z (.refl z)
    split <;> omega

theorem least_congr {E E' : List (Nat × Nat)} {n : Nat} {R : Nat → Nat}
    (h : ∀ y z, Eqv E y z ↔ Eqv E' y z) (hl : Least E n R) : Least E' n R := by
  intro y hy
  obtain ⟨a, b, c⟩ := hl y hy
  exact ⟨a, (h _ _).mp b, fun z hz => c z ((h _ _).mpr hz)⟩

/-! ## The start, and the union as the Rust does it -/

theorem par_range (n y : Nat) : par (List.range n) y = y := by
  unfold par
  rw [List.getD_eq_getElem?_getD]
  by_cases h : y < n
  · simp [h]
  · rw [List.getElem?_eq_none (by simp; omega)]; rfl

theorem good_range (n : Nat) : Good (List.range n) := fun i _ => by rw [par_range]; exact Nat.le_refl _

theorem root_range (n y : Nat) : ufRoot (List.range n) y = y := by
  rw [ufRoot_eq, par_range]; simp

theorem least_range (n : Nat) : Least [] n (ufRoot (List.range n)) := by
  have : ufRoot (List.range n) = id := funext (root_range n)
  rw [this]; exact least_nil n

/-- The union of `a` and `b`, on the array's roots: the greater root's parent set to the lesser. -/
def link (p : List Nat) (a b : Nat) : List Nat :=
  let ra := ufRoot p a
  let rb := ufRoot p b
  if ra < rb then p.set rb ra else if rb < ra then p.set ra rb else p

/-- The invariant of the flood: a good parent array of length `n` naming `E`'s classes by their
least members. -/
def Inv (E : List (Nat × Nat)) (n : Nat) (p : List Nat) : Prop :=
  Good p ∧ p.length = n ∧ Least E n (ufRoot p)

theorem inv_range (n : Nat) : Inv [] n (List.range n) := ⟨good_range n, by simp, least_range n⟩

theorem root_of_root {p : List Nat} (hg : Good p) (x : Nat) : par p (ufRoot p x) = ufRoot p x :=
  par_root hg x

/-- **The union keeps the invariant**, for one more pair. -/
theorem inv_link {E : List (Nat × Nat)} {n : Nat} {p : List Nat} (hi : Inv E n p)
    (hE : ∀ e ∈ E, e.1 < n ∧ e.2 < n) {a b : Nat} (ha : a < n) (hb : b < n) :
    Inv (E ++ [(a, b)]) n (link p a b) := by
  obtain ⟨hg, hn, hl⟩ := hi
  have hra := (hl a ha).1
  have hrb := (hl b hb).1
  unfold link
  simp only
  by_cases h1 : ufRoot p a < ufRoot p b
  · rw [if_pos h1]
    refine ⟨link_good hg h1, by simp [hn], ?_⟩
    have e := link_root hg h1 (by omega) (par_root hg b) (par_root hg a)
    rw [funext e]
    exact least_union hl hE ha hb h1
  · rw [if_neg h1]
    by_cases h2 : ufRoot p b < ufRoot p a
    · rw [if_pos h2]
      refine ⟨link_good hg h2, by simp [hn], ?_⟩
      have e := link_root hg h2 (by omega) (par_root hg a) (par_root hg b)
      rw [funext e]
      have := least_union hl hE hb ha h2
      apply least_congr _ this
      intro y z
      have eab : ∀ {E : List (Nat × Nat)}, (a, b) ∈ E → Eqv E b a := fun h => (Eqv.edge (e := (a, b)) h).symm
      have eba : ∀ {E : List (Nat × Nat)}, (b, a) ∈ E → Eqv E a b := fun h => (Eqv.edge (e := (b, a)) h).symm
      constructor
      · apply Eqv.of_sub
        intro e he
        rcases List.mem_append.mp he with h | h
        · exact .edge (List.mem_append_left _ h)
        · simp at h; subst h; exact eab (by simp)
      · apply Eqv.of_sub
        intro e he
        rcases List.mem_append.mp he with h | h
        · exact .edge (List.mem_append_left _ h)
        · simp at h; subst h; exact eba (by simp)
    · rw [if_neg h2]
      refine ⟨hg, hn, ?_⟩
      have heq : ufRoot p a = ufRoot p b := by omega
      apply least_congr _ hl
      intro y z
      constructor
      · exact Eqv.mono (fun e he => List.mem_append_left _ he)
      · apply Eqv.of_sub
        intro e he
        rcases List.mem_append.mp he with h | h
        · exact .edge h
        · simp at h; subst h; exact (least_iff hl ha hb).mpr heq

/-- Two arrays with the same roots link to the same roots. -/
theorem link_same {p q : List Nat} (hq : ∀ y, ufRoot q y = ufRoot p y) (a b : Nat) :
    link q a b = (if ufRoot p a < ufRoot p b then q.set (ufRoot p b) (ufRoot p a)
      else if ufRoot p b < ufRoot p a then q.set (ufRoot p a) (ufRoot p b) else q) := by
  unfold link; simp only [hq]

end ForgeSheetCore.UF
