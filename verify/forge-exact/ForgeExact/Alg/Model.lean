import ForgeExact.Alg.Poly

/-!
# A model of the reals, and the sign certificates the algebraic numbers rest on

* `Model K`: an ordered field `K` with ℚ embedded (`ι`, an order embedding) and the intermediate
  value property for polynomials with rational coefficients. ℝ with `Rat.cast` is a model; so is
  every real closed field. Every theorem about algebraic numbers is stated for every model.
* `noPosRoot p`: every nonzero coefficient has one sign. Then `p` has no positive root
  (`noPosRoot_spec`).
* `onePosRoot p`: the coefficients change sign once (a split index `k`: those below are ≥ 0 with
  one > 0, those from `k` up are ≤ 0, or all of it negated). Then `p` has at most one positive
  root (`onePosRoot_spec`): `x^k p(y) < y^k p(x)` for `0 < x < y`. These are the two cases of
  Descartes' rule of signs that root isolation needs (Vincent–Collins–Akritas), proven by
  comparing terms, without derivatives.
* `moebius p a b = (1 + x)^n p((a + b x)/(1 + x))`: its positive roots are `p`'s roots in
  `(a, b)` (`moebius_root`), so the two certificates count roots in an interval.
-/

namespace ForgeExact.Alg

open Lean.Grind

set_option linter.unusedSectionVars false

section Model

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K]

/-- A model of the real numbers for these theorems. -/
structure Model (K : Type) [Field K] [LE K] [LT K] where
  /-- ℚ in `K` -/
  ι : Hom Rat K
  /-- an order embedding -/
  lt : ∀ a b : Rat, a < b → ι.f a < ι.f b
  /-- the intermediate value property, for polynomials with rational coefficients -/
  ivt : ∀ (p : Poly) (a b : K), a ≤ b → evalWith ι p a ≤ 0 → 0 ≤ evalWith ι p b →
    ∃ x, a ≤ x ∧ x ≤ b ∧ evalWith ι p x = 0

variable (M : Model K)

/-- The value of a rational polynomial in the model. -/
abbrev Model.ev (p : Poly) (x : K) : K := evalWith M.ι p x

/-- A rational, in the model. -/
abbrev Model.q (a : Rat) : K := M.ι.f a

theorem Model.q_lt {a b : Rat} : M.q a < M.q b ↔ a < b := by
  constructor
  · intro h
    apply Classical.byContradiction
    intro hn
    rcases Rat.le_iff_lt_or_eq.mp (Rat.not_lt.mp hn) with h' | h'
    · have := M.lt _ _ h'; grind
    · subst h'; grind
  · exact M.lt a b

theorem Model.q_le {a b : Rat} : M.q a ≤ M.q b ↔ a ≤ b := by
  constructor
  · intro h; apply Classical.byContradiction; intro hn
    have := (M.q_lt).mpr (Rat.not_le.mp hn); grind
  · intro h
    rcases Rat.le_iff_lt_or_eq.mp h with h' | h'
    · have := M.lt _ _ h'; grind
    · subst h'; grind

theorem Model.q_eq {a b : Rat} : M.q a = M.q b ↔ a = b := by
  constructor
  · intro h
    have h1 := (M.q_le).mp (show M.q a ≤ M.q b by grind)
    have h2 := (M.q_le).mp (show M.q b ≤ M.q a by grind)
    exact Rat.le_antisymm h1 h2
  · intro h; rw [h]

@[simp] theorem Model.q_zero : M.q 0 = 0 := M.ι.map_zero
@[simp] theorem Model.q_one : M.q 1 = 1 := M.ι.map_one
theorem Model.q_add (a b : Rat) : M.q (a + b) = M.q a + M.q b := M.ι.map_add a b
theorem Model.q_mul (a b : Rat) : M.q (a * b) = M.q a * M.q b := M.ι.map_mul a b
theorem Model.q_neg (a : Rat) : M.q (-a) = - M.q a := M.ι.map_neg a
theorem Model.q_sub (a b : Rat) : M.q (a - b) = M.q a - M.q b := by
  rw [Rat.sub_eq_add_neg, M.q_add, M.q_neg]; grind
