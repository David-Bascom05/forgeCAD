import ForgeCertify.Sign

/-!
# Interval parity: the engine of every crossing argument

Affine functions `g s = c + m s` on an interval `[s₀, s₁]`. The set where all of them are positive
is an interval; it holds `s₀` or `s₁` or both or neither, and each of its ends inside `(s₀, s₁)`
is a root of one of them at which every other is positive. So:

`ip` : `[all positive at s₀] ⊕ [all positive at s₁]` is the parity of the functions with a root
inside `(s₀, s₁)` at which no other function is `≤ 0` (an *event*), provided none vanishes at an
end and no two vanish at one inner point.

Each crossing lemma of the certificate is this, with the functions read off a triangle (its edge
sides and its plane) along a segment or a ray, and its events read as crossings of edges.
-/

namespace ForgeCertify

/-- An affine function of one rational. -/
structure Aff where
  c : Rat
  m : Rat
  deriving DecidableEq, Inhabited

namespace Aff

def ev (g : Aff) (s : Rat) : Rat := g.c + g.m * s

/-- Its root (meaningful when `m ≠ 0`). -/
def root (g : Aff) : Rat := -g.c / g.m

theorem ev_eq (g : Aff) (h : g.m ≠ 0) (s : Rat) : g.ev s = g.m * (s - g.root) := by
  unfold ev root; grind

theorem ev_root (g : Aff) (h : g.m ≠ 0) : g.ev g.root = 0 := by
  rw [ev_eq g h]; grind

/-- An affine function is positive between two points where it is positive. -/
theorem pos_between (g : Aff) {s₀ s₁ s : Rat} (h0 : 0 < g.ev s₀) (h1 : 0 < g.ev s₁)
    (hs0 : s₀ ≤ s) (hs1 : s ≤ s₁) : 0 < g.ev s := by
  unfold ev at *
  rcases lt_trichotomy' g.m 0 with hm | hm | hm
  · have := Rat.mul_le_mul_of_nonneg_left (a := s) (b := s₁) hs1 (c := -g.m) (by grind)
    grind
  · have e : g.m * s₀ = 0 := by rw [hm]; grind
    have e' : g.m * s = 0 := by rw [hm]; grind
    grind
  · have := Rat.mul_le_mul_of_nonneg_left (a := s₀) (b := s) hs0 (c := g.m) (by grind)
    grind

theorem neg_between (g : Aff) {s₀ s₁ s : Rat} (h0 : g.ev s₀ < 0) (h1 : g.ev s₁ < 0)
    (hs0 : s₀ ≤ s) (hs1 : s ≤ s₁) : g.ev s < 0 := by
  have := pos_between ⟨-g.c, -g.m⟩ (s := s) (s₀ := s₀) (s₁ := s₁) (by unfold ev at *; grind)
    (by unfold ev at *; grind) hs0 hs1
  unfold ev at *; grind