theorem Model.q_pos {a : Rat} : 0 < M.q a ↔ 0 < a := by
  have := @Model.q_lt _ _ _ _ _ _ _ M 0 a; rw [M.q_zero] at this; exact this
theorem Model.q_neg_iff {a : Rat} : M.q a < 0 ↔ a < 0 := by
  have := @Model.q_lt _ _ _ _ _ _ _ M a 0; rw [M.q_zero] at this; exact this
theorem Model.q_nonneg {a : Rat} : 0 ≤ M.q a ↔ 0 ≤ a := by
  have := @Model.q_le _ _ _ _ _ _ _ M 0 a; rw [M.q_zero] at this; exact this
theorem Model.q_nonpos {a : Rat} : M.q a ≤ 0 ↔ a ≤ 0 := by
  have := @Model.q_le _ _ _ _ _ _ _ M a 0; rw [M.q_zero] at this; exact this

theorem Model.q_inv (a : Rat) (ha : a ≠ 0) : M.q (1 / a) = 1 / M.q a := by
  have h1 : M.q (1 / a) * M.q a = 1 := by
    rw [← M.q_mul, Rat.div_mul_cancel ha]; simp
  have h2 : M.q a ≠ 0 := fun h => ha ((M.q_eq).mp (by rw [h]; simp))
  grind

theorem Model.q_div (a b : Rat) (hb : b ≠ 0) : M.q (a / b) = M.q a / M.q b := by
  rw [Rat.div_def, M.q_mul, show b⁻¹ = 1 / b by rw [Rat.div_def, Rat.one_mul], M.q_inv b hb]; grind

/-- Evaluation at a rational point is the executable's `ev`. -/
theorem Model.ev_q (x : Rat) : ∀ p : Poly, M.ev p (M.q x) = M.q (Poly.ev p x)
  | [] => by simp only [Model.ev, evalWith_nil, Poly.ev, List.foldr_nil]; exact M.q_zero.symm
  | c :: p => by
    have := Model.ev_q x p
    simp only [Model.ev, evalWith_cons, Poly.ev, List.foldr_cons] at this ⊢
    rw [this, M.q_add, M.q_mul]

/-- The IVT for a sign change either way, endpoints excluded. -/
theorem Model.root_between (p : Poly) {a b : K} (hab : a < b)
    (hs : M.ev p a * M.ev p b < 0) : ∃ x, a < x ∧ x < b ∧ M.ev p x = 0 := by
  have ha0 : M.ev p a ≠ 0 := fun h => by rw [h] at hs; grind
  have hb0 : M.ev p b ≠ 0 := fun h => by rw [h] at hs; grind
  have ends : ∀ x, a ≤ x → x ≤ b → M.ev p x = 0 → a < x ∧ x < b := by
    intro x h1 h2 h3
    have e1 : x ≠ a := fun e => ha0 (e ▸ h3)
    have e2 : x ≠ b := fun e => hb0 (e ▸ h3)
    constructor <;> grind
  rcases LinearOrder.trichotomy (M.ev p a) 0 with ha | ha | ha
  · have hb : 0 < M.ev p b := by
      apply Classical.byContradiction; intro hn
      have := OrderedRing.mul_nonneg_of_nonpos_of_nonpos (a := M.ev p a) (b := M.ev p b) (by grind) (by grind)
      grind
    obtain ⟨x, h1, h2, h3⟩ := M.ivt p a b (by grind) (by grind) (by grind)
    exact ⟨x, (ends x h1 h2 h3).1, (ends x h1 h2 h3).2, h3⟩
  · exact absurd ha ha0
  · have hb : M.ev p b < 0 := by
      apply Classical.byContradiction; intro hn
      have := OrderedRing.mul_nonneg (a := M.ev p a) (b := M.ev p b) (by grind) (by grind)
      grind
    obtain ⟨x, h1, h2, h3⟩ := M.ivt (pneg p) a b (by grind)
      (by simp only [eval_pneg]; grind) (by simp only [eval_pneg]; grind)
    rw [eval_pneg] at h3
    have h3' : M.ev p x = 0 := by grind
    exact ⟨x, (ends x h1 h2 h3').1, (ends x h1 h2 h3').2, h3'⟩

end Model

/-! ## Powers in an ordered field -/

section Pow

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K]

theorem pow_pos' {x : K} (hx : 0 < x) : ∀ k : Nat, 0 < x ^ k
  | 0 => by rw [Semiring.pow_zero]; exact OrderedRing.zero_lt_one
  | k + 1 => by rw [Semiring.pow_succ]; exact OrderedRing.mul_pos (pow_pos' hx k) hx

theorem pow_le_pow' {x y : K} (hx : 0 < x) (hxy : x ≤ y) : ∀ k : Nat, x ^ k ≤ y ^ k
  | 0 => by rw [Semiring.pow_zero, Semiring.pow_zero]; grind
  | k + 1 => by
    rw [Semiring.pow_succ, Semiring.pow_succ]
    have h1 := pow_le_pow' hx hxy k
    have h2 := pow_pos' hx k
    have h3 : x ^ k * x ≤ x ^ k * y := OrderedRing.mul_le_mul_of_nonneg_left hxy (by grind)
    have h4 : x ^ k * y ≤ y ^ k * y := OrderedRing.mul_le_mul_of_nonneg_right h1 (by grind)
    grind

theorem pow_lt_pow' {x y : K} (hx : 0 < x) (hxy : x < y) : ∀ k : Nat, x ^ (k + 1) < y ^ (k + 1)
  | k => by
    rw [Semiring.pow_succ, Semiring.pow_succ]
    have h1 := pow_le_pow' (y := y) hx (by grind) k
    have h2 := pow_pos' hx k
    have h3 : x ^ k * x < x ^ k * y := OrderedRing.mul_lt_mul_of_pos_left hxy h2
    have h4 : x ^ k * y ≤ y ^ k * y := OrderedRing.mul_le_mul_of_nonneg_right h1 (by grind)
    grind

end Pow

/-! ## Descartes' rule, the two cases -/

/-- Every coefficient is ≥ 0. -/
def allNonneg (p : Poly) : Bool := p.all (0 ≤ ·)

/-- No positive root, certified: the nonzero coefficients have one sign. -/
def noPosRoot (p : Poly) : Bool :=
  (p.all (0 ≤ ·) && p.any (0 < ·)) || (p.all (· ≤ 0) && p.any (· < 0))

/-- The split pattern: the first `k` coefficients ≥ 0, the rest ≤ 0. -/
def patOK : Poly → Nat → Bool
  | [], _ => true
  | c :: p, 0 => decide (c ≤ 0) && patOK p 0
  | c :: p, k + 1 => decide (0 ≤ c) && patOK p k

/-- One of the first `k` coefficients is > 0. -/
def patStrict : Poly → Nat → Bool
  | [], _ => false
  | _ :: _, 0 => false
  | c :: p, k + 1 => decide (0 < c) || patStrict p k

/-- A split index with the pattern, if there is one. -/
def onePattern (p : Poly) : Bool :=
  (List.range (p.length + 1)).any fun k => patOK p k && patStrict p k

/-- At most one positive root, certified: one sign change (for `p` or `-p`). -/
def onePosRoot (p : Poly) : Bool := onePattern p || onePattern (pneg p)

section Descartes

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K] (M : Model K)

theorem ev_pos_of_coeffs : ∀ (p : Poly) {x : K}, 0 < x → (∀ c ∈ p, 0 ≤ c) →
    (∃ c ∈ p, 0 < c) → 0 < M.ev p x
  | [], _, _, _, ⟨c, hc, _⟩ => nomatch hc
  | c :: p, x, hx, h0, ⟨d, hd, hd0⟩ => by
    simp only [Model.ev, evalWith_cons]
    have hc : 0 ≤ M.q c := M.q_nonneg.mpr (h0 c (List.mem_cons_self ..))
    have hp0 : 0 ≤ M.ev p x := by
      by_cases hpos : ∃ c ∈ p, 0 < c
      · exact Preorder.le_of_lt (ev_pos_of_coeffs p hx (fun c hc => h0 c (List.mem_cons_of_mem _ hc)) hpos)
      · rw [Model.ev, eval_of_all_zero]
        · grind
        · intro c' hc'
          have h1 := h0 c' (List.mem_cons_of_mem _ hc')
          have h2 : ¬ 0 < c' := fun h => hpos ⟨c', hc', h⟩
          have : c' = 0 := Rat.le_antisymm (Rat.not_lt.mp h2) h1
          subst this; exact M.ι.map_zero
    rcases List.mem_cons.mp hd with rfl | hd
    · have := M.q_pos.mpr hd0
      have := OrderedRing.mul_nonneg (Preorder.le_of_lt hx) hp0
      grind
    · have hp := ev_pos_of_coeffs p hx (fun c hc => h0 c (List.mem_cons_of_mem _ hc)) ⟨d, hd, hd0⟩
      have := OrderedRing.mul_pos hx hp
      grind