/-- A sign change from `+` at `s₀` to `-` at `s₁ > s₀`: a root between, and the sign on each
side of it. -/
theorem down (g : Aff) {s₀ s₁ : Rat} (h01 : s₀ < s₁) (h0 : 0 < g.ev s₀) (h1 : g.ev s₁ < 0) :
    g.m < 0 ∧ s₀ < g.root ∧ g.root < s₁ ∧ ∀ s, (0 < g.ev s ↔ s < g.root) := by
  have hm : g.m < 0 := by
    apply Classical.byContradiction; intro hm
    have := Rat.mul_le_mul_of_nonneg_left (a := s₀) (b := s₁) (Rat.le_of_lt h01) (c := g.m) (by grind)
    unfold ev at h0 h1; grind
  have hm0 : g.m ≠ 0 := by grind
  have e := ev_eq g hm0
  refine ⟨hm, ?_, ?_, fun s => ?_⟩
  · rw [e] at h0; have := (mul_pos_iff' g.m (s₀ - g.root)).mp h0; grind
  · rw [e] at h1; have := (mul_neg_iff' g.m (s₁ - g.root)).mp h1; grind
  · rw [e, mul_pos_iff']; grind

theorem up (g : Aff) {s₀ s₁ : Rat} (h01 : s₀ < s₁) (h0 : g.ev s₀ < 0) (h1 : 0 < g.ev s₁) :
    0 < g.m ∧ s₀ < g.root ∧ g.root < s₁ ∧ ∀ s, (0 < g.ev s ↔ g.root < s) := by
  have hm : 0 < g.m := by
    apply Classical.byContradiction; intro hm
    have := Rat.mul_le_mul_of_nonneg_left (a := s₀) (b := s₁) (Rat.le_of_lt h01) (c := -g.m) (by grind)
    unfold ev at h0 h1; grind
  have hm0 : g.m ≠ 0 := by grind
  have e := ev_eq g hm0
  refine ⟨hm, ?_, ?_, fun s => ?_⟩
  · rw [e] at h0; have := (mul_neg_iff' g.m (s₀ - g.root)).mp h0; grind
  · rw [e] at h1; have := (mul_pos_iff' g.m (s₁ - g.root)).mp h1; grind
  · rw [e, mul_pos_iff']; grind

end Aff

/-- All positive at `s`. -/
def pos (gs : List Aff) (s : Rat) : Bool := gs.all fun g => decide (0 < g.ev s)

/-- How many are `≤ 0` at `s`. -/
def nonpos (gs : List Aff) (s : Rat) : Nat := gs.countP fun g => decide (g.ev s ≤ 0)

/-- `g`'s root is an event of `gs` in `(s₀, s₁)`: inside, and every other function positive
there (`g` itself is the one function of `gs` not positive). -/
def isEv (s₀ s₁ : Rat) (gs : List Aff) (g : Aff) : Bool :=
  decide (g.m ≠ 0) && decide (s₀ < g.root) && decide (g.root < s₁) && nonpos gs g.root == 1

/-- No two of them vanish at one point of `(s₀, s₁)`. -/
def Distinct (gs : List Aff) (s₀ s₁ : Rat) : Prop :=
  ∀ s, s₀ < s → s < s₁ → gs.countP (fun g => decide (g.ev s = 0)) ≤ 1

theorem nonpos_zero_iff (gs : List Aff) (s : Rat) : nonpos gs s = 0 ↔ pos gs s = true := by
  unfold nonpos pos
  rw [List.countP_eq_zero, List.all_eq_true]
  constructor
  · intro h g hg; have := h g hg; simp at this ⊢; grind
  · intro h g hg; have := h g hg; simp at this ⊢; grind

theorem isEv_cons_self (s₀ s₁ : Rat) (g : Aff) (gs : List Aff) :
    isEv s₀ s₁ (g :: gs) g =
      (decide (g.m ≠ 0) && decide (s₀ < g.root) && decide (g.root < s₁) && pos gs g.root) := by
  unfold isEv
  by_cases hm : g.m = 0
  · simp [hm]
  · have h0 := Aff.ev_root g hm
    have : nonpos (g :: gs) g.root = nonpos gs g.root + 1 := by
      unfold nonpos; rw [List.countP_cons]; simp [h0]
    rw [this]
    have hz := nonpos_zero_iff gs g.root
    cases hp : pos gs g.root
    · have : nonpos gs g.root ≠ 0 := fun h => by rw [hz.mp h] at hp; cases hp
      simp; grind
    · have := hz.mpr hp; simp [this]

theorem isEv_cons_other (s₀ s₁ : Rat) (g h : Aff) (gs : List Aff) (hh : h ∈ gs) :
    isEv s₀ s₁ (g :: gs) h = (isEv s₀ s₁ gs h && decide (0 < g.ev h.root)) := by
  unfold isEv
  by_cases hm : h.m = 0
  · simp [hm]
  · have h0 := Aff.ev_root h hm
    have hpos : 0 < nonpos gs h.root := by
      unfold nonpos; apply List.countP_pos_iff.mpr; exact ⟨h, hh, by simp [h0]⟩
    by_cases hg : 0 < g.ev h.root
    · have : nonpos (g :: gs) h.root = nonpos gs h.root := by
        unfold nonpos; rw [List.countP_cons]; simp; grind
      rw [this]; simp [hg]
    · have : nonpos (g :: gs) h.root = nonpos gs h.root + 1 := by
        unfold nonpos; rw [List.countP_cons]; simp; grind
      rw [this]; simp [hg]; grind

theorem distinct_tail {g : Aff} {gs : List Aff} {s₀ s₁ : Rat} (h : Distinct (g :: gs) s₀ s₁) :
    Distinct gs s₀ s₁ := fun s h0 h1 => by
  have := h s h0 h1; rw [List.countP_cons] at this; grind

theorem distinct_sub {gs : List Aff} {s₀ s₁ t₀ t₁ : Rat} (h : Distinct gs s₀ s₁) (h0 : s₀ ≤ t₀)
    (h1 : t₁ ≤ s₁) : Distinct gs t₀ t₁ := fun s a b => h s (by grind) (by grind)

/-- At a root of `g` inside the interval, no other function vanishes. -/
theorem distinct_root {g : Aff} {gs : List Aff} {s₀ s₁ : Rat} (h : Distinct (g :: gs) s₀ s₁)
    (hm : g.m ≠ 0) (h0 : s₀ < g.root) (h1 : g.root < s₁) : ∀ k ∈ gs, k.ev g.root ≠ 0 := by
  intro k hk hz
  have := h g.root h0 h1
  rw [List.countP_cons] at this
  have hpos : 0 < gs.countP (fun g' => decide (g'.ev g.root = 0)) :=
    List.countP_pos_iff.mpr ⟨k, hk, by simp [hz]⟩
  simp [Aff.ev_root g hm] at this; grind

/-- **Interval parity.** -/
theorem ip : ∀ (gs : List Aff) (s₀ s₁ : Rat), s₀ < s₁ →
    (∀ g ∈ gs, g.ev s₀ ≠ 0 ∧ g.ev s₁ ≠ 0) → Distinct gs s₀ s₁ →
    (pos gs s₀ ^^ pos gs s₁) = par (gs.map (isEv s₀ s₁ gs))
  | [], _, _, _, _, _ => by simp [pos]
  | g :: gs, s₀, s₁, h01, hends, hd => by
    have hg := hends g (List.mem_cons_self ..)
    have hends' : ∀ k ∈ gs, k.ev s₀ ≠ 0 ∧ k.ev s₁ ≠ 0 := fun k hk => hends k (List.mem_cons_of_mem _ hk)
    have hd' := distinct_tail hd
    simp only [List.map_cons, par_cons, isEv_cons_self]
    rw [List.map_congr_left (fun h hh => isEv_cons_other s₀ s₁ g h gs hh)]
    have hposc : ∀ s, pos (g :: gs) s = (decide (0 < g.ev s) && pos gs s) := fun s => by
      simp [pos]
    rw [hposc, hposc]
    rcases lt_trichotomy' 0 (g.ev s₀) with a0 | a0 | a0 <;>
      rcases lt_trichotomy' 0 (g.ev s₁) with a1 | a1 | a1
    · -- positive throughout: no root inside, and every event keeps its sign
      have hpos : ∀ s, s₀ ≤ s → s ≤ s₁ → 0 < g.ev s := fun s h0 h1 => g.pos_between a0 a1 h0 h1
      have hno : (decide (g.m ≠ 0) && decide (s₀ < g.root) && decide (g.root < s₁) && pos gs g.root) = false := by
        by_cases hm : g.m = 0
        · simp [hm]
        · have := hpos g.root
          have := Aff.ev_root g hm
          simp; intro _ h0 h1; have := hpos g.root (by grind) (by grind); grind
      rw [hno]
      rw [List.map_congr_left (g := isEv s₀ s₁ gs) (fun h hh => by
        unfold isEv
        cases hev : (decide (h.m ≠ 0) && decide (s₀ < h.root) && decide (h.root < s₁) && nonpos gs h.root == 1)
        · simp
        · simp at hev; simp; exact hpos h.root (by grind) (by grind))]
      simp [a0, a1]
      exact ip gs s₀ s₁ h01 hends' hd'
    · exact absurd a1.symm hg.2
    · -- + to -: the root r, and everything to its left
      obtain ⟨hm, hr0, hr1, hsg⟩ := g.down h01 a0 a1
      have hm0 : g.m ≠ 0 := by grind
      have hnz := distinct_root hd hm0 hr0 hr1
      have ih := ip gs s₀ g.root hr0 (fun k hk => ⟨(hends' k hk).1, hnz k hk⟩)
        (distinct_sub hd' (Rat.le_refl) (Rat.le_of_lt hr1))
      rw [List.map_congr_left (g := isEv s₀ g.root gs) (fun h hh => by
        unfold isEv; rw [show decide (0 < g.ev h.root) = decide (h.root < g.root) from by simp [hsg]]
        cases decide (h.m ≠ 0) <;> cases hd1 : decide (s₀ < h.root) <;> simp at hd1 ⊢ <;> grind)]
      rw [← ih]
      simp [a0, show ¬ 0 < g.ev s₁ by grind, hm0, hr0, hr1]
      cases pos gs s₀ <;> cases pos gs g.root <;> rfl
    · exact absurd a0.symm hg.1
    · exact absurd a0.symm hg.1
    · exact absurd a0.symm hg.1
    · -- - to +: the root r, and everything to its right
      obtain ⟨hm, hr0, hr1, hsg⟩ := g.up h01 a0 a1
      have hm0 : g.m ≠ 0 := by grind
      have hnz := distinct_root hd hm0 hr0 hr1
      have ih := ip gs g.root s₁ hr1 (fun k hk => ⟨hnz k hk, (hends' k hk).2⟩)
        (distinct_sub hd' (Rat.le_of_lt hr0) (Rat.le_refl))
      rw [List.map_congr_left (g := isEv g.root s₁ gs) (fun h hh => by
        unfold isEv; rw [show decide (0 < g.ev h.root) = decide (g.root < h.root) from by simp [hsg]]
        cases decide (h.m ≠ 0) <;> cases hd1 : decide (h.root < s₁) <;> simp at hd1 ⊢ <;> grind)]
      rw [← ih]
      simp [a1, show ¬ 0 < g.ev s₀ by grind, hm0, hr0, hr1]
    · exact absurd a1.symm hg.2
    · -- negative throughout: nothing is positive, nothing is an event
      have hneg : ∀ s, s₀ ≤ s → s ≤ s₁ → g.ev s < 0 := fun s h0 h1 => g.neg_between a0 a1 h0 h1
      have hno : (decide (g.m ≠ 0) && decide (s₀ < g.root) && decide (g.root < s₁) && pos gs g.root) = false := by
        by_cases hm : g.m = 0
        · simp [hm]
        · simp; intro _ h0 h1; have := hneg g.root (by grind) (by grind)
          have := Aff.ev_root g hm; grind
      rw [hno]
      rw [par_all_false]
      · simp [show ¬ 0 < g.ev s₀ by grind, show ¬ 0 < g.ev s₁ by grind]
      · intro b hb
        obtain ⟨h, hh, rfl⟩ := List.mem_map.mp hb
        unfold isEv
        cases hev : (decide (h.m ≠ 0) && decide (s₀ < h.root) && decide (h.root < s₁) && nonpos gs h.root == 1)
        · simp
        · simp at hev; simp; intro _; have := hneg h.root (by grind) (by grind); grind

end ForgeCertify