theorem ev_neg_of_coeffs (p : Poly) {x : K} (hx : 0 < x) (h0 : ∀ c ∈ p, c ≤ 0)
    (h1 : ∃ c ∈ p, c < 0) : M.ev p x < 0 := by
  have := ev_pos_of_coeffs M (pneg p) hx
    (fun c hc => by
      obtain ⟨d, hd, rfl⟩ := List.mem_map.mp hc
      have := h0 d hd; grind)
    (by obtain ⟨d, hd, hd0⟩ := h1; exact ⟨-d, List.mem_map.mpr ⟨d, hd, rfl⟩, by grind⟩)
  simp only [Model.ev, pneg] at this
  rw [show List.map (fun x => -x) p = pneg p from rfl, eval_pneg] at this
  simp only [Model.ev]; grind

/-- **No positive root.** -/
theorem noPosRoot_spec (p : Poly) (h : noPosRoot p = true) {x : K} (hx : 0 < x) :
    M.ev p x ≠ 0 := by
  simp only [noPosRoot, Bool.or_eq_true, Bool.and_eq_true, List.all_eq_true, List.any_eq_true,
    decide_eq_true_eq] at h
  rcases h with ⟨h0, h1⟩ | ⟨h0, h1⟩
  · have := ev_pos_of_coeffs M p hx h0 h1; grind
  · have := ev_neg_of_coeffs M p hx h0 h1; grind

theorem ev_nonpos_of_patOK0 : ∀ (p : Poly) {x : K}, 0 < x → patOK p 0 = true → M.ev p x ≤ 0
  | [], _, _, _ => by simp [Model.ev] <;> grind
  | c :: p, x, hx, h => by
    simp only [patOK, Bool.and_eq_true, decide_eq_true_eq] at h
    have := ev_nonpos_of_patOK0 p hx h.2
    have hc := M.q_nonpos.mpr h.1
    have := OrderedRing.mul_nonpos_of_nonneg_of_nonpos (Preorder.le_of_lt hx) this
    simp only [Model.ev, evalWith_cons] at *; grind

/-- The weak comparison: `x^k p(y) ≤ y^k p(x)` for `0 < x ≤ y`. -/
theorem patOK_le : ∀ (p : Poly) (k : Nat) {x y : K}, 0 < x → x ≤ y → patOK p k = true →
    x ^ k * M.ev p y ≤ y ^ k * M.ev p x
  | [], _, _, _, _, _, _ => by simp [Model.ev]; grind
  | c :: p, 0, x, y, hx, hxy, h => by
    simp only [patOK, Bool.and_eq_true, decide_eq_true_eq] at h
    have ih := patOK_le p 0 hx hxy h.2
    have hpy := ev_nonpos_of_patOK0 M p (Preorder.lt_of_lt_of_le hx hxy) h.2
    simp only [Model.ev, evalWith_cons, Semiring.pow_zero] at *
    have h1 : y * evalWith M.ι p y ≤ x * evalWith M.ι p y :=
      OrderedRing.mul_le_mul_of_nonpos_right hxy hpy
    have h2 : x * evalWith M.ι p y ≤ x * evalWith M.ι p x :=
      OrderedRing.mul_le_mul_of_nonneg_left (by grind) (Preorder.le_of_lt hx)
    grind
  | c :: p, k + 1, x, y, hx, hxy, h => by
    simp only [patOK, Bool.and_eq_true, decide_eq_true_eq] at h
    have ih := patOK_le p k hx hxy h.2
    have hc := M.q_nonneg.mpr h.1
    have hk := pow_le_pow' hx hxy (k + 1)
    simp only [Model.ev, evalWith_cons] at *
    have h1 : x ^ (k + 1) * M.q c ≤ y ^ (k + 1) * M.q c :=
      OrderedRing.mul_le_mul_of_nonneg_right hk hc
    have h2 : (x * y) * (x ^ k * evalWith M.ι p y) ≤ (x * y) * (y ^ k * evalWith M.ι p x) :=
      OrderedRing.mul_le_mul_of_nonneg_left ih
        (OrderedRing.mul_nonneg (Preorder.le_of_lt hx) (by grind))
    rw [Semiring.pow_succ, Semiring.pow_succ] at *
    grind

/-- The strict comparison: `x^k p(y) < y^k p(x)` for `0 < x < y`. -/
theorem patStrict_lt : ∀ (p : Poly) (k : Nat) {x y : K}, 0 < x → x < y → patOK p k = true →
    patStrict p k = true → x ^ k * M.ev p y < y ^ k * M.ev p x
  | [], _, _, _, _, _, _, h => by simp [patStrict] at h
  | _ :: _, 0, _, _, _, _, _, h => by simp [patStrict] at h
  | c :: p, k + 1, x, y, hx, hxy, h, hs => by
    simp only [patOK, Bool.and_eq_true, decide_eq_true_eq] at h
    simp only [patStrict, Bool.or_eq_true, decide_eq_true_eq] at hs
    have hk := pow_lt_pow' hx hxy k
    have hxyp : 0 < x * y := OrderedRing.mul_pos hx (Preorder.lt_trans hx hxy)
    have hc := M.q_nonneg.mpr h.1
    rcases hs with hs | hs
    · have ih := patOK_le M p k hx (Preorder.le_of_lt hxy) h.2
      have hc' := M.q_pos.mpr hs
      simp only [Model.ev, evalWith_cons] at *
      have h1 : x ^ (k + 1) * M.q c < y ^ (k + 1) * M.q c :=
        OrderedRing.mul_lt_mul_of_pos_right hk hc'
      have h2 : (x * y) * (x ^ k * evalWith M.ι p y) ≤ (x * y) * (y ^ k * evalWith M.ι p x) :=
        OrderedRing.mul_le_mul_of_nonneg_left ih (Preorder.le_of_lt hxyp)
      rw [Semiring.pow_succ, Semiring.pow_succ] at *
      grind
    · have ih := patStrict_lt p k hx hxy h.2 hs
      simp only [Model.ev, evalWith_cons] at *
      have h1 : x ^ (k + 1) * M.q c ≤ y ^ (k + 1) * M.q c :=
        OrderedRing.mul_le_mul_of_nonneg_right (Preorder.le_of_lt hk) hc
      have h2 : (x * y) * (x ^ k * evalWith M.ι p y) < (x * y) * (y ^ k * evalWith M.ι p x) :=
        OrderedRing.mul_lt_mul_of_pos_left ih hxyp
      rw [Semiring.pow_succ, Semiring.pow_succ] at *
      grind

theorem onePattern_unique (p : Poly) (h : onePattern p = true) {x y : K} (hx : 0 < x)
    (hy : 0 < y) (hpx : M.ev p x = 0) (hpy : M.ev p y = 0) : x = y := by
  simp only [onePattern, List.any_eq_true, Bool.and_eq_true] at h
  obtain ⟨k, _, hk1, hk2⟩ := h
  rcases LinearOrder.trichotomy x y with hxy | hxy | hxy
  · have := patStrict_lt M p k hx hxy hk1 hk2; rw [hpx, hpy] at this; grind
  · exact hxy
  · have := patStrict_lt M p k hy hxy hk1 hk2; rw [hpx, hpy] at this; grind

/-- **At most one positive root.** -/
theorem onePosRoot_spec (p : Poly) (h : onePosRoot p = true) {x y : K} (hx : 0 < x)
    (hy : 0 < y) (hpx : M.ev p x = 0) (hpy : M.ev p y = 0) : x = y := by
  simp only [onePosRoot, Bool.or_eq_true] at h
  rcases h with h | h
  · exact onePattern_unique M p h hx hy hpx hpy
  · exact onePattern_unique M (pneg p) h hx hy (by simp only [Model.ev, eval_pneg] at *; grind)
      (by simp only [Model.ev, eval_pneg] at *; grind)

end Descartes

end ForgeExact.Alg
